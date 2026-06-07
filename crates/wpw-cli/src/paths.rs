use std::path::PathBuf;

/// Default vault file location. The CLI used to duplicate this snippet inside every
/// command file; centralising it keeps wpw-cli and wpw-host in agreement and makes the
/// override knob (`--vault` / `WPW_VAULT_PATH`) easy to add later.
pub fn default_vault_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("Documents").join("wpw").join("vault.wpw")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".local").join("share").join("wpw").join("vault.wpw")
    }
}

pub fn resolve_vault_path(cli_override: Option<&str>) -> PathBuf {
    cli_override.map(PathBuf::from).unwrap_or_else(default_vault_path)
}
