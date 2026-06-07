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
    create_vault_with_params(path, password, &KdfParams::default())
}

/// Create a vault using caller-supplied Argon2id parameters. Used by `wpw init` so the
/// values stored under `config.toml` `[kdf]` actually take effect; before this, init
/// hard-coded the defaults regardless of what the user had configured.
///
/// Enforces an OWASP-2023-aligned floor (m_cost ≥ 19 MiB, t_cost ≥ 2, p_cost ≥ 1) so a
/// fat-fingered config can't silently weaken a brand-new vault.
pub fn create_vault_with_params(
    path: &Path,
    password: &[u8],
    params: &KdfParams,
) -> Result<VaultData, VaultError> {
    if params.m_cost < 19_456 || params.t_cost < 2 || params.p_cost < 1 {
        return Err(VaultError::Kdf(format!(
            "KDF parameters too weak: m_cost={} t_cost={} p_cost={} (minimum 19456/2/1)",
            params.m_cost, params.t_cost, params.p_cost
        )));
    }
    let vault_data = VaultData::new();
    save_vault_with_params(path, password, &vault_data, params)?;
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

    decrypt_payload(&file_bytes, &header, &derived.encryption_key)
}

/// Open a vault using an already-derived encryption key, skipping Argon2id.
/// Used when the CLI session cached the key from a prior `unlock`; otherwise this
/// would re-run the KDF (≈1s) on every command.
pub fn open_vault_with_key(path: &Path, key: &crypto::EncryptionKey) -> Result<VaultData, VaultError> {
    if !path.exists() {
        return Err(VaultError::NotFound);
    }
    let file_bytes = fs::read(path)?;
    if file_bytes.len() < header::HEADER_LEN {
        return Err(VaultError::Header(header::HeaderError::TooShort));
    }
    let header = VaultHeader::parse(&file_bytes)?;
    decrypt_payload(&file_bytes, &header, key)
}

fn decrypt_payload(
    file_bytes: &[u8],
    header: &VaultHeader,
    key: &crypto::EncryptionKey,
) -> Result<VaultData, VaultError> {
    let aad = header.aad_bytes();
    let payload_start = header.header_length as usize;
    if file_bytes.len() < payload_start + 16 {
        return Err(VaultError::DecryptionFailed);
    }
    let ciphertext_with_tag = &file_bytes[payload_start..];
    let plaintext = crypto::decrypt(key, &header.nonce, ciphertext_with_tag, &aad)
        .map_err(|_| VaultError::DecryptionFailed)?;
    VaultData::from_msgpack(&plaintext)
        .map_err(|e| VaultError::Serialization(e.to_string()))
}

/// Encrypt and save vault data to file with atomic write.
pub fn save_vault(path: &Path, password: &[u8], data: &VaultData) -> Result<(), VaultError> {
    save_vault_with_params(path, password, data, &KdfParams::default())
}

fn save_vault_with_params(
    path: &Path,
    password: &[u8],
    data: &VaultData,
    params: &KdfParams,
) -> Result<(), VaultError> {
    let plaintext = data.to_msgpack()
        .map_err(|e| VaultError::Serialization(e.to_string()))?;

    let header = VaultHeader::new_random(params.m_cost, params.t_cost as u8, params.p_cost as u8, 0);
    let kdf_params = KdfParams {
        m_cost: header.m_cost(),
        t_cost: header.t_cost as u32,
        p_cost: header.p_cost as u32,
    };

    let derived = crypto::derive_keys(password, &header.salt, &kdf_params)
        .map_err(|e| VaultError::Kdf(e.to_string()))?;

    encrypt_and_write(path, &header, &derived.encryption_key, &plaintext)
}

/// Save a vault using a previously-derived encryption key. The new file is encrypted with a
/// fresh random salt+nonce, so subsequent opens with the master password still work — the
/// key is just re-derived from the new salt next time. However, calling this with a stale
/// key after a `wpw config set kdf.*` change will *not* pick up the new KDF cost: we still
/// derive the next key from the same master password using the *new* salt, but with the
/// hard-coded default cost stored in the header.
pub fn save_vault_with_key(
    path: &Path,
    key: &crypto::EncryptionKey,
    data: &VaultData,
) -> Result<(), VaultError> {
    let plaintext = data.to_msgpack()
        .map_err(|e| VaultError::Serialization(e.to_string()))?;

    // Reuse the existing salt+cost params from the current file so the master password
    // continues to derive the same key. Only the nonce is fresh per write.
    let existing = fs::read(path).ok();
    let header = match existing.as_deref().and_then(|b| VaultHeader::parse(b).ok()) {
        Some(prev) => {
            use rand::RngCore;
            let mut nonce = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce);
            VaultHeader {
                format_version: header::CURRENT_VERSION,
                header_length: header::HEADER_LEN as u32,
                payload_length: 0,
                salt: prev.salt,
                m_cost_exponent: prev.m_cost_exponent,
                t_cost: prev.t_cost,
                p_cost: prev.p_cost,
                nonce,
            }
        }
        None => VaultHeader::new_random(65536, 3, 4, 0),
    };

    encrypt_and_write(path, &header, key, &plaintext)
}

fn encrypt_and_write(
    path: &Path,
    header: &VaultHeader,
    key: &crypto::EncryptionKey,
    plaintext: &[u8],
) -> Result<(), VaultError> {
    let aad = header.aad_bytes();
    let ciphertext = crypto::encrypt(key, &header.nonce, plaintext, &aad)
        .map_err(|_| VaultError::DecryptionFailed)?;

    let mut file_data = Vec::with_capacity(header::HEADER_LEN + ciphertext.len());
    file_data.extend_from_slice(&header.to_bytes());
    file_data.extend_from_slice(&ciphertext);

    write_vault_file_atomic(path, &file_data)
}

/// Write `bytes` to `path` such that a crash never leaves us with neither the old nor the
/// new file:
///   1. write the full payload to `path.wpw.tmp` (so it's already on disk before we touch
///      the original)
///   2. if `path` exists, move it aside to `path.wpw.bak`
///   3. rename the tmp into place
///
/// The previous implementation renamed the original to `.bak` first and ignored the rename
/// error, so a crash between steps 1 and 2 (or a silently-failed `.bak` rename) could leave
/// the user with no readable vault.
pub fn write_vault_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let tmp_path = path.with_extension("wpw.tmp");
    fs::write(&tmp_path, bytes)?;

    if path.exists() {
        let backup_path = path.with_extension("wpw.bak");
        let _ = fs::remove_file(&backup_path);
        if let Err(e) = fs::rename(path, &backup_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(VaultError::Io(e));
        }
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(VaultError::Io(e));
    }

    Ok(())
}
