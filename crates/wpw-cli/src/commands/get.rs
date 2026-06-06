use crate::{Cli, session, tty, clipboard};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(cli: &Cli, id: &str, field: Option<&str>, copy: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let _enc_key = session::get_encryption_key(300)
        .ok_or("Vault is locked. Run `wpw unlock` first.")?;
    let master_password = tty::prompt_password("Enter master password: ")?;
    let vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let entry = vault_data.find_entry(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    if let Some(field_name) = field {
        let value = match field_name {
            "username" => entry.username.as_deref().unwrap_or(""),
            "password" => entry.password.as_deref().unwrap_or(""),
            "url" => entry.url.as_deref().unwrap_or(""),
            "notes" => entry.notes.as_deref().unwrap_or(""),
            "totp" => entry.totp_secret.as_deref().unwrap_or(""),
            _ => return Err(format!("Unknown field: {}", field_name).into()),
        };
        if copy && field_name == "password" {
            clipboard::copy_and_clear(value, "Password")?;
        } else if copy {
            clipboard::copy_and_clear(value, field_name)?;
        } else if show || field_name != "password" {
            println!("{}", value);
        } else {
            println!("(use --show to display or --copy to copy)");
        }
    } else {
        // Show all fields except password
        println!("ID:       {}", entry.id);
        println!("Title:    {}", entry.title);
        if let Some(ref url) = entry.url { println!("URL:      {}", url); }
        if let Some(ref username) = entry.username { println!("Username: {}", username); }
        if entry.password.is_some() {
            if show {
                println!("Password: {}", entry.password.as_ref().unwrap());
            } else {
                println!("Password: (use --show or --copy)");
            }
        }
        if let Some(ref notes) = entry.notes { println!("Notes:    {}", notes); }
        if !entry.tags.is_empty() { println!("Tags:     {}", entry.tags.join(", ")); }
    }
    
    Ok(())
}
