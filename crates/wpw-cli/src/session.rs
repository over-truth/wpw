use std::path::{Path, PathBuf};
use std::fs;
use wpw_core::crypto::EncryptionKey;

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

/// Ensure the session directory exists with restrictive permissions.
///
/// - **Linux/macOS**: `0700` (only the owning user can traverse).
/// - **Windows**: relies on `%LOCALAPPDATA%`'s default per-user ACL. Anyone running as
///   the same user can already read this regardless; cross-user isolation is what we
///   need here and that comes for free from LOCALAPPDATA. Per DESIGN §1.4, malware
///   running as the current user is explicitly out of scope.
fn ensure_session_dir() -> std::io::Result<PathBuf> {
    let dir = session_dir();
    fs::create_dir_all(&dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dir)?.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&dir, perms)?;
    }

    Ok(dir)
}

/// Write a sensitive byte string to `path` with 0600 perms (Linux/macOS).
///
/// We can't atomically create-with-mode in std on Linux, so this opens via
/// `OpenOptions::mode(0o600).create_new(true)` after first removing any old file. That
/// closes the brief window where a freshly created file inherits the umask-default 0644.
fn write_secret(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // Remove first so create_new(true) succeeds and we never run with the old (possibly
    // wider) mode that an earlier process may have left behind.
    let _ = fs::remove_file(path);

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    #[cfg(not(unix))]
    {
        // Windows: inherit the per-user LOCALAPPDATA ACL. See ensure_session_dir comment.
        fs::write(path, bytes)?;
    }

    Ok(())
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

    let metadata = fs::metadata(&key_path).ok()?;
    let modified = metadata.modified().ok()?;
    let elapsed = modified.elapsed().ok()?;
    if elapsed.as_secs() > timeout_secs {
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

    ensure_session_dir()?;

    let mut session_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut session_key);

    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);

    let key = EncryptionKey::new(session_key);
    let ciphertext = wpw_core::crypto::encrypt(&key, &nonce, encryption_key_bytes, &[])
        .map_err(|e| format!("Session encryption failed: {e}"))?;
    let session_key_bytes_for_disk = *key.expose_key();

    write_secret(&session_key_path(), &session_key_bytes_for_disk)?;

    let mut session_data = Vec::with_capacity(12 + ciphertext.len());
    session_data.extend_from_slice(&nonce);
    session_data.extend_from_slice(&ciphertext);
    write_secret(&session_data_path(), &session_data)?;

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
    // session_key drops here — bytes zeroized.

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
