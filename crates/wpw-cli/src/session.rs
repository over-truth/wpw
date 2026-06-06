use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;
use wpw_core::crypto::EncryptionKey;

pub struct Session {
    pub encryption_key: EncryptionKey,
    pub unlock_time: u64,
}

impl Session {
    pub fn new(encryption_key: EncryptionKey) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self { encryption_key, unlock_time: now }
    }
}

fn session_dir() -> PathBuf {
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
}

fn session_key_path() -> PathBuf {
    session_dir().join("session.key")
}

fn session_data_path() -> PathBuf {
    session_dir().join("session.dat")
}

/// Try to load the session. Returns None if not unlocked or expired.
pub fn load_session(timeout_secs: u64) -> Option<Vec<u8>> {
    let key_path = session_key_path();
    let data_path = session_data_path();
    
    if !key_path.exists() || !data_path.exists() {
        return None;
    }
    
    let key_bytes = fs::read(&key_path).ok()?;
    if key_bytes.len() != 32 {
        return None;
    }
    
    // Check timeout via mtime
    let metadata = fs::metadata(&key_path).ok()?;
    let modified = metadata.modified().ok()?;
    let elapsed = modified.elapsed().ok()?;
    if elapsed.as_secs() > timeout_secs {
        // Expired - clean up
        let _ = fs::remove_file(&key_path);
        let _ = fs::remove_file(&data_path);
        return None;
    }
    
    let data = fs::read(&data_path).ok()?;
    Some(data)
}

/// Save the session: encrypt encryption_key with a random session_key.
pub fn save_session(encryption_key_bytes: &[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    use rand::RngCore;
    
    let dir = session_dir();
    fs::create_dir_all(&dir)?;
    
    let mut session_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut session_key);
    
    // Encrypt encryption_key with session_key using AES-256-GCM
    let nonce = {
        let mut n = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut n);
        n
    };
    
    let key = EncryptionKey::new(session_key);
    let ciphertext = wpw_core::crypto::encrypt(&key, &nonce, encryption_key_bytes, &[])
        .map_err(|e| format!("Session encryption failed: {e}"))?;
    
    // Save session_key
    let key_path = session_key_path();
    fs::write(&key_path, &session_key)?;
    
    // Save session data (nonce + ciphertext)
    let data_path = session_data_path();
    let mut session_data = Vec::with_capacity(12 + ciphertext.len());
    session_data.extend_from_slice(&nonce);
    session_data.extend_from_slice(&ciphertext);
    fs::write(&data_path, &session_data)?;
    
    Ok(())
}

/// Load session and decrypt to get the encryption_key bytes.
pub fn get_encryption_key(timeout_secs: u64) -> Option<[u8; 32]> {
    let data = load_session(timeout_secs)?;
    let key_path = session_key_path();
    let key_bytes = fs::read(&key_path).ok()?;
    
    if key_bytes.len() != 32 || data.len() < 12 + 16 {
        return None;
    }
    
    let mut session_key_bytes = [0u8; 32];
    session_key_bytes.copy_from_slice(&key_bytes);
    
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[..12]);
    let ciphertext = &data[12..];
    
    let session_key = EncryptionKey::new(session_key_bytes);
    let plaintext = wpw_core::crypto::decrypt(&session_key, &nonce, ciphertext, &[]).ok()?;
    
    if plaintext.len() != 32 {
        return None;
    }
    
    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&plaintext);
    
    Some(enc_key)
}

/// Delete session files (lock).
pub fn clear_session() {
    let _ = fs::remove_file(session_key_path());
    let _ = fs::remove_file(session_data_path());
}
