use crate::{session, vault_io, Cli};

pub fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = vault_io::vault_path(cli);
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
