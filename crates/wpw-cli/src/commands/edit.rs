use crate::{vault_io, Cli};

pub fn run(
    cli: &Cli,
    id: &str,
    title: Option<String>,
    url: Option<String>,
    username: Option<String>,
    password_stdin: bool,
    generate: bool,
    notes: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = vault_io::open_with_session(cli)?;

    let entry = vault.data.find_entry_mut(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    if let Some(t) = title { entry.title = t; }
    if url.is_some() { entry.url = url; }
    if username.is_some() { entry.username = username; }

    if generate {
        entry.push_password_history();
        let opts = wpw_core::generator::PasswordOptions::default();
        entry.password = Some(wpw_core::generator::generate_password(&opts));
    } else if password_stdin {
        entry.push_password_history();
        entry.password = Some(crate::tty::read_password_stdin()?);
    }

    if notes.is_some() { entry.notes = notes; }
    entry.modified_at = time::OffsetDateTime::now_utc().unix_timestamp();
    let entry_title = entry.title.clone();

    vault.save()?;

    if !cli.quiet {
        println!("Entry '{}' updated.", entry_title);
    }

    Ok(())
}
