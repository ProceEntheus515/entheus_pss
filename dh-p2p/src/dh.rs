use async_trait::async_trait;
use base64::Engine;
use sha1::Digest;
use std::{collections::HashMap, fmt, net::SocketAddrV4};
use tokio::{net::UdpSocket, time};
use xml::reader::{EventReader, XmlEvent};

use crate::crypto;
use crate::ptcp::{PTCPBody, PTCPSession, PTCP};

static MAIN_SERVER: &str = "www.easy4ipcloud.com:8800";

static CLOUD_USERNAME: &str = "cba1b29e32cb17aa46b8ff9e73c7f40b";
static CLOUD_USERKEY: &str = "996103384cdf19179e19243e959bbf8b";

// ---------------------------------------------------------------------------
// Tipos publicos para el sistema polivalente
// ---------------------------------------------------------------------------

/// Modo de conexion solicitado por el usuario.
/// Auto intenta directo primero y cae a relay si falla.
pub enum ConnectionMode {
    Auto,
    Direct,
    Relay,
}

impl fmt::Display for ConnectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionMode::Auto => write!(f, "auto"),
            ConnectionMode::Direct => write!(f, "direct"),
            ConnectionMode::Relay => write!(f, "relay"),
        }
    }
}

/// Modo de conexion efectivamente establecido.
#[derive(Debug, Clone, Copy)]
pub enum ActiveMode {
    Direct,
    Relay,
}

impl fmt::Display for ActiveMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActiveMode::Direct => write!(f, "DIRECT"),
            ActiveMode::Relay => write!(f, "RELAY"),
        }
    }
}

/// Resultado del handshake: socket conectado, sesion PTCP y modo activo.
pub struct P2PConnection {
    pub socket: UdpSocket,
    pub session: PTCPSession,
    pub mode: ActiveMode,
}

// ---------------------------------------------------------------------------
// Tipos internos
// ---------------------------------------------------------------------------

fn ip_to_bytes_inverted(ip: &str) -> Vec<u8> {
    let addr: SocketAddrV4 = ip.parse().unwrap();
    let ip = addr.ip().octets();
    let port = addr.port();

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&port.to_be_bytes());
    bytes.extend_from_slice(&ip);

    bytes.iter().map(|b| !b).collect()
}

fn ip_to_bytes_raw(ip: &str) -> Vec<u8> {
    let addr: SocketAddrV4 = ip.parse().unwrap();
    let ip = addr.ip().octets();
    let port = addr.port();

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&port.to_be_bytes());
    bytes.extend_from_slice(&ip);

    bytes
}

fn log_raw_packet(label: &str, data: &[u8]) {
    println!("[hexdump] {} ({} bytes)", label, data.len());
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        println!();
    }
}

struct DeviceAuth {
    key: Vec<u8>,
    nonce: u32,
    username: String,
}

// ---------------------------------------------------------------------------
// Handshake polivalente
// ---------------------------------------------------------------------------

