use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, format: &str, file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file)?;
    
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let mut vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let initial_count = vault_data.entries.len();
    
    match format {
        "json" => {
            let imported: Vec<serde_json::Value> = serde_json::from_str(&content)?;
            for item in imported {
                let title = item["title"].as_str().unwrap_or("Untitled").to_string();
                let url = item["url"].as_str().map(|s| s.to_string());
                let username = item["username"].as_str().map(|s| s.to_string());
                let password = item["password"].as_str().map(|s| s.to_string());
                let notes = item["notes"].as_str().map(|s| s.to_string());
                
                // Check for duplicates
                let is_dup = vault_data.entries.iter().any(|e| {
                    e.url == url && e.username == username
                });
                
                if is_dup {
                    println!("Skipping duplicate: {} ({}@{})", title, username.as_deref().unwrap_or(""), url.as_deref().unwrap_or(""));
                    continue;
                }
                
                let mut entry = wpw_core::vault::entry::Entry::new(title);
                entry.url = url;
                entry.username = username;
                entry.password = password;
                entry.notes = notes;
                
                vault_data.add_entry(entry);
            }
        }
        _ => return Err(format!("Import format '{}' not yet supported", format).into()),
    }
    
    let imported_count = vault_data.entries.len() - initial_count;
    wpw_core::vault::save_vault(&vault_path, master_password.as_bytes(), &vault_data)?;
    
    if !cli.quiet {
        println!("Imported {} entries.", imported_count);
    }
    
    Ok(())
}
