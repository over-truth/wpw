use crate::{paths, session, tty, Cli};
use std::path::PathBuf;

pub fn run(cli: &Cli, timeout: u64, vault_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = vault_override
        .map(PathBuf::from)
        .or_else(|| cli.vault.as_ref().map(PathBuf::from))
        .unwrap_or_else(paths::default_vault_path);

    let password = tty::prompt_password("Enter master password: ")?;

    // Derive the encryption key once, then verify the password by attempting decryption.
    let file_bytes = std::fs::read(&vault_path)?;
    let header = wpw_core::vault::header::VaultHeader::parse(&file_bytes)?;
    let params = wpw_core::crypto::kdf::KdfParams {
        m_cost: header.m_cost(),
        t_cost: header.t_cost as u32,
        p_cost: header.p_cost as u32,
    };
    let derived = wpw_core::crypto::kdf::derive_keys(password.as_bytes(), &header.salt, &params)
        .map_err(|e| format!("KDF failed: {e}"))?;

    // Verify before persisting the session — open_vault_with_key would do this implicitly
    // but failing here keeps the session file untouched on wrong-password input.
    let key = wpw_core::crypto::EncryptionKey::new(*derived.encryption_key.expose_key());
    let vault_data = wpw_core::vault::open_vault_with_key(&vault_path, &key)?;

    session::save_session(derived.encryption_key.expose_key())?;

    if !cli.quiet {
        println!("Vault unlocked. {} entries loaded. Session timeout: {}s",
            vault_data.entries.len(), timeout);
    }

    Ok(())
}
