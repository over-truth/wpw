use crate::{tty, vault_io, Cli};

pub fn run(cli: &Cli, format: &str, output: Option<&str>, include_history: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !include_history {
        eprintln!("Warning: Export includes plaintext passwords. Use --include-history to also export password history.");
    }
    if !tty::confirm("Export vault to plaintext? This is a security risk.")? {
        println!("Cancelled.");
        return Ok(());
    }

    let vault = vault_io::open_with_session(cli)?;

    let output_str = match format {
        "json" => {
            let entries: Vec<serde_json::Value> = vault.data.entries.iter().map(|e| {
                let mut obj = serde_json::json!({
                    "id": e.id,
                    "title": e.title,
                    "url": e.url,
                    "username": e.username,
                    "password": e.password,
                    "notes": e.notes,
                    "tags": e.tags,
                });
                if include_history {
                    obj["password_history"] = serde_json::json!(e.password_history);
                }
                obj
            }).collect();
            serde_json::to_string_pretty(&entries)?
        }
        _ => return Err(format!("Export format '{}' not yet supported", format).into()),
    };

    if let Some(path) = output {
        std::fs::write(path, &output_str)?;
        println!("Exported to {}", path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}
