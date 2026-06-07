use crate::{clipboard, vault_io, Cli};

pub fn run(
    cli: &Cli,
    id: &str,
    show: bool,
    copy: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let vault = vault_io::open_with_session(cli)?;

    let entry = vault.data.find_entry(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    if entry.password_history.is_empty() {
        println!("No password history for '{}'.", entry.title);
        return Ok(());
    }

    if let Some(idx) = copy {
        let h = entry.password_history.get(idx.saturating_sub(1))
            .ok_or_else(|| format!("History index {} out of range (1..={})", idx, entry.password_history.len()))?;
        clipboard::copy_and_clear(&h.password, "Historical password")?;
        return Ok(());
    }

    println!("Password history for '{}':", entry.title);
    for (i, h) in entry.password_history.iter().enumerate() {
        let dt = time::OffsetDateTime::from_unix_timestamp(h.changed_at)
            .map(|d| format!("{}", d.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()))
            .unwrap_or_else(|_| h.changed_at.to_string());
        if show {
            println!("  {}. [{}] {}", i + 1, dt, h.password);
        } else {
            println!("  {}. [{}] (use --show to display, --copy <N> to copy)", i + 1, dt);
        }
    }

    Ok(())
}
