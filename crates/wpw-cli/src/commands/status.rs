use crate::{Cli, session};
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

pub fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let exists = vault_path.exists();
    let is_unlocked = session::get_encryption_key(300).is_some();
    
    if cli.json {
        let status = serde_json::json!({
            "vault_path": vault_path.display().to_string(),
            "vault_exists": exists,
            "locked": !is_unlocked,
        });
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Vault: {}", vault_path.display());
        println!("Exists: {}", if exists { "yes" } else { "no" });
        println!("Status: {}", if is_unlocked { "unlocked" } else { "locked" });
    }
    
    Ok(())
}
