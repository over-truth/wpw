use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};

use crate::crypto::key::EncryptionKey;

/// Encrypt plaintext using AES-256-GCM.
/// Returns ciphertext (includes authentication tag appended by aes-gcm crate).
/// nonce: 12 bytes, randomly generated for each encryption.
/// aad: additional authenticated data (header immutable fields).
pub fn encrypt(
    key: &EncryptionKey,
    nonce: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(key.expose_key())
        .map_err(|_| aes_gcm::Error)?;
    let nonce = Nonce::from_slice(nonce);

    let payload = Payload {
        msg: plaintext,
        aad,
    };
    cipher.encrypt(nonce, payload)
}

/// Decrypt ciphertext using AES-256-GCM.
/// Returns plaintext if decryption and authentication succeed.
pub fn decrypt(
    key: &EncryptionKey,
    nonce: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(key.expose_key())
        .map_err(|_| aes_gcm::Error)?;
    let nonce = Nonce::from_slice(nonce);

    let payload = Payload {
        msg: ciphertext,
        aad,
    };
    cipher.decrypt(nonce, payload)
}