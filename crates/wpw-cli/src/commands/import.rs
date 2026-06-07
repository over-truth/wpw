use crate::{vault_io, Cli};

pub fn run(cli: &Cli, format: &str, file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file)?;

    let mut vault = vault_io::open_with_session(cli)?;
    let initial_count = vault.data.entries.len();

    match format {
        "json" => {
            let imported: Vec<serde_json::Value> = serde_json::from_str(&content)?;
            for item in imported {
                let title = item["title"].as_str().unwrap_or("Untitled").to_string();
                let url = item["url"].as_str().map(|s| s.to_string());
                let username = item["username"].as_str().map(|s| s.to_string());
                let password = item["password"].as_str().map(|s| s.to_string());
                let notes = item["notes"].as_str().map(|s| s.to_string());

                let is_dup = vault.data.entries.iter().any(|e| {
                    e.url == url && e.username == username
                });

                if is_dup {
                    println!("Skipping duplicate: {} ({}@{})", title, username.as_deref().unwrap_or(""), url.as_deref().unwrap_or(""));
                    continue;
                }

                let mut entry = wpw_core::vault::entry::Entry::new(title);
                entry.url = url;
                entry.username = username;
                entry.password = password;
                entry.notes = notes;

                vault.data.add_entry(entry);
            }
        }
        _ => return Err(format!("Import format '{}' not yet supported", format).into()),
    }

    let imported_count = vault.data.entries.len() - initial_count;
    vault.save()?;

    if !cli.quiet {
        println!("Imported {} entries.", imported_count);
    }

    Ok(())
}
