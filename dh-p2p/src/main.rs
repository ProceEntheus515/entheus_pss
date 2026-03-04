use clap::Parser;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{mpsc, oneshot},
    time::Duration,
};

use crate::{
    dh::{p2p_handshake, ActiveMode, ConnectionMode},
    process::{dh_reader, dh_writer, process_reader, process_writer},
    ptcp::PTCPEvent,
};

mod crypto;
mod dh;
mod process;
mod ptcp;

#[derive(Parser)]
#[command(about = "Polivalent Dahua P2P tunnel - auto-detects auth and connection mode.", long_about = None)]
struct Cli {
    /// Bind address, port and remote port. Default: 127.0.0.1:1554:554
    #[arg(short, long, value_name = "[bind_address:]port:remote_port")]
    port: Option<String>,

    /// Force relay mode (skip direct P2P attempt)
    #[arg(short, long)]
    relay: bool,

    /// Force direct P2P mode (fail if NAT blocks)
    #[arg(short, long)]
    direct: bool,

    /// Username for device authentication (used only if device requires it)
    #[arg(short, long)]
    username: Option<String>,

    /// Password for device authentication
    #[arg(long)]
    password: Option<String>,

    /// Serial number of the camera
    serial: String,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let serial = args.serial;
    let port = args.port.unwrap_or("127.0.0.1:1554:554".to_string());

    let parts: Vec<&str> = port.split(':').collect();
    let (bind_address, bind_port, remote_port): (&str, u16, u16) = match parts.len() {
        2 => (
            "127.0.0.1",
            parts[0].parse().unwrap(),
            parts[1].parse().unwrap(),
        ),
        3 => (
            parts[0],
            parts[1].parse().unwrap(),
            parts[2].parse().unwrap(),
        ),
        _ => panic!("Formato de puerto invalido. Usa: [bind_address:]port:remote_port"),
    };

    let mode = if args.relay && args.direct {
        panic!("No se puede usar --relay y --direct al mismo tiempo");
    } else if args.relay {
        ConnectionMode::Relay
    } else if args.direct {
        ConnectionMode::Direct
    } else {
        ConnectionMode::Auto
    };

    let listener = TcpListener::bind(format!("{}:{}", bind_address, bind_port))
        .await
        .unwrap();

    println!("============================================");
    println!("  dh-p2p polivalente v0.2.0");
    println!("============================================");
    println!("[config] Modo: {}", mode);
    println!("[config] Serial: {}", serial);
    println!("[config] Escuchando en {}:{}", bind_address, bind_port);
    println!("[config] Puerto remoto: {}", remote_port);
    if args.username.is_some() {
        println!("[config] Credenciales: proporcionadas");
    } else {
        println!("[config] Credenciales: no (se usaran si el dispositivo las requiere)");
    }
    if remote_port == 554 {
        println!(
            "[config] RTSP URL: rtsp://{}{}/cam/realmonitor?channel=1&subtype=0",
            bind_address,
            if bind_port != 554 {
                format!(":{}", bind_port)
            } else {
                String::new()
            }
        );
    }
    println!("============================================");