pub async fn p2p_handshake(
    socket: UdpSocket,
    serial: String,
    mode: &ConnectionMode,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<P2PConnection, String> {
    let mut cseq = 0;

    let device_auth = match (username, password) {
        (Some(user), Some(pass)) => {
            let key = crypto::get_key(user, pass);
            let nonce = crypto::get_nonce();
            println!("[smart] Credenciales disponibles para '{}'", user);
            Some(DeviceAuth {
                key,
                nonce,
                username: user.to_string(),
            })
        }
        _ => {
            println!("[smart] Sin credenciales, solo modo no-autenticado");
            None
        }
    };

    // --- Fase 1: Descubrimiento P2P ---
    println!("[handshake] Contactando Easy4IPCloud...");
    socket
        .connect(MAIN_SERVER)
        .await
        .map_err(|e| format!("No se pudo conectar a Easy4IPCloud: {}", e))?;

    socket.dh_request("/probe/p2psrv", None, &mut cseq).await;
    socket
        .dh_read()
        .await
        .map_err(|e| format!("[fase:probe] {}", e))?;

    socket
        .dh_request(
            format!("/online/p2psrv/{}", serial).as_ref(),
            None,
            &mut cseq,
        )
        .await;
    let p2psrv_res = socket
        .dh_read()
        .await
        .map_err(|e| format!("[fase:p2psrv] {}", e))?;
    let p2psrv = p2psrv_res
        .body
        .as_ref()
        .and_then(|b| b.get("body/US"))
        .ok_or("[fase:p2psrv] Respuesta sin campo US")?
        .clone();

    socket.dh_request("/online/relay", None, &mut cseq).await;
    let relay_res = socket
        .dh_read()
        .await
        .map_err(|e| format!("[fase:relay-discovery] {}", e))?;
    let relay = relay_res
        .body
        .as_ref()
        .and_then(|b| b.get("body/Address"))
        .ok_or("[fase:relay-discovery] Respuesta sin campo Address")?
        .clone();

    // --- Fase 2: Probe del dispositivo ---
    println!(
        "[handshake] Probing dispositivo {} en P2P server {}...",
        serial, p2psrv
    );
    let socket2 = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("No se pudo crear socket2: {}", e))?;
    socket2.connect(&p2psrv).await.unwrap();

    socket2
        .dh_request(
            format!("/probe/device/{}", serial).as_ref(),
            None,
            &mut cseq,
        )
        .await;
    socket2
        .dh_read()
        .await
        .map_err(|e| format!("[fase:probe-device] {}", e))?;

    // --- Fase 3: Canal P2P (SIEMPRE sin auth primero - deteccion smart) ---
    let cid: [u8; 8] = rand::random();
    let identify = cid
        .iter()
        .map(|b| format!("{:x}", b))
        .collect::<Vec<_>>()
        .join(" ");

    let simple_body = format!(
        "<body><Identify>{}</Identify><IpEncrpt>true</IpEncrpt>\
         <LocalAddr>127.0.0.1:{}</LocalAddr><version>5.0.0</version></body>",
        identify,
        socket.local_addr().unwrap().port(),
    );

    println!("[smart] Probando canal P2P sin autenticacion...");
    socket
        .dh_request(
            format!("/device/{}/p2p-channel", serial).as_ref(),
            Some(&simple_body),
            &mut cseq,
        )
        .await;

    // --- Fase 4: Negociar relay (fase mas fragil - depende de que el dispositivo conecte a tiempo) ---
    println!("[handshake] Negociando relay en {}...", relay);
    socket2.connect(&relay).await.unwrap();

    socket2.dh_request("/relay/agent", None, &mut cseq).await;
    let relay_agent_res = socket2
        .dh_read()
        .await
        .map_err(|e| format!("[fase:relay/agent] {}", e))?;
    let relay_body = relay_agent_res
        .body
        .ok_or("[fase:relay/agent] Respuesta sin body")?;
    let token = DHResponse::get_body_key(&relay_body, "body/Token")
        .ok_or("[fase:relay/agent] Respuesta sin Token (clave body/Token)")?;
    let agent = DHResponse::get_body_key(&relay_body, "body/Agent")
        .ok_or("[fase:relay/agent] Respuesta sin Agent (clave body/Agent)")?;

    println!(
        "[relay] Agent={}, Token={}...{}",
        agent,
        &token[..std::cmp::min(8, token.len())],
        &token[token.len().saturating_sub(8)..]
    );

    socket2.connect(&agent).await.unwrap();

    socket2
        .dh_request(
            format!("/relay/start/{}", token).as_ref(),
            Some("<body><Client>:0</Client></body>"),
            &mut cseq,
        )
        .await;
    socket2.dh_read().await.map_err(|e| {
        format!(
            "[fase:relay/start] {}. Causa probable: el dispositivo no alcanzo \
             el relay agent a tiempo (NAT lento, rate-limiting, o dispositivo offline).",
            e
        )
    })?;

    // --- Fase 5: Leer respuesta del dispositivo (deteccion smart de auth) ---
    println!("[smart] Esperando respuesta del dispositivo (puede tardar si hay NAT lento)...");
    let mut res = socket
        .dh_read_raw()
        .await
        .map_err(|e| format!("[fase:p2p-channel-response] {}", e))?;
    if res.code == 100 {
        println!("[smart] Respuesta provisional (100), esperando respuesta final...");
        res = socket
            .dh_read_raw()
            .await
            .map_err(|e| format!("[fase:p2p-channel-final] {}", e))?;
    }

    let mut authenticated_mode = false;

    if res.code == 403 {
        match &device_auth {
            Some(auth) => {
                println!("[smart] Dispositivo requiere autenticacion (403). Reintentando con IpEncrptV2...");
                let laddr = format!("127.0.0.1:{}", socket.local_addr().unwrap().port());
                let encrypted_laddr = crypto::get_enc(&auth.key, auth.nonce, &laddr);
                let auth_xml =
                    crypto::get_auth(&auth.username, &auth.key, auth.nonce, &encrypted_laddr);
                let auth_body = format!(
                    "<body>{}<Identify>{}</Identify><IpEncrptV2>true</IpEncrptV2>\
                     <LocalAddr>{}</LocalAddr><version>5.0.0</version></body>",
                    auth_xml, identify, encrypted_laddr
                );
                socket
                    .dh_request(
                        format!("/device/{}/p2p-channel", serial).as_ref(),
                        Some(&auth_body),
                        &mut cseq,
                    )
                    .await;
                res = socket
                    .dh_read_raw()
                    .await
                    .map_err(|e| format!("[fase:p2p-channel-auth-response] {}", e))?;
                if res.code == 100 {
                    res = socket
                        .dh_read_raw()
                        .await
                        .map_err(|e| format!("[fase:p2p-channel-auth-final] {}", e))?;
                }
                authenticated_mode = true;
            }
            None => {
                return Err(
                    "Dispositivo requiere autenticacion. Usa --username y --password.".into(),
                );
            }
        }
    }

    if res.code >= 400 {
        return Err(format!(
            "Error del dispositivo: {} {}",
            res.code, res.status
        ));
    }

    // Parsear respuesta del dispositivo
    let data = res.body.ok_or("Respuesta del dispositivo sin body")?;

    // Logs de diagnostico adicionales para entender la politica y parametros del canal P2P
    let policy = data
        .get("body/Policy")
        .cloned()
        .unwrap_or_else(|| "desconocido".into());
    let time = data
        .get("body/Time")
        .cloned()
        .unwrap_or_else(|| "desconocido".into());
    let realm = data
        .get("body/Realm")
        .cloned()
        .unwrap_or_else(|| "desconocido".into());
    let role = data
        .get("body/Role")
        .cloned()
        .unwrap_or_else(|| "desconocido".into());
    println!(
        "[smart] p2p-channel => Policy={}, Time={}, Realm={}, Role={}",
        policy, time, realm, role
    );

    let raw_device_laddr = data
        .get("body/LocalAddr")
        .ok_or("Dispositivo no envio LocalAddr")?
        .clone();
    let device_pub_addr = data
        .get("body/PubAddr")
        .ok_or("Dispositivo no envio PubAddr")?
        .clone();

    let device_nonce_from_response = data.get("body/Nonce").cloned();
    let device_laddr = match (&device_auth, &device_nonce_from_response) {
        (Some(auth), Some(resp_nonce)) if authenticated_mode => {
            let nonce_val: u32 = resp_nonce.parse().unwrap_or(0);
            if nonce_val > 0 {
                let decrypted = crypto::get_dec(&auth.key, nonce_val, &raw_device_laddr);
                println!("[smart] LocalAddr descifrada: {}", decrypted);
                decrypted
            } else {
                raw_device_laddr
            }
        }
        _ => {
            println!("[smart] LocalAddr sin cifrar: {}", raw_device_laddr);
            raw_device_laddr
        }
    };

    println!(
        "[smart] Dispositivo detectado => PubAddr={}, LocalAddr={}, auth={}",
        device_pub_addr,
        device_laddr,
        if authenticated_mode { "si" } else { "no" }
    );

    socket.connect(&device_pub_addr).await.unwrap();

    // --- Fase 6: Crear relay-channel ---
    let relay_channel_body = if authenticated_mode {
        let auth = device_auth.as_ref().unwrap();
        let nonce_val = device_nonce_from_response
            .as_ref()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(auth.nonce);
        let auth_xml = crypto::get_auth(&auth.username, &auth.key, nonce_val, "");
        format!(
            "<body>{}<agentAddr>{}</agentAddr></body>",
            auth_xml, agent
        )
    } else {
        format!("<body><agentAddr>{}</agentAddr></body>", agent)
    };

    println!("[handshake] Creando relay-channel...");
    cseq += 1;
    socket2
        .dh_request_to(
            MAIN_SERVER,
            format!("/device/{}/relay-channel", serial).as_ref(),
            Some(&relay_channel_body),
            &cseq,
        )
        .await;
    socket2
        .dh_read()
        .await
        .map_err(|e| format!("[fase:relay-channel] {}", e))?;

    // --- Fase 7: Sesion PTCP via relay ---
    println!("[handshake] Estableciendo sesion PTCP via relay...");
    let mut relay_session = PTCPSession::new();

    socket2
        .ptcp_request(relay_session.send(PTCPBody::Sync))
        .await;
    relay_session.recv(socket2.ptcp_read().await);

    if matches!(mode, ConnectionMode::Relay) {
        println!("[smart] Modo relay forzado - sesion PTCP lista");
        return Ok(P2PConnection {
            socket: socket2,
            session: relay_session,
            mode: ActiveMode::Relay,
        });
    }

    // --- Fase 8: Obtener Sign via relay ---
    println!("[handshake] Obteniendo Sign via relay...");
    socket2
        .ptcp_request(relay_session.send(PTCPBody::Command(
            b"\x17\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec(),
        )))
        .await;
    let mut ptcp_res = relay_session.recv(socket2.ptcp_read().await);

    while let PTCPBody::Empty = ptcp_res.body {
        ptcp_res = relay_session.recv(socket2.ptcp_read().await);
    }

    let sign = match ptcp_res.body {
        PTCPBody::Command(ref c) => c[12..].to_vec(),
        _ => return Err("Respuesta PTCP invalida al solicitar Sign".into()),
    };

    println!(
        "[handshake] Sign obtenido: {}",
        sign.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    );

    socket2
        .ptcp_request(relay_session.send(PTCPBody::Empty))
        .await;

    // --- Fase 9: Intentar conexion directa P2P ---
    println!("[smart] Intentando conexion directa P2P...");
    match try_direct_p2p(&socket, &device_pub_addr, &device_laddr, &cid, &sign, authenticated_mode)
        .await
    {
        Ok(direct_session) => {
            println!("[smart] Conexion directa P2P establecida!");
            Ok(P2PConnection {
                socket,
                session: direct_session,
                mode: ActiveMode::Direct,
            })
        }
        Err(reason) => {
            if matches!(mode, ConnectionMode::Direct) {
                Err(format!(
                    "Conexion directa fallo y modo es --direct: {}",
                    reason
                ))
            } else {
                println!(
                    "[smart] Directo fallo ({}). Fallback automatico a RELAY.",
                    reason
                );
                Ok(P2PConnection {
                    socket: socket2,
                    session: relay_session,
                    mode: ActiveMode::Relay,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NAT traversal + sesion directa (extraido para manejo limpio de errores)
// ---------------------------------------------------------------------------

async fn try_direct_p2p(
    socket: &UdpSocket,
    device_pub_addr: &str,
    device_laddr: &str,
    cid: &[u8; 8],
    sign: &[u8],
    authenticated: bool,
) -> Result<PTCPSession, String> {
    println!(
        "[nat] Parametros directo: PubAddr={}, LocalAddr={}, auth={}",
        device_pub_addr,
        device_laddr,
        if authenticated { "si" } else { "no" }
    );

    let cookie: [u8; 4] = rand::random();
    let trans_id: [u8; 12] = rand::random();
    let cid_inverted: Vec<u8> = cid.iter().map(|b| !b).collect();

    // Primer paquete STUN a PubAddr (bits invertidos)
    println!("[nat] Enviando STUN a {}...", device_pub_addr);
    let first_packet = [
        b"\xff\xfe\xff\xe7".to_vec(),
        cookie.to_vec(),
        trans_id.to_vec(),
        b"\x7f\xd5\xff\xf7".to_vec(),
        cid_inverted.clone(),
        b"\xff\xfb\xff\xf7\xff\xfe".to_vec(),
        ip_to_bytes_inverted(device_pub_addr),
    ]
    .concat();
    log_raw_packet("[nat] >>", &first_packet);
    socket
        .send(&first_packet)
        .await
        .map_err(|e| format!("Error enviando STUN: {}", e))?;

    let mut buf = [0u8; 4096];
    let n = match time::timeout(time::Duration::from_secs(5), socket.recv(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(format!("NAT traversal rechazado: {}", e)),
        Err(_) => return Err("Timeout NAT traversal (posible NAT simetrico)".into()),
    };
    log_raw_packet("[nat] <<", &buf[0..n]);

    let rtrans_id = buf[8..20].to_vec();

    // Segundo paquete STUN a LocalAddr (bytes SIN invertir)
    let second_packet = [
        b"\xfe\xfe\xff\xe7".to_vec(),
        cookie.to_vec(),
        rtrans_id.clone(),
        b"\x7f\xd6\xff\xf7".to_vec(),
        cid_inverted.clone(),
        b"\xff\xfb\xff\xf7\xff\xfe".to_vec(),
        ip_to_bytes_raw(device_laddr),
    ]
    .concat();
    log_raw_packet("[nat] >>", &second_packet);
    socket
        .send(&second_packet)
        .await
        .map_err(|e| format!("Error STUN LocalAddr: {}", e))?;

    if authenticated {
        // Secuencia extra para modo autenticado
        let recv_result =
            time::timeout(time::Duration::from_secs(5), socket.recv(&mut buf)).await;
        if let Ok(Ok(n)) = recv_result {
            log_raw_packet("[nat] << (extra)", &buf[0..n]);
        }

        let extra_packet = [
            b"\xfe\xfe\xff\xf3".to_vec(),
            cookie.to_vec(),
            rtrans_id.clone(),
            b"\x7f\xd6\xff\xf7".to_vec(),
            cid_inverted.clone(),
            b"\xff\xfb\xff\xf7\xff\xfe".to_vec(),
            ip_to_bytes_raw(device_laddr),
        ]
        .concat();

        for _ in 0..5 {
            let _ = socket.send(&extra_packet).await;
        }
        for _ in 0..5 {
            match time::timeout(time::Duration::from_secs(5), socket.recv(&mut buf)).await {
                Ok(Ok(n)) => log_raw_packet("[nat] <<", &buf[0..n]),
                _ => break,
            }
        }
    } else {
        for _ in 0..5 {
            match time::timeout(time::Duration::from_secs(5), socket.recv(&mut buf)).await {
                Ok(Ok(n)) => log_raw_packet("[nat] <<", &buf[0..n]),
                _ => break,
            }
        }
    }

    // Sesion PTCP directa
    println!("[nat] Estableciendo sesion PTCP directa...");
    let mut session = PTCPSession::new();

    socket.ptcp_request(session.send(PTCPBody::Sync)).await;

    let sync_result =
        time::timeout(time::Duration::from_secs(5), socket.ptcp_read_safe()).await;
    match sync_result {
        Ok(Some(packet)) => {
            let res = session.recv(packet);
            if !matches!(res.body, PTCPBody::Sync) {
                return Err("Respuesta PTCP invalida (no es Sync)".into());
            }
        }
        Ok(None) => return Err("Error leyendo PTCP Sync del dispositivo".into()),
        Err(_) => return Err("Timeout esperando PTCP Sync".into()),
    }

    // Enviar Sign al dispositivo
    println!("[nat] Enviando Sign al dispositivo...");
    socket
        .ptcp_request(
            session.send(PTCPBody::Command(
                [
                    b"\x19\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec(),
                    sign.to_vec(),
                ]
                .concat(),
            )),
        )
        .await;

    let sign_result =
        time::timeout(time::Duration::from_secs(5), socket.ptcp_read_safe()).await;
    let mut ptcp_res = match sign_result {
        Ok(Some(packet)) => session.recv(packet),
        Ok(None) => return Err("Error leyendo respuesta al Sign".into()),
        Err(_) => return Err("Timeout esperando respuesta al Sign".into()),
    };

    while let PTCPBody::Empty = ptcp_res.body {
        let next = time::timeout(time::Duration::from_secs(5), socket.ptcp_read_safe()).await;
        ptcp_res = match next {
            Ok(Some(packet)) => session.recv(packet),
            _ => return Err("Error leyendo respuesta al Sign (loop)".into()),
        };
    }

    match ptcp_res.body {
        PTCPBody::Command(ref c) if c[0] == 0x1A => {
            println!("[nat] Sign aceptado (0x1A)");
        }
        _ => return Err("Dispositivo rechazo Sign".into()),
    }

    // Confirmar sesion directa
    socket
        .ptcp_request(session.send(PTCPBody::Command(
            b"\x1b\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec(),
        )))
        .await;

    let confirm_result =
        time::timeout(time::Duration::from_secs(5), socket.ptcp_read_safe()).await;
    if let Ok(Some(packet)) = confirm_result {
        session.recv(packet);
    }

    println!("[nat] Sesion PTCP directa confirmada");
    Ok(session)
}

// ---------------------------------------------------------------------------
// Parser de respuestas DH (sin cambios funcionales)
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[allow(dead_code)]
struct DHResponse {
    version: String,
    code: u16,
    status: String,
    headers: HashMap<String, String>,
    body: Option<HashMap<String, String>>,
}

impl DHResponse {
    fn parse_body(body: &str) -> HashMap<String, String> {
        let mut parser = EventReader::from_str(body);
        let mut stack = Vec::new();
        let mut tree = HashMap::new();

        loop {
            match parser.next() {
                Ok(XmlEvent::StartElement { name, .. }) => {
                    stack.push(name.local_name);
                }
                Ok(XmlEvent::EndElement { .. }) => {
                    stack.pop().unwrap();
                }
                Ok(XmlEvent::Characters(s)) => {
                    let key = stack.as_slice().join("/");
                    tree.insert(key, s);
                }
                Ok(XmlEvent::EndDocument) => {
                    break;
                }
                Err(e) => panic!("[fatal] Error parseando XML: {}", e),
                _ => {}
            }
        }

        tree
    }

    /// Obtiene un valor del body probando la clave y su variante en minusculas (por si el servidor normaliza XML).
    fn get_body_key(body: &HashMap<String, String>, key: &str) -> Option<String> {
        body.get(key)
            .or_else(|| body.get(&key.to_lowercase()))
            .cloned()
    }

    fn parse_response(res: &str) -> DHResponse {
        let mut parts = res.splitn(2, "\r\n\r\n");
        let head = parts.next().unwrap();
        let body = parts.next().unwrap_or("");

        let mut head_parts = head.split("\r\n");
        let mut status_line = head_parts.next().unwrap().split(" ");
        let version = status_line.next().unwrap().to_string();
        let code = status_line.next().unwrap().parse::<u16>().unwrap();
        let status = status_line.next().unwrap().to_string();

        let mut headers = HashMap::new();
        for line in head_parts {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, ": ");
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            headers.insert(key, value);
        }

        let body = match body.trim().len() {
            0 => None,
            _ => Some(DHResponse::parse_body(body)),
        };

        DHResponse {
            version,
            code,
            status,
            headers,
            body,
        }
    }
}

// ---------------------------------------------------------------------------
// Traits de comunicacion DH y PTCP safe
// ---------------------------------------------------------------------------

#[async_trait]
trait DHP2P {
    async fn dh_request(&self, path: &str, body: Option<&str>, seq: &mut u32);
    async fn dh_request_to(&self, addr: &str, path: &str, body: Option<&str>, seq: &u32);
    async fn dh_read_raw(&self) -> Result<DHResponse, String>;

    async fn dh_read(&self) -> Result<DHResponse, String> {
        let res = self.dh_read_raw().await?;
        if res.code >= 300 {
            return Err(format!("Error del servidor: {} {}", res.code, res.status));
        }
        Ok(res)
    }
}

#[async_trait]
trait PTCPSafe {
    async fn ptcp_read_safe(&self) -> Option<crate::ptcp::PTCPPacket>;
}

#[async_trait]
impl PTCPSafe for UdpSocket {
    async fn ptcp_read_safe(&self) -> Option<crate::ptcp::PTCPPacket> {
        let mut buf = [0u8; 4096];
        match self.recv(&mut buf).await {
            Ok(n) => {
                let packet = crate::ptcp::PTCPPacket::parse(&buf[0..n]);
                println!("<<< {}", self.peer_addr().unwrap());
                println!("{:?}", packet);
                println!("---");
                Some(packet)
            }
            Err(e) => {
                println!("[error] Error leyendo PTCP: {}", e);
                None
            }
        }
    }
}

#[async_trait]
impl DHP2P for UdpSocket {
    async fn dh_request(&self, path: &str, body: Option<&str>, seq: &mut u32) {
        let method = match body {
            Some(_) => "DHPOST",
            None => "DHGET",
        };

        let body = match body {
            Some(s) => s,
            None => "",
        };

        let nonce = rand::random::<u32>();
        let currdate = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let pwd = format!("{}{}DHP2P:{}:{}", nonce, currdate, CLOUD_USERNAME, CLOUD_USERKEY);

        let mut hasher = sha1::Sha1::new();
        hasher.update(pwd);
        let hash_digest = hasher.finalize();
        let digest = base64::engine::general_purpose::STANDARD.encode(&hash_digest);

        *seq += 1;

        let req = format!(
            "{} {} HTTP/1.1\r\n\
            CSeq: {}\r\n\
            Authorization: WSSE profile=\"UsernameToken\"\r\n\
            X-WSSE: UsernameToken Username=\"{}\", PasswordDigest=\"{}\", Nonce=\"{}\", Created=\"{}\"\r\n\r\n{}",
            method, path, seq, CLOUD_USERNAME, digest, nonce, currdate, body,
        );

        println!(">>> {}", self.peer_addr().unwrap());
        println!("{}", req);
        println!("---");

        log_raw_packet("[dh] >>", req.as_bytes());

        self.send(req.as_bytes()).await.unwrap();
    }

    async fn dh_request_to(&self, addr: &str, path: &str, body: Option<&str>, seq: &u32) {
        let method = match body {
            Some(_) => "DHPOST",
            None => "DHGET",
        };

        let body = match body {
            Some(s) => s,
            None => "",
        };

        let nonce = rand::random::<u32>();
        let currdate = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let pwd = format!("{}{}DHP2P:{}:{}", nonce, currdate, CLOUD_USERNAME, CLOUD_USERKEY);

        let mut hasher = sha1::Sha1::new();
        hasher.update(pwd);
        let hash_digest = hasher.finalize();
        let digest = base64::engine::general_purpose::STANDARD.encode(&hash_digest);

        let req = format!(
            "{} {} HTTP/1.1\r\n\
            CSeq: {}\r\n\
            Authorization: WSSE profile=\"UsernameToken\"\r\n\
            X-WSSE: UsernameToken Username=\"{}\", PasswordDigest=\"{}\", Nonce=\"{}\", Created=\"{}\"\r\n\r\n{}",
            method, path, seq, CLOUD_USERNAME, digest, nonce, currdate, body,
        );

        println!(">>> {} (via send_to)", addr);
        println!("{}", req);
        println!("---");

        log_raw_packet("[dh] >> (via send_to)", req.as_bytes());

        use std::net::ToSocketAddrs;
        let dest: std::net::SocketAddr = addr
            .to_socket_addrs()
            .expect("Direccion invalida")
            .next()
            .expect("Sin direccion resuelta");
        self.send_to(req.as_bytes(), dest).await.unwrap();
    }

    async fn dh_read_raw(&self) -> Result<DHResponse, String> {
        let peer = self
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "desconocido".into());
        println!("### esperando respuesta de {}...", peer);

        let mut buf = [0u8; 4096];
        let n = match time::timeout(time::Duration::from_secs(15), self.recv(&mut buf)).await {
            Ok(Ok(0)) => return Err(format!("Conexion cerrada por {} (0 bytes)", peer)),
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("Error de red leyendo de {}: {}", peer, e)),
            Err(_) => {
                return Err(format!(
                    "Timeout (15s) esperando respuesta de {}. Posibles causas: \
                     rate-limiting de Dahua, dispositivo no alcanzo el relay, \
                     o el servidor no respondio.",
                    peer
                ))
            }
        };

        let label = format!("DH desde {}", peer);
        log_raw_packet(&label, &buf[0..n]);

        let raw = String::from_utf8_lossy(&buf[0..n]);

        // Diagnostico: detectar paquetes UDP con XML duplicado/corrupto
        let xml_fragments = raw.matches("<?xml").count();
        if xml_fragments > 1 {
            println!(
                "[diag] WARN: Respuesta de {} contiene {} fragmentos XML en {} bytes (datos duplicados en UDP)",
                peer, xml_fragments, n
            );
        }

        println!("<<< {} ({} bytes)", peer, n);
        println!("{}", raw);
        println!("---");

        let res = DHResponse::parse_response(&raw);

        // Diagnostico: verificar Content-Length vs bytes reales
        if let Some(cl) = res.headers.get("Content-Length") {
            if let Ok(expected) = cl.parse::<usize>() {
                let header_end = raw.find("\r\n\r\n").unwrap_or(0) + 4;
                let actual_body_len = n.saturating_sub(header_end);
                if actual_body_len != expected {
                    println!(
                        "[diag] WARN: Content-Length={} pero body real={} bytes (delta={})",
                        expected,
                        actual_body_len,
                        (actual_body_len as i64) - (expected as i64)
                    );
                }
            }
        }

        println!("{:?}", res);
        Ok(res)
    }
}
