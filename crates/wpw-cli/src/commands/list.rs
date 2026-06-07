use crate::{vault_io, Cli};

pub fn run(cli: &Cli, tag: Option<&str>, url: Option<&str>, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vault = vault_io::open_with_session(cli)?;

    let entries: Vec<_> = vault.data.entries.iter().filter(|e| {
        if let Some(t) = tag {
            if !e.tags.contains(&t.to_string()) { return false; }
        }
        if let Some(u) = url {
            match &e.url {
                Some(entry_url) if entry_url.contains(u) => {}
                _ => return false,
            }
        }
        true
    }).collect();

    match format {
        "json" => {
            let json_entries: Vec<serde_json::Value> = entries.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "title": e.title,
                    "url": e.url,
                    "username": e.username,
                    "tags": e.tags,
                })
            }).collect();
            println!("{}", serde_json::to_string_pretty(&json_entries)?);
        }
        "csv" => {
            println!("id,title,url,username");
            for e in &entries {
                println!("{},{},{},{}", e.id, e.title, e.url.as_deref().unwrap_or(""), e.username.as_deref().unwrap_or(""));
            }
        }
        _ => {
            if entries.is_empty() {
                println!("No entries found.");
            } else {
                println!("{:<38} {:<25} {:<40} {:<20}", "ID", "TITLE", "URL", "USERNAME");
                println!("{}", "-".repeat(125));
                for e in &entries {
                    let short_id = if e.id.len() > 36 { &e.id[..36] } else { &e.id };
                    println!("{:<38} {:<25} {:<40} {:<20}",
                        short_id,
                        truncate(&e.title, 25),
                        e.url.as_deref().map(|u| truncate(u, 40)).unwrap_or_default(),
                        e.username.as_deref().map(|u| truncate(u, 20)).unwrap_or_default(),
                    );
                }
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max-1]) }
}
