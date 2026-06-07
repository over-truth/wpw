use crate::{vault_io, Cli};

pub fn run(cli: &Cli, id: &str, at: i64) -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = vault_io::open_with_session(cli)?;

    let entry = vault.data.find_entry_mut(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    let restored = entry.password_history.iter()
        .find(|h| h.changed_at == at)
        .ok_or("No password found at that timestamp")?
        .password
        .clone();

    entry.push_password_history();
    entry.password = Some(restored);
    entry.modified_at = time::OffsetDateTime::now_utc().unix_timestamp();
    let entry_title = entry.title.clone();

    vault.save()?;

    if !cli.quiet {
        println!("Password restored for '{}'.", entry_title);
    }

    Ok(())
}
