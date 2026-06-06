use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, id: &str, at: i64) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let mut vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let entry = vault_data.find_entry_mut(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    let hist_idx = entry.password_history.iter().position(|h| h.changed_at == at)
        .ok_or("No password found at that timestamp")?;
    
    let hist_entry = entry.password_history.remove(hist_idx);
    
    // Push current password to history
    entry.push_password_history();
    entry.password = Some(hist_entry.password);
    entry.modified_at = time::OffsetDateTime::now_utc().unix_timestamp();
    let entry_title = entry.title.clone();
    
    wpw_core::vault::save_vault(&vault_path, master_password.as_bytes(), &vault_data)?;
    
    if !cli.quiet {
        println!("Password restored for '{}'.", entry_title);
    }
    
    Ok(())
}
