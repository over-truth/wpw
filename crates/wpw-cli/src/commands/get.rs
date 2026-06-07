use crate::{clipboard, vault_io, Cli};

pub fn run(cli: &Cli, id: &str, field: Option<&str>, copy: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let vault = vault_io::open_with_session(cli)?;

    let entry = vault.data.find_entry(id)
        .ok_or_else(|| format!("Entry '{}' not found", id))?;

    if let Some(field_name) = field {
        let value = match field_name {
            "username" => entry.username.as_deref().unwrap_or(""),
            "password" => entry.password.as_deref().unwrap_or(""),
            "url" => entry.url.as_deref().unwrap_or(""),
            "notes" => entry.notes.as_deref().unwrap_or(""),
            "totp" => entry.totp_secret.as_deref().unwrap_or(""),
            _ => return Err(format!("Unknown field: {}", field_name).into()),
        };
        if copy {
            let label = if field_name == "password" { "Password" } else { field_name };
            clipboard::copy_and_clear(value, label)?;
        } else if show || field_name != "password" {
            println!("{}", value);
        } else {
            println!("(use --show to display or --copy to copy)");
        }
    } else {
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
