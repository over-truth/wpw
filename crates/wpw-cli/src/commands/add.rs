use crate::{Cli, session, tty, clipboard};
use std::path::PathBuf;

fn load_vault(cli: &Cli) -> Result<(PathBuf, Vec<u8>, wpw_core::vault::format::VaultData), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
        #[cfg(not(target_os = "windows"))]
        { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
    });
    let enc_key = session::get_encryption_key(300)
        .ok_or("Vault is locked. Run `wpw unlock` first.")?;
    let password = b"session"; // dummy - we use session key
    let data = wpw_core::vault::open_vault(&vault_path, password)
        .or_else(|_| {
            // Try with session key approach: we need to re-open with actual password
            // For simplicity in session mode, we'll need to store the password or use a different approach
            Err::<_, Box<dyn std::error::Error>>("Failed to open vault".into())
        })?;
    Ok((vault_path, enc_key.to_vec(), data))
}

pub fn run(
    cli: &Cli,
    title: Option<String>,
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    generate: bool,
    notes: Option<String>,
    tags: Vec<String>,
    totp: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = cli.vault.as_ref().map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join("Documents").join("wpw").join("vault.wpw") }
        #[cfg(not(target_os = "windows"))]
        { let h = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")); h.join(".local").join("share").join("wpw").join("vault.wpw") }
    });
    
    let enc_key_bytes = session::get_encryption_key(300)
        .ok_or("Vault is locked. Run `wpw unlock` first.")?;
    
    let title = title.ok_or("Title is required (--title)")?;
    
    let password_val = if generate {
        let opts = wpw_core::generator::PasswordOptions::default();
        Some(wpw_core::generator::generate_password(&opts))
    } else {
        password
    };
    
    // Read and decrypt the vault to get salt/nonce
    let file_bytes = std::fs::read(&vault_path)?;
    let header = wpw_core::vault::header::VaultHeader::parse(&file_bytes)?;
    
    // We need the master password to save - this is a design limitation.
    // For the CLI, we'll ask for the password again on save.
    // Alternative: store encrypted master password in session.
    let master_password = tty::prompt_password("Enter master password to save: ")?;
    
    let mut vault_data = wpw_core::vault::open_vault(&vault_path, master_password.as_bytes())?;
    
    let mut entry = wpw_core::vault::entry::Entry::new(title);
    entry.url = url;
    entry.username = username;
    entry.password = password_val;
    entry.notes = notes;
    entry.tags = tags;
    entry.totp_secret = totp;
    
    vault_data.add_entry(entry);
    
    wpw_core::vault::save_vault(&vault_path, master_password.as_bytes(), &vault_data)?;
    
    if !cli.quiet {
        println!("Entry '{}' added.", vault_data.entries.last().unwrap().title);
    }
    
    Ok(())
}
