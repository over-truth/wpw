use crate::{tty, vault_io, Cli};

pub fn run(cli: &Cli, id: &str, yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !yes && !tty::confirm(&format!("Delete entry '{}'?", id))? {
        println!("Cancelled.");
        return Ok(());
    }

    let mut vault = vault_io::open_with_session(cli)?;

    let removed = vault.data.remove_entry(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    vault.save()?;

    if !cli.quiet {
        println!("Entry '{}' deleted.", removed.title);
    }

    Ok(())
}
