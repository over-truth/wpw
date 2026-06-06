use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let entry = vault_data.find_entry(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    if entry.password_history.is_empty() {
        println!("No password history for '{}'.", entry.title);
    } else {
        println!("Password history for '{}':", entry.title);
        for (i, h) in entry.password_history.iter().enumerate() {
            let dt = time::OffsetDateTime::from_unix_timestamp(h.changed_at)
                .map(|d| format!("{}", d.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()))
                .unwrap_or_else(|_| h.changed_at.to_string());
            println!("  {}. [{}] {}", i + 1, dt, h.password);
        }
    }
    
    Ok(())
}
