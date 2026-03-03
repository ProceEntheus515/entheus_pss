use aes::Aes256;
use base64::Engine;
use cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use ofb::Ofb;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type Aes256Ofb = Ofb<Aes256>;
type HmacSha256 = Hmac<Sha256>;

static RANDSALT: &str = "5daf91fc5cfc1be8e081cfb08f792726";
static IV: &[u8; 16] = b"2z52*lk9o6HRyJrf";

/// Genera la clave de autenticacion del dispositivo a partir de usuario y contrasena
/// Replica la logica de Python: MD5("{user}:Login to {RANDSALT}:{pass}").hexdigest().upper()
pub fn get_key(username: &str, password: &str) -> Vec<u8> {
    let input = format!("{}:Login to {}:{}", username, RANDSALT, password);
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hex = format!("{:X}", result);
    hex.into_bytes()
}

/// Genera un nonce aleatorio en el rango [0, 2^31).
pub fn get_nonce() -> u32 {
    rand::random::<u32>() & 0x7FFFFFFF
}

/// Deriva una clave AES-256 usando PBKDF2-HMAC-SHA256 y cifra `data` con AES-256-OFB.
/// Devuelve el resultado en base64.
fn derive_and_crypt(key: &[u8], nonce: u32, data: &[u8]) -> Vec<u8> {
    let salt = nonce.to_string();
    let mut dk = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(key, salt.as_bytes(), 20_000, &mut dk);

    let mut cipher = Aes256Ofb::new(dk.as_ref().into(), IV.into());
    let mut buf = data.to_vec();
    cipher.apply_keystream(&mut buf);
    buf
}

/// Cifra `data` con AES-256-OFB derivando la clave via PBKDF2. Devuelve base64.
pub fn get_enc(key: &[u8], nonce: u32, data: &str) -> String {
    let encrypted = derive_and_crypt(key, nonce, data.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(&encrypted)
}

/// Descifra `data` (base64) con AES-256-OFB derivando la clave via PBKDF2. Devuelve el texto plano.
pub fn get_dec(key: &[u8], nonce: u32, data: &str) -> String {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("Base64 invalido en respuesta del dispositivo");
    let decrypted = derive_and_crypt(key, nonce, &decoded);
    String::from_utf8(decrypted).expect("UTF-8 invalido tras descifrar LocalAddr")
}

/// Genera el bloque XML de autenticacion del dispositivo.
/// Replica: HMAC-SHA256(key, "{nonce}{timestamp}{payload}") -> base64
pub fn get_auth(username: &str, key: &[u8], nonce: u32, payload: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Error obteniendo timestamp")
        .as_secs();

    let message = format!("{}{}{}", nonce, timestamp, payload);

    let mut mac =
        HmacSha256::new_from_slice(key).expect("HMAC puede aceptar claves de cualquier tamano");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let auth_b64 = base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

    format!(
        "<CreateDate>{}</CreateDate>\
         <DevAuth>{}</DevAuth>\
         <Nonce>{}</Nonce>\
         <RandSalt>{}</RandSalt>\
         <UserName>{}</UserName>",
        timestamp, auth_b64, nonce, RANDSALT, username
    )
}
