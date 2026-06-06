use std::path::PathBuf;
use std::fs;
use zeroize::Zeroize;
use wpw_core::vault::format::VaultData;
use wpw_core::vault::header::{VaultHeader, self};
use wpw_core::crypto::EncryptionKey;

/// Session state for the host process.
/// The host keeps the encryption_key and header params in memory while unlocked.
pub struct HostSession {
    pub locked: bool,
    pub encryption_key: Option<[u8; 32]>,
    pub vault_path: PathBuf,
    /// Stored header params for re-encryption without password
    salt: [u8; 32],
    m_cost_exponent: u8,
    t_cost: u8,
    p_cost: u8,
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

impl HostSession {
    pub fn new() -> Self {
        let vault_path = default_vault_path();
        let _exists = vault_path.exists();
        
        // Try to load from CLI session
        let encryption_key = load_cli_session_key();
        let locked = encryption_key.is_none();
        
        // If loaded from CLI session, also read the vault header to get params
        let (salt, m_cost_exponent, t_cost, p_cost) = if encryption_key.is_some() {
            read_vault_params(&vault_path).unwrap_or_default()
        } else {
            ([0u8; 32], 0, 0, 0)
        };
        
        Self {
            locked,
            encryption_key,
            vault_path,
            salt,
            m_cost_exponent,
            t_cost,
            p_cost,
        }
    }
    
    pub fn unlock(&mut self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_bytes = fs::read(&self.vault_path)?;
        let header = VaultHeader::parse(&file_bytes)?;
        
        // Validate KDF params from header; use safe defaults if corrupted
        let kdf_params = validate_kdf_params(header.t_cost, header.p_cost, header.m_cost());
        
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
        
        self.encryption_key = Some(*derived.encryption_key.expose_key());
        self.salt = header.salt;
        self.m_cost_exponent = header.m_cost_exponent;
        self.t_cost = header.t_cost;
        self.p_cost = header.p_cost;
        self.locked = false;
        
        Ok(())
    }
    
    pub fn lock(&mut self) {
        if let Some(ref mut key) = self.encryption_key {
            key.zeroize();
        }
        self.encryption_key = None;
        self.salt.zeroize();
        self.locked = true;
    }
    
    /// Decrypt vault using the stored encryption key (no password needed).
    pub fn decrypt_vault(&self) -> Result<VaultData, Box<dyn std::error::Error>> {
        if self.locked {
            return Err("Vault is locked".into());
        }
        
        let key = self.encryption_key.ok_or("No encryption key")?;
        let file_bytes = fs::read(&self.vault_path)?;
        let header = VaultHeader::parse(&file_bytes)?;
        let enc_key = EncryptionKey::new(key);
        let aad = header.aad_bytes();
        
        let payload_start = header.header_length as usize;
        if file_bytes.len() < payload_start + 16 {
            return Err("Invalid vault file".into());
        }
        
        let ciphertext_with_tag = &file_bytes[payload_start..];
        let plaintext = wpw_core::crypto::decrypt(&enc_key, &header.nonce, ciphertext_with_tag, &aad)
            .map_err(|_| "Decryption failed")?;
        
        let vault_data = VaultData::from_msgpack(&plaintext)
            .map_err(|e| format!("Deserialization error: {e}"))?;
        
        Ok(vault_data)
    }
    
    /// Save vault using the stored encryption key (no password needed).
    /// Uses validated params to prevent corrupting the file.
    pub fn save_vault(&self, data: &VaultData) -> Result<(), Box<dyn std::error::Error>> {
        if self.locked {
            return Err("Vault is locked".into());
        }
        
        let key = self.encryption_key.ok_or("No encryption key")?;
        
        // Validate params; fall back to defaults if corrupted
        let t_cost = if self.t_cost >= 1 { self.t_cost } else { 3 };
        let p_cost = if self.p_cost >= 1 { self.p_cost } else { 4 };
        let m_cost_exponent = if self.m_cost_exponent >= 1 { self.m_cost_exponent } else { 16 };
        let salt = if self.salt.iter().any(|&b| b != 0) { self.salt } else {
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let mut s = [0u8; 32];
            rng.fill_bytes(&mut s);
            s
        };
        
        let plaintext = data.to_msgpack()
            .map_err(|e| format!("Serialization error: {e}"))?;
        
        // Generate new nonce for each save
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut nonce = [0u8; 12];
        rng.fill_bytes(&mut nonce);
        
        let header = VaultHeader {
            format_version: wpw_core::vault::header::CURRENT_VERSION,
            header_length: wpw_core::vault::header::HEADER_LEN as u32,
            payload_length: 0,
            salt,
            m_cost_exponent,
            t_cost,
            p_cost,
            nonce,
        };
        
        let enc_key = EncryptionKey::new(key);
        let aad = header.aad_bytes();
        let ciphertext = wpw_core::crypto::encrypt(&enc_key, &nonce, &plaintext, &aad)
            .map_err(|_| "Encryption failed")?;
        
        let mut file_data = Vec::with_capacity(header::HEADER_LEN + ciphertext.len());
        file_data.extend_from_slice(&header.to_bytes());
        file_data.extend_from_slice(&ciphertext);
        
        // Atomic write
        if self.vault_path.exists() {
            let backup_path = self.vault_path.with_extension("wpw.bak");
            let _ = fs::rename(&self.vault_path, &backup_path);
        }
        
        let tmp_path = self.vault_path.with_extension("wpw.tmp");
        fs::write(&tmp_path, &file_data)?;
        fs::rename(&tmp_path, &self.vault_path)?;
        
        Ok(())
    }
}

/// Read KDF params from the vault header.
fn read_vault_params(vault_path: &PathBuf) -> Option<([u8; 32], u8, u8, u8)> {
    let file_bytes = fs::read(vault_path).ok()?;
    if file_bytes.len() < header::HEADER_LEN {
        return None;
    }
    let header = VaultHeader::parse(&file_bytes).ok()?;
    Some((header.salt, header.m_cost_exponent, header.t_cost, header.p_cost))
}

/// Validate KDF params; return safe defaults if corrupted.
fn validate_kdf_params(t_cost: u8, p_cost: u8, m_cost: u32) -> wpw_core::crypto::kdf::KdfParams {
    let t = if t_cost >= 1 { t_cost as u32 } else { 3 };
    let p = if p_cost >= 1 { p_cost as u32 } else { 4 };
    let m = if m_cost >= 8 * p { m_cost } else { 65536 };
    wpw_core::crypto::kdf::KdfParams { m_cost: m, t_cost: t, p_cost: p }
}

fn load_cli_session_key() -> Option<[u8; 32]> {
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
    
    let key = wpw_core::crypto::EncryptionKey::new(session_key);
    let plaintext = wpw_core::crypto::decrypt(&key, &nonce, ciphertext, &[]).ok()?;
    
    if plaintext.len() != 32 {
        return None;
    }
    
    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&plaintext);
    
    Some(enc_key)
}

impl Default for HostSession {
    fn default() -> Self {
        Self::new()
    }
}
