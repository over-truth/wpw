use crate::{Cli, session, tty};
use std::path::PathBuf;

fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
    #[cfg(not(target_os = "windows"))]
    { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
}

pub fn run(
    cli: &Cli,
    id: &str,
    title: Option<String>,
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    generate: bool,
    notes: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(default_vault_path);
    let master_password = tty::prompt_password("Enter master password: ")?;
    let mut vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let entry = vault_data.find_entry_mut(id)
        .ok_or(format!("Entry '{}' not found", id))?;
    
    if let Some(t) = title { entry.title = t; }
    if url.is_some() { entry.url = url; }
    if username.is_some() { entry.username = username; }
    
    if generate {
        entry.push_password_history();
        let opts = wpw_core::generator::PasswordOptions::default();
        entry.password = Some(wpw_core::generator::generate_password(&opts));
    } else if password.is_some() {
        entry.push_password_history();
        entry.password = password;
    }
    
    if notes.is_some() { entry.notes = notes; }
    entry.modified_at = time::OffsetDateTime::now_utc().unix_timestamp();
    let entry_title = entry.title.clone();
    
    wpw_core::vault::save_vault(&vault_path, master_password.as_bytes(), &vault_data)?;
    
    if !cli.quiet {
        println!("Entry '{}' updated.", entry_title);
    }
    
    Ok(())
}
