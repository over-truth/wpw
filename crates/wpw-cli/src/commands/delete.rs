use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, id: &str, yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !yes {
        let confirmed = tty::confirm(&format!("Delete entry '{}'?", id))?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let mut vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let removed = vault_data.remove_entry(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    wpw_core::vault::save_vault(&vault_path, master_password.as_bytes(), &vault_data)?;
    
    if !cli.quiet {
        println!("Entry '{}' deleted.", removed.title);
    }
    
    Ok(())
}
