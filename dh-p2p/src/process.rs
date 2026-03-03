use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UdpSocket,
    sync::{mpsc, oneshot},
};

use crate::ptcp::{PTCPBody, PTCPEvent, PTCPPayload, PTCPSession, PTCP};

/**
 * Read data from the channel and write it back to the client
 */
pub async fn process_writer(
    mut writer: tokio::net::tcp::OwnedWriteHalf,
    mut rx: mpsc::Receiver<Vec<u8>>,
) {
    loop {
        let data = match rx.recv().await {
            Some(d) => d,
            None => break,
        };
        if writer.write_all(&data).await.is_err() {
            break;
        }
    }
}

/**
 * Read data from the client and send it to the channel
 */
pub async fn process_reader(
    mut reader: tokio::net::tcp::OwnedReadHalf,
    realm_id: u32,
    dh_tx: mpsc::Sender<PTCPEvent>,
) {
    let mut buf = [0u8; 4096];

    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) | Err(_) => {
                let _ = dh_tx.send(PTCPEvent::Disconnect(realm_id)).await;
                break;
            }
            Ok(n) => n,
        };

        if dh_tx
            .send(PTCPEvent::Data(realm_id, buf[0..n].to_vec()))
            .await
            .is_err()
        {
            break;
        }
    }
}

/**
* Read data from client and send it to devices
*/
pub async fn dh_writer(
    session: Arc<Mutex<PTCPSession>>,
    socket: Arc<UdpSocket>,
    mut dh_rx: mpsc::Receiver<PTCPEvent>,
    remote_port: u32,
) {
    loop {
        let ev = match dh_rx.recv().await {
            Some(ev) => ev,
            None => break,
        };

        match ev {
            PTCPEvent::Heartbeat => {
                let p = session.lock().unwrap().send(PTCPBody::Heartbeat);
                socket.ptcp_request(p).await;
            }
            PTCPEvent::Connect(realm) => {
                let p = session
                    .lock()
                    .unwrap()
                    .send(PTCPBody::Bind(realm, remote_port));
                socket.ptcp_request(p).await;
            }
            PTCPEvent::Disconnect(realm) => {
                let p = session
                    .lock()
                    .unwrap()
                    .send(PTCPBody::Status(realm, "DISC".to_string()));
                socket.ptcp_request(p).await;
            }
            PTCPEvent::Data(realm, data) => {
                let p = session
                    .lock()
                    .unwrap()
                    .send(PTCPBody::Payload(PTCPPayload { realm, data }));
                socket.ptcp_request(p).await;
            }
        }
    }
}

/// Lee paquetes PTCP del dispositivo y los envia a los clientes TCP.
/// Cuando la sesion muere (timeout o error de red), envia senal por shutdown_tx.
pub async fn dh_reader(
    session: Arc<Mutex<PTCPSession>>,
    socket: Arc<UdpSocket>,
    channels: Arc<Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>>,
    conn_channels: Arc<Mutex<HashMap<u32, oneshot::Sender<bool>>>>,
    shutdown_tx: mpsc::Sender<String>,
) {
    loop {
        let packet = match socket.ptcp_try_read().await {
            Ok(p) => p,
            Err(reason) => {
                println!("[reader] Sesion perdida: {}", reason);
                let _ = shutdown_tx.send(reason).await;
                return;
            }
        };

        let packet = session.lock().unwrap().recv(packet);

        if let PTCPBody::Empty = packet.body {
            continue;
        }

        let p = session.lock().unwrap().send(PTCPBody::Empty);
        socket.ptcp_request(p).await;

        match packet.body {
            PTCPBody::Status(realm, status) => {
                if status == "CONN" {
                    if let Some(tx) = conn_channels.lock().unwrap().remove(&realm) {
                        let _ = tx.send(true);
                    }
                }
            }
            PTCPBody::Payload(p) => {
                let tx = channels.lock().unwrap().get(&p.realm).cloned();
                if let Some(tx) = tx {
                    if tx.send(p.data).await.is_err() {
                        println!("[reader] Realm {:08x} unavailable", p.realm);
                    }
                }
            }
            _ => {}
        }
    }
}
