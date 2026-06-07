use crate::commands::config as config_cmd;
use crate::{paths, tty, Cli};
use std::path::PathBuf;

pub fn run(cli: &Cli, vault_override: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = vault_override
        .map(PathBuf::from)
        .or_else(|| cli.vault.as_ref().map(PathBuf::from))
        .unwrap_or_else(paths::default_vault_path);

    if vault_path.exists() {
        return Err(format!("Vault already exists at {}", vault_path.display()).into());
    }

    if let Some(parent) = vault_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let password = tty::prompt_password_confirm()?;

    // Pick up any [kdf] overrides from config.toml so `wpw config set kdf.*` is no
    // longer a dead knob. Validation lives in wpw-core::create_vault_with_params so the
    // CLI and Native Host enforce the same floor.
    let (params, notes) = config_cmd::load_kdf_params();
    wpw_core::vault::create_vault_with_params(&vault_path, password.as_bytes(), &params)?;

    if !cli.quiet {
        println!("Vault initialized at {}", vault_path.display());
        if !notes.is_empty() {
            println!("KDF overrides applied from config: {}", notes.join(", "));
        }
    }

    Ok(())
}
