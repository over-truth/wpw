# wpw — Personal Password Manager

**wpw** is a fully local, zero-trust password manager. Your vault lives on your own device — no cloud, no account, no subscription. Three components work together: a CLI for full control, a browser extension for autofill, and a native messaging bridge between them.

```
┌──────────────────────┐
│  Browser Extension   │  ← Chrome / Edge MV3, autofill + popup UI
│  (JS / Manifest V3)  │
└────────┬─────────────┘
         │ Native Messaging (stdin/stdout, 4-byte length-prefixed JSON)
┌────────▼─────────────┐
│  wpw-host            │  ← Bridge: validates extension origin,
│  (Rust, NM host)     │     proxies requests, shares session with CLI
└────────┬─────────────┘
         │
┌────────▼─────────────┐      ┌─────────────────────┐
│  wpw-cli             │  ←──→│  wpw-core            │
│  (Rust, full-featured│      │  (pure Rust, no I/O) │
│   terminal client)   │      │  • Argon2id KDF      │
└──────────────────────┘      │  • AES-256-GCM       │
                              │  • Vault format      │
                              │  • Password/passphrase gen │
                              │  • TOTP (RFC 6238)   │
                              └─────────────────────┘
```

## Features

- **Fully offline**: No servers, no sync service, no internet required. Vault is a single encrypted file you own.
- **Argon2id + AES-256-GCM**: Industry-standard KDF and cipher. Vault header includes salt, nonce, and KDF params as AAD.
- **Session caching**: Unlock once, use across CLI and browser without re-entering your master password (file-based session, configurable timeout, default 5 min).
- **TOTP support**: Generate time-based one-time codes from stored secrets alongside your passwords.
- **Password & passphrase generation**: Configurable character sets and EFF large wordlist passphrases.
- **Built-in password history**: Track recent passwords per entry (capped at 10), restore previous ones.
- **Browser extension**: Chrome/Edge MV3 extension with popup UI and on-demand autofill.
- **Atomic vault writes**: Backup → write `.tmp` → rename — your vault is never corrupted by an interrupted write.
- **Cross-platform**: Windows 10+ and Linux (glibc 2.28+). macOS support planned.
- **No root required**: Full user-space installation.

## Quick Start

### 1. Build

```bash
cargo build --release
```

Produces two binaries: `target/release/wpw` (CLI) and `target/release/wpw-host` (native host).

### 2. Install

**Linux:**
```bash
./install/install-linux.sh
# Or with browser extension support:
./install/install-linux.sh --extension-id YOUR_CHROME_EXTENSION_ID
```

**Windows:**
```powershell
.\install\install-windows.ps1
# Or with browser extension:
.\install\install-windows.ps1 -ExtensionId YOUR_CHROME_EXTENSION_ID
```

The installer copies binaries to `~/.local/bin` (Linux) or `%LOCALAPPDATA%\wpw` (Windows), registers the native messaging host for Chrome/Edge, and (optionally) configures extension origin verification.

### 3. Create your vault

```bash
wpw init
```

You'll be prompted for a master password. The vault file is created at `~/Documents/wpw/vault.wpw` (Windows) or `~/.local/share/wpw/vault.wpw` (Linux).

### 4. Add and retrieve entries

```bash
wpw add       # Interactive prompt for title, URL, username, password
wpw list      # List all entries
wpw get       # Retrieve an entry (copy password, username, etc.)
wpw generate  # Generate a random password or passphrase
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `init` | Create a new vault |
| `unlock` | Unlock vault and cache session key |
| `lock` | Clear cached session key |
| `status` | Show vault and session status |
| `add` | Add a new entry |
| `get` | Retrieve entries by field |
| `list` | List all entries |
| `edit` | Modify an entry |
| `delete` | Remove an entry |
| `generate` | Generate passwords or passphrases |
| `totp` | Generate a TOTP code for an entry |
| `history` | View or restore password history |
| `restore` | Restore vault from backup |
| `export` | Export vault (JSON) |
| `import` | Import entries from JSON |
| `config` | View or edit configuration |

Most commands accept `--vault <PATH>` to override the default vault location.

## Browser Extension

The [MV3 extension](extension/) provides a popup UI for unlocking, searching, and copying entries, plus a content script for autofill.

To use it:

1. Load the `extension/` directory as an unpacked extension in Chrome/Edge.
2. Note the extension ID from `chrome://extensions`.
3. Re-run the installer with that ID to register the native messaging host.
4. Click the extension icon to unlock and interact with your vault.

## Vault Format

A vault file consists of:

```
┌──────────────────────────────────┐
│ Magic bytes: WPW\0              │  (4 B)
├──────────────────────────────────┤
│ Header (unencrypted):           │  (79 B)
│  • KDF parameters (variant,     │
│    memory, iterations, lanes)   │
│  • Salt (32 B)                  │
│  • Nonce (12 B)                 │
├──────────────────────────────────┤
│ Encrypted payload:              │
│  • MessagePack-encoded entries  │
├──────────────────────────────────┤
│ GCM auth tag                    │  (16 B)
└──────────────────────────────────┘
```

Header fields are included as Additional Authenticated Data (AAD) — tampering with KDF params or salt invalidates the auth tag.

## Security

- **Argon2id**: 64 MiB memory, 3 iterations, 4 lanes — do not reduce without a security review.
- **Memory zeroing**: All key types implement `ZeroizeOnDrop` via the `zeroize` crate.
- **No secrets in logs**: Custom panic hooks prevent plaintext leakage in error output.
- **Extension ID verification**: Double-checked (CLI arg + compiled-in allowlist).
- **Clipboard clearing**: Extension clears copied passwords from clipboard after 30 seconds.
- **Session key encryption**: The vault encryption key is stored on disk encrypted with a random 32-byte session key.
- **Threat model**: Protects against vault file theft (stolen disk, compromised cloud sync). Does **not** protect against root/admin-level malware on the device.

## Configuration

Config file: `%APPDATA%\wpw\config.toml` (Windows) / `~/.config/wpw/config.toml` (Linux).

```toml
# Vault path (default varies by platform)
# vault = "..."

[defaults]
# Default generation settings
# password_length = 20
# passphrase_words = 6

[kdf]
# Argon2id parameters (defaults shown)
memory = 65536       # KiB (64 MiB)
iterations = 3
parallelism = 4
```

## Development

```bash
cargo test              # Run all tests
cargo test -p wpw-core  # Test specific crate
cargo clippy -- -D warnings  # Lint
cargo fmt --check       # Format check
```

See [DESIGN.md](DESIGN.md) for full architecture, protocol specification, and design rationale.

## License

MIT
