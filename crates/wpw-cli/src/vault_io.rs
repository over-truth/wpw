use std::path::{Path, PathBuf};
use wpw_core::crypto::EncryptionKey;
use wpw_core::vault::format::VaultData;

use crate::{paths, session, tty, Cli};

/// One in-memory snapshot of the vault plus the credential needed to write it back.
///
/// We allow the credential to be either the cached session key (preferred, ≈0 cost) or
/// the master password (≈1s Argon2id). Before this struct existed, every command derived
/// the key from the master password on every invocation, which meant `wpw unlock` was
/// useless on the CLI side: every subsequent command still prompted.
pub struct UnlockedVault {
    pub path: PathBuf,
    pub data: VaultData,
    credential: Credential,
}

enum Credential {
    SessionKey(EncryptionKey),
    MasterPassword(String),
}

impl UnlockedVault {
    /// Save the vault back using whichever credential we opened it with.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.credential {
            Credential::SessionKey(k) => {
                wpw_core::vault::save_vault_with_key(&self.path, k, &self.data)?;
            }
            Credential::MasterPassword(pw) => {
                wpw_core::vault::save_vault(&self.path, pw.as_bytes(), &self.data)?;
            }
        }
        Ok(())
    }
}

/// Open the vault, preferring the cached session key. Only prompts for the master
/// password if no session is active or the cached key fails to decrypt (vault file
/// changed since unlock — e.g., re-keyed externally).
pub fn open_with_session(cli: &Cli) -> Result<UnlockedVault, Box<dyn std::error::Error>> {
    open_with_session_timeout(cli, 300)
}

pub fn open_with_session_timeout(cli: &Cli, timeout: u64) -> Result<UnlockedVault, Box<dyn std::error::Error>> {
    let path = paths::resolve_vault_path(cli.vault.as_deref());

    if let Some(key_bytes) = session::get_encryption_key(timeout) {
        let key = EncryptionKey::new(key_bytes);
        match wpw_core::vault::open_vault_with_key(&path, &key) {
            Ok(data) => {
                return Ok(UnlockedVault {
                    path,
                    data,
                    credential: Credential::SessionKey(key),
                });
            }
            Err(wpw_core::vault::VaultError::DecryptionFailed) => {
                // Cached key no longer matches the file (file was overwritten / re-keyed).
                // Fall through to master-password prompt.
            }
            Err(e) => return Err(e.into()),
        }
    }

    let pw = tty::prompt_password("Enter master password: ")?;
    let data = wpw_core::vault::open_vault(&path, pw.as_bytes())?;
    Ok(UnlockedVault {
        path,
        data,
        credential: Credential::MasterPassword(pw),
    })
}

/// Just the path, without opening the vault. Useful for `status` / `init`.
pub fn vault_path(cli: &Cli) -> PathBuf {
    paths::resolve_vault_path(cli.vault.as_deref())
}

#[allow(dead_code)]
pub fn vault_path_with_override(cli: &Cli, override_path: Option<&str>) -> PathBuf {
    if let Some(p) = override_path {
        PathBuf::from(p)
    } else {
        paths::resolve_vault_path(cli.vault.as_deref())
    }
}

/// Used by `init` where we deliberately don't want session/path fallback semantics.
#[allow(dead_code)]
pub fn raw_path(p: &Path) -> PathBuf {
    p.to_path_buf()
}
