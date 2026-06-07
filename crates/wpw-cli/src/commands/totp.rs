use crate::{clipboard, vault_io, Cli};

pub fn run(cli: &Cli, id: &str, copy: bool) -> Result<(), Box<dyn std::error::Error>> {
    let vault = vault_io::open_with_session(cli)?;

    let entry = vault.data.find_entry(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    let secret = entry.totp_secret.as_ref()
        .ok_or("This entry does not have a TOTP secret configured")?;

    let (code, remaining) = wpw_core::totp::generate_totp(secret, entry.totp_issuer.as_deref())?;

    if copy {
        clipboard::copy_and_clear(&code, "TOTP code")?;
    }

    println!("TOTP: {} ({}s remaining)", code, remaining);

    Ok(())
}
