# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build --release          # Build all crates (release, LTO + size-optimized)
cargo build -p wpw-core        # Build individual crate
cargo build -p wpw-cli
cargo build -p wpw-host

cargo test                     # Run all tests
cargo test -p wpw-core         # Test specific crate
cargo test -p wpw-core -- kdf  # Run tests matching a name filter
cargo clippy -- -D warnings    # Lint
cargo fmt --check              # Format check
```

Installation (after build):
- Linux: `./install/install-linux.sh [--extension-id ID]`
- Windows: `./install/install-windows.ps1`

## Architecture

Three-component system with a shared cryptographic core:

```
Browser Extension (JS/MV3)
    ↕ Native Messaging (4-byte LE length + JSON)
wpw-host  ← bridges browser ↔ local system
wpw-cli   ← user-facing terminal interface
    ↕
wpw-core  ← pure crypto library (no I/O)
```

### wpw-core

No I/O; all functions are pure. Key modules:
- `crypto/`: Argon2id KDF, AES-256-GCM cipher, `ZeroizeOnDrop` key types
- `vault/`: File format parsing/serialization, `VaultData` + `Entry` structs
- `generator/`: Password and passphrase generation
- `totp/`: TOTP code calculation

**Vault file format**: 4-byte magic (`WPW\0`) + 79-byte unencrypted header (KDF params, nonce, salt) + AES-256-GCM encrypted MessagePack payload + 16-byte auth tag. The header fields are included as AAD in the auth tag.

**Entry model**: Each entry has a UUID v4 id, title, url, username, password, optional TOTP secret, custom fields map, and password history (capped at 10).

### wpw-cli

Commands: `init`, `unlock`, `lock`, `status`, `add`, `get`, `list`, `edit`, `delete`, `generate`, `totp`, `history`, `restore`, `export`, `import`, `config`.

**Session**: File-based (`session.key` + `session.dat`). A 32-byte random key encrypts the vault encryption key on disk. Timeout defaults to 300s, checked via file mtime. Vault path defaults to `%USERPROFILE%\Documents\wpw\vault.wpw` (Windows) / `~/.local/share/wpw/vault.wpw` (Linux), overridable via `--vault`. Session files stored at `%LOCALAPPDATA%\wpw\session\` (Windows) / `~/.local/share/wpw/session/` (Linux). Config file at `%APPDATA%\wpw\config.toml` (Windows) / `~/.config/wpw/config.toml` (Linux).

**Vault writes** are atomic: backup old file → write to `.tmp` → rename.

### wpw-host

Native Messaging host (Chrome/Edge protocol). Validates the connecting extension ID from command-line args and a built-in allowlist.

**Session state** is thread-local in-memory: locked flag + cached encryption key. On `unlock`, wpw-host reads the same `session.key`/`session.dat` files that wpw-cli writes — the two components share the CLI session. Supported message types: `status`, `unlock`, `lock`, `query`, `get_entry`, `get_totp`, `add_entry`, `delete_entry`.

**Security**: Custom panic hook prevents info leakage in panic messages.

### Browser Extension (MV3)

- **Service Worker** (`background/service-worker.js`): Maintains `connectNative` connection to `wpw-host`, routes requests with UUID correlation, 5s timeout per request, 30s heartbeat, badge management (`!` = locked, count = entries found).
- **Popup** (`popup/popup.html`): UI for unlock and entry selection.
- **Content Script** (`content/autofill.js`): Injected on demand to fill form fields; clears password reference after injection. Service worker clears clipboard after 30s.

## Key Design Constraints

- **Argon2id params**: 64 MiB memory, 3 iterations, 4 parallelism — don't reduce without security review.
- **All sensitive types** (`DerivedKey`, `EncryptionKey`, etc.) must implement `ZeroizeOnDrop` from the `secrecy`/`zeroize` crates.
- **No plaintext secrets in error messages, logs, or panic output** — enforced by the custom panic hook in wpw-host.
- **Extension ID verification** is double-checked (CLI arg + manifest allowlist); both must be present.
- MSRV is Rust 1.74.

## Key Crate Dependencies

| crate | purpose |
|-------|---------|
| `argon2` | Argon2id KDF |
| `aes-gcm` | AES-256-GCM encryption |
| `zeroize` / `secrecy` | Memory zeroing; wrap secrets in `SecretString`/`SecretBox` |
| `serde` + `rmp-serde` | MessagePack payload serialization |
| `clap` | CLI argument parsing (derive macros) |
| `totp-rs` | TOTP calculation (RFC 6238) |
| `arboard` | Cross-platform clipboard |
| `rpassword` | Hidden password input |
| `dirs` | XDG / Windows Known Folders paths |
| `anyhow` / `thiserror` | Error handling (anyhow in binaries, thiserror in wpw-core) |