    loop {
        // Esperar a que un cliente se conecte ANTES de establecer el tunel.
        // En modo relay (Time=30), esto garantiza que el timer empiece fresco.
        println!("[tunnel] Esperando conexion de cliente...");

        let (pending_client, pending_addr) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                println!("[tunnel] Error aceptando conexion: {}", e);
                continue;
            }
        };

        println!(
            "[tunnel] Cliente {} conectado. Estableciendo tunel P2P on-demand...",
            pending_addr
        );

        // Establecer el tunel AHORA (relay timer empieza fresco). Reintento 1 vez si falla.
        let connection = {
            let mut last_err = String::new();
            let mut conn_opt = None;
            for handshake_attempt in 0..2 {
                if handshake_attempt > 0 {
                    println!("[tunnel] Reintento de handshake (2/2)...");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                let sock = match UdpSocket::bind("0.0.0.0:0").await {
                    Ok(s) => s,
                    Err(e) => {
                        last_err = format!("Error creando socket: {}", e);
                        println!("[tunnel] {}", last_err);
                        break;
                    }
                };
                match p2p_handshake(
                    sock,
                    serial.clone(),
                    &mode,
                    args.username.as_deref(),
                    args.password.as_deref(),
                )
                .await
                {
                    Ok(conn) => {
                        conn_opt = Some(conn);
                        break;
                    }
                    Err(e) => last_err = e,
                }
            }
            match conn_opt {
                Some(conn) => conn,
                None => {
                    println!("[tunnel] Handshake fallido: {}", last_err);
                    println!("[tunnel] El cliente sera desconectado. Esperando nuevo cliente...");
                    drop(pending_client);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }
            }
        };

        let is_relay = matches!(connection.mode, ActiveMode::Relay);
        println!("============================================");
        println!("[tunnel] Conectado via {}", connection.mode);
        if is_relay {
            println!("[tunnel] Relay activo - ventana de datos ~30s");
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<String>(1);
        let (dh_tx, dh_rx) = mpsc::channel::<PTCPEvent>(128);

        let session = Arc::new(Mutex::new(connection.session));
        let channels = Arc::new(Mutex::new(HashMap::<u32, mpsc::Sender<Vec<u8>>>::new()));
        let conn_channels = Arc::new(Mutex::new(HashMap::<u32, oneshot::Sender<bool>>::new()));

        let reader_socket = Arc::new(connection.socket);
        let writer_socket = reader_socket.clone();

        let writer_session = session.clone();
        let reader_session = session.clone();
        let reader_channels = channels.clone();
        let accept_channels = channels.clone();
        let reader_conn_channels = conn_channels.clone();
        let accept_conn_channels = conn_channels.clone();

        let hb_tx = dh_tx.clone();
        let hb_interval_secs = if is_relay { 3 } else { 5 };
        let hb_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(hb_interval_secs)).await;
                if hb_tx.send(PTCPEvent::Heartbeat).await.is_err() {
                    break;
                }
            }
        });

        let writer_handle = tokio::spawn(async move {
            dh_writer(writer_session, writer_socket, dh_rx, remote_port.into()).await;
        });

        let reader_shutdown = shutdown_tx.clone();
        let reader_handle = tokio::spawn(async move {
            dh_reader(
                reader_session,
                reader_socket,
                reader_channels,
                reader_conn_channels,
                reader_shutdown,
            )
            .await;
        });

        // Procesar el cliente que disparo el handshake INMEDIATAMENTE
        let client_ok = {
            let (tx, rx) = mpsc::channel::<Vec<u8>>(128);
            let (conn_tx, conn_rx) = oneshot::channel::<bool>();
            let dh_tx_clone = dh_tx.clone();
            let realm_id = rand::random::<u32>();

            accept_channels.lock().unwrap().insert(realm_id, tx);
            accept_conn_channels
                .lock()
                .unwrap()
                .insert(realm_id, conn_tx);

            if dh_tx_clone
                .send(PTCPEvent::Connect(realm_id))
                .await
                .is_err()
            {
                println!("[tunnel] Canal PTCP cerrado antes de conectar cliente");
                false
            } else {
                match conn_rx.await {
                    Ok(_) => {
                        println!("[tunnel] Cliente {} enrutado al dispositivo", pending_addr);
                        let (tcp_reader, tcp_writer) = pending_client.into_split();
                        let dh_tx_reader = dh_tx_clone.clone();
                        tokio::spawn(async move {
                            process_reader(tcp_reader, realm_id, dh_tx_reader).await;
                        });
                        tokio::spawn(async move {
                            process_writer(tcp_writer, rx).await;
                        });
                        true
                    }
                    Err(_) => {
                        println!("[tunnel] Dispositivo no confirmo conexion");
                        false
                    }
                }
            }
        };

        if !client_ok {
            hb_handle.abort();
            writer_handle.abort();
            reader_handle.abort();
            println!("[tunnel] Reintentando en 3 segundos...");
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        }

        println!("[tunnel] Tunel activo. Aceptando conexiones adicionales...");
        println!("============================================");

        // Aceptar conexiones adicionales hasta que la sesion muera
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((client, addr)) => {
                            println!("[tunnel] Conexion adicional desde {}", addr);

                            let (tx, rx) = mpsc::channel::<Vec<u8>>(128);
                            let (conn_tx, conn_rx) = oneshot::channel::<bool>();
                            let dh_tx = dh_tx.clone();
                            let realm_id = rand::random::<u32>();

                            accept_channels.lock().unwrap().insert(realm_id, tx);
                            accept_conn_channels.lock().unwrap().insert(realm_id, conn_tx);

                            if dh_tx.send(PTCPEvent::Connect(realm_id)).await.is_err() {
                                println!("[tunnel] Canal PTCP cerrado, reconectando...");
                                break;
                            }

                            match conn_rx.await {
                                Ok(_) => {
                                    let (tcp_reader, tcp_writer) = client.into_split();
                                    tokio::spawn(async move {
                                        process_reader(tcp_reader, realm_id, dh_tx).await;
                                    });
                                    tokio::spawn(async move {
                                        process_writer(tcp_writer, rx).await;
                                    });
                                }
                                Err(_) => {
                                    println!("[tunnel] Conexion rechazada (sesion muerta)");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            println!("[tunnel] Error aceptando conexion: {}", e);
                        }
                    }
                }
                reason = shutdown_rx.recv() => {
                    let reason = reason.unwrap_or_else(|| "Canal cerrado".to_string());
                    println!("[tunnel] Sesion perdida: {}", reason);
                    break;
                }
            }
        }

        hb_handle.abort();
        writer_handle.abort();
        reader_handle.abort();

        println!("[tunnel] Tunel cerrado. Esperando nuevo cliente...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
