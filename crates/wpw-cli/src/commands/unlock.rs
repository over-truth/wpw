use crate::{Cli, tty, session};
use std::path::PathBuf;

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

pub fn run(cli: &Cli, timeout: u64, vault_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = vault_override
        .map(PathBuf::from)
        .or_else(|| cli.vault.as_ref().map(PathBuf::from))
        .unwrap_or_else(default_vault_path);
    
    let password = tty::prompt_password("Enter master password: ")?;
    
    let vault_data = wpw_core::vault::open_vault(&vault_path, password.as_bytes())?;
    
    // Re-derive keys to store in session
    use wpw_core::crypto::kdf;
    let file_bytes = std::fs::read(&vault_path)?;
    let header = wpw_core::vault::header::VaultHeader::parse(&file_bytes)?;
    let params = kdf::KdfParams {
        m_cost: header.m_cost(),
        t_cost: header.t_cost as u32,
        p_cost: header.p_cost as u32,
    };
    let derived = kdf::derive_keys(password.as_bytes(), &header.salt, &params)
        .map_err(|e| format!("KDF failed: {e}"))?;
    
    session::save_session(derived.encryption_key.expose_key())?;
    
    if !cli.quiet {
        println!("Vault unlocked. {} entries loaded. Session timeout: {}s", vault_data.entries.len(), timeout);
    }
    
    Ok(())
}
