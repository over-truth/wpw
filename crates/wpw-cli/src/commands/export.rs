use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, format: &str, output: Option<&str>, include_history: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !include_history {
        eprintln!("Warning: Export includes plaintext passwords. Use --include-history to also export password history.");
    }
    let confirmed = tty::confirm("Export vault to plaintext? This is a security risk.")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }
    
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let output_str = match format {
        "json" => {
            let entries: Vec<serde_json::Value> = vault_data.entries.iter().map(|e| {
                let mut obj = serde_json::json!({
                    "id": e.id,
                    "title": e.title,
                    "url": e.url,
                    "username": e.username,
                    "password": e.password,
                    "notes": e.notes,
                    "tags": e.tags,
                });
                if include_history {
                    obj["password_history"] = serde_json::json!(e.password_history);
                }
                obj
            }).collect();
            serde_json::to_string_pretty(&entries)?
        }
        _ => return Err(format!("Export format '{}' not yet supported", format).into()),
    };
    
    if let Some(path) = output {
        std::fs::write(path, &output_str)?;
        println!("Exported to {}", path);
    } else {
        println!("{}", output_str);
    }
    
    Ok(())
}
