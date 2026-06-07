use std::path::PathBuf;
use std::fs;
use wpw_core::vault::format::VaultData;
use wpw_core::vault::header::VaultHeader;
use wpw_core::crypto::EncryptionKey;

/// Session state for the host process. Held in memory while the browser keeps the host
/// alive (Chrome may terminate it at any time, which acts as an implicit lock).
///
/// `encryption_key` is an `EncryptionKey` (ZeroizeOnDrop), not a raw `[u8; 32]`, so it
/// zeroizes on `lock()`, on session replacement, and on process drop — including the
/// panic-unwind path that previously left key bytes in the freed allocation.
pub struct HostSession {
    pub encryption_key: Option<EncryptionKey>,
    pub vault_path: PathBuf,
}

impl HostSession {
    pub fn new() -> Self {
        let vault_path = default_vault_path();
        // Inherit any unlocked session from the CLI (same session files on disk).
        let encryption_key = load_cli_session_key();
        Self {
            encryption_key,
            vault_path,
        }
    }

    pub fn locked(&self) -> bool {
        self.encryption_key.is_none()
    }

    pub fn unlock(&mut self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_bytes = fs::read(&self.vault_path)?;
        let header = VaultHeader::parse(&file_bytes)?;

        // Reject obviously-broken KDF params from the header rather than silently
        // substituting defaults (which would derive a different key from the same
        // password and look like "wrong password" to the user).
        if header.t_cost == 0 || header.p_cost == 0 || header.m_cost() < 8 * header.p_cost as u32 {
            return Err("vault header has invalid KDF parameters".into());
        }

        let kdf_params = wpw_core::crypto::kdf::KdfParams {
            m_cost: header.m_cost(),
            t_cost: header.t_cost as u32,
            p_cost: header.p_cost as u32,
        };

        let derived = wpw_core::crypto::kdf::derive_keys(password.as_bytes(), &header.salt, &kdf_params)
            .map_err(|e| format!("KDF failed: {e}"))?;

        let aad = header.aad_bytes();
        let payload_start = header.header_length as usize;
        if file_bytes.len() < payload_start + 16 {
            return Err("Invalid vault file".into());
        }
        let ciphertext_with_tag = &file_bytes[payload_start..];
        let plaintext = wpw_core::crypto::decrypt(&derived.encryption_key, &header.nonce, ciphertext_with_tag, &aad)
            .map_err(|_| "wrong password")?;

        let _vault_data = VaultData::from_msgpack(&plaintext)
            .map_err(|_| "Decryption failed")?;

        // Move the key into our session; old value (if any) zeroizes on Option::replace drop.
        let key_bytes = *derived.encryption_key.expose_key();
        self.encryption_key = Some(EncryptionKey::new(key_bytes));

        Ok(())
    }

    pub fn lock(&mut self) {
        // Setting to None drops the EncryptionKey, which zeroizes the key bytes.
        self.encryption_key = None;
    }

    pub fn decrypt_vault(&self) -> Result<VaultData, Box<dyn std::error::Error>> {
        let key = self.encryption_key.as_ref().ok_or("Vault is locked")?;
        wpw_core::vault::open_vault_with_key(&self.vault_path, key)
            .map_err(|e| e.to_string().into())
    }

    pub fn save_vault(&self, data: &VaultData) -> Result<(), Box<dyn std::error::Error>> {
        let key = self.encryption_key.as_ref().ok_or("Vault is locked")?;
        wpw_core::vault::save_vault_with_key(&self.vault_path, key, data)
            .map_err(|e| e.to_string().into())
    }
}

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("Documents").join("wpw").join("vault.wpw")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".local").join("share").join("wpw").join("vault.wpw")
    }
}

fn load_cli_session_key() -> Option<EncryptionKey> {
    let session_dir = {
        #[cfg(target_os = "windows")]
        {
            let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(base).join("wpw").join("session")
        }
        #[cfg(not(target_os = "windows"))]
        {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".local").join("share").join("wpw").join("session")
        }
    };

    let key_path = session_dir.join("session.key");
    let data_path = session_dir.join("session.dat");

    if !key_path.exists() || !data_path.exists() {
        return None;
    }

    let key_bytes = fs::read(&key_path).ok()?;
    if key_bytes.len() != 32 {
        return None;
    }

    let data = fs::read(&data_path).ok()?;
    if data.len() < 12 + 16 {
        return None;
    }

    let mut session_key = [0u8; 32];
    session_key.copy_from_slice(&key_bytes);

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let ciphertext = &data[12..];

    let wrap_key = EncryptionKey::new(session_key);
    let plaintext = wpw_core::crypto::decrypt(&wrap_key, &nonce, ciphertext, &[]).ok()?;
    // wrap_key drops here — zeroizes session_key bytes inside it.

    if plaintext.len() != 32 {
        return None;
    }

    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&plaintext);
    // `plaintext: Vec<u8>` is freed normally — not zeroized. That's a residual leak for
    // the duration of one allocator slab reuse; acceptable in this threat model since the
    // process address space is already trusted at this point.

    Some(EncryptionKey::new(enc_key))
}

impl Default for HostSession {
    fn default() -> Self {
        Self::new()
    }
}
