use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use rand::RngCore;
use std::env;

static ENCRYPTION_KEY: Lazy<Key<Aes256Gcm>> = Lazy::new(|| {
    dotenv().ok();
    let key_b64 = env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY not set");
    let key_bytes = general_purpose::STANDARD
        .decode(&key_b64)
        .expect("Invalid Base64 encryption key");
    if key_bytes.len() != 32 {
        panic!("ENCRYPTION_KEY must be 32 bytes (Base64-encoded)");
    }
    Key::<Aes256Gcm>::from_slice(&key_bytes).to_owned()
});

pub fn encrypt_string(plaintext: &str) -> String {
    let cipher = Aes256Gcm::new(&ENCRYPTION_KEY);

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption failed");

    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    general_purpose::STANDARD.encode(&combined)
}

pub fn decrypt_string(encoded: &str) -> Result<String, String> {
    let data = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "base64 decode failed")?;

    if data.len() < 12 {
        return Err("data too wittle (short)".to_string());
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(&ENCRYPTION_KEY);

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "decryption failed")?;

    String::from_utf8(decrypted_bytes).map_err(|_| "UTF8 decode failed".to_string())
}
