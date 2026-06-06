use crate::{Cli, tty};
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

pub fn run(cli: &Cli, vault_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = vault_override
        .map(PathBuf::from)
        .or_else(|| cli.vault.as_ref().map(PathBuf::from))
        .unwrap_or_else(default_vault_path);
    
    if vault_path.exists() {
        return Err(format!("Vault already exists at {}", vault_path.display()).into());
    }
    
    // Create parent directory
    if let Some(parent) = vault_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let password = tty::prompt_password_confirm()?;
    
    wpw_core::vault::create_vault(&vault_path, password.as_bytes())?;
    
    if !cli.quiet {
        println!("Vault initialized at {}", vault_path.display());
    }
    
    Ok(())
}
