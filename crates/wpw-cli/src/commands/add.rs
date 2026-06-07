use crate::{vault_io, Cli};

pub fn run(
    cli: &Cli,
    title: Option<String>,
    url: Option<String>,
    username: Option<String>,
    password_stdin: bool,
    generate: bool,
    notes: Option<String>,
    tags: Vec<String>,
    totp: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let title = title.ok_or("Title is required (--title)")?;

    let password_val = if generate {
        let opts = wpw_core::generator::PasswordOptions::default();
        Some(wpw_core::generator::generate_password(&opts))
    } else if password_stdin {
        Some(crate::tty::read_password_stdin()?)
    } else {
        let pw = crate::tty::prompt_password("Entry password (leave blank to skip): ")?;
        if pw.is_empty() { None } else { Some(pw) }
    };

    let mut vault = vault_io::open_with_session(cli)?;

    let mut entry = wpw_core::vault::entry::Entry::new(title);
    entry.url = url;
    entry.username = username;
    entry.password = password_val;
    entry.notes = notes;
    entry.tags = tags;
    entry.totp_secret = totp;

    vault.data.add_entry(entry);
    vault.save()?;

    if !cli.quiet {
        println!("Entry '{}' added.", vault.data.entries.last().unwrap().title);
    }

    Ok(())
}
