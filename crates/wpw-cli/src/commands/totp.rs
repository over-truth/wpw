use crate::{Cli, session, tty, clipboard};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, id: &str, copy: bool) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let entry = vault_data.find_entry(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    let secret = entry.totp_secret.as_ref()
        .ok_or("This entry does not have a TOTP secret configured")?;
    
    let (code, remaining) = wpw_core::totp::generate_totp(secret, entry.totp_issuer.as_deref())?;
    
    if copy {
        clipboard::copy_and_clear(&code, "TOTP code")?;
    }
    
    println!("TOTP: {} ({}s remaining)", code, remaining);
    
    Ok(())
}
