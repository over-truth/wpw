pub mod entry;
pub mod format;
pub mod header;

use std::path::Path;
use std::fs;
use crate::crypto;
use crate::crypto::kdf::KdfParams;
use self::header::VaultHeader;
use self::format::VaultData;

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("header error: {0}")]
    Header(#[from] header::HeaderError),
    #[error("KDF error: {0}")]
    Kdf(String),
    #[error("decryption failed (wrong password or corrupted data)")]
    DecryptionFailed,
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("vault file not found")]
    NotFound,
}

/// Create a new vault file at the given path with the given master password.
pub fn create_vault(path: &Path, password: &[u8]) -> Result<VaultData, VaultError> {
    let vault_data = VaultData::new();
    save_vault(path, password, &vault_data)?;
    Ok(vault_data)
}

/// Open and decrypt a vault file.
pub fn open_vault(path: &Path, password: &[u8]) -> Result<VaultData, VaultError> {
    if !path.exists() {
        return Err(VaultError::NotFound);
    }

    let file_bytes = fs::read(path)?;
    if file_bytes.len() < header::HEADER_LEN {
        return Err(VaultError::Header(header::HeaderError::TooShort));
    }

    let header = VaultHeader::parse(&file_bytes)?;

    let kdf_params = KdfParams {
        m_cost: header.m_cost(),
        t_cost: header.t_cost as u32,
        p_cost: header.p_cost as u32,
    };

    let derived = crypto::derive_keys(password, &header.salt, &kdf_params)
        .map_err(|e| VaultError::Kdf(e.to_string()))?;

    let aad = header.aad_bytes();

    // Payload starts at header.header_length
    let payload_start = header.header_length as usize;
    if file_bytes.len() < payload_start + 16 {
        return Err(VaultError::DecryptionFailed);
    }

    // ciphertext_with_tag = ciphertext + GCM auth tag (16 bytes appended by aes-gcm)
    let ciphertext_with_tag = &file_bytes[payload_start..];

    let plaintext = crypto::decrypt(&derived.encryption_key, &header.nonce, ciphertext_with_tag, &aad)
        .map_err(|_| VaultError::DecryptionFailed)?;

    let vault_data = VaultData::from_msgpack(&plaintext)
        .map_err(|e| VaultError::Serialization(e.to_string()))?;

    Ok(vault_data)
}

/// Encrypt and save vault data to file with atomic write.
pub fn save_vault(path: &Path, password: &[u8], data: &VaultData) -> Result<(), VaultError> {
    let plaintext = data.to_msgpack()
        .map_err(|e| VaultError::Serialization(e.to_string()))?;

    let header = VaultHeader::new_random(65536, 3, 4, 0); // payload_length set below
    let kdf_params = KdfParams {
        m_cost: header.m_cost(),
        t_cost: header.t_cost as u32,
        p_cost: header.p_cost as u32,
    };

    let derived = crypto::derive_keys(password, &header.salt, &kdf_params)
        .map_err(|e| VaultError::Kdf(e.to_string()))?;

    let aad = header.aad_bytes();
    let ciphertext = crypto::encrypt(&derived.encryption_key, &header.nonce, &plaintext, &aad)
        .map_err(|_| VaultError::DecryptionFailed)?;

    // Build the file: header + ciphertext (auth tag is included in ciphertext by aes-gcm)
    let mut file_data = Vec::with_capacity(header::HEADER_LEN + ciphertext.len());
    file_data.extend_from_slice(&header.to_bytes());
    file_data.extend_from_slice(&ciphertext);

    // Atomic write: backup existing, write to tmp, rename
    if path.exists() {
        let backup_path = path.with_extension("wpw.bak");
        let _ = fs::rename(path, &backup_path);
    }

    let tmp_path = path.with_extension("wpw.tmp");
    fs::write(&tmp_path, &file_data)?;

    // Atomic rename
    #[cfg(target_os = "windows")]
    {
        // Windows: rename over existing file
        fs::rename(&tmp_path, path)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(&tmp_path, path)?;
    }

    Ok(())
}
