use crate::protocol::Message;
use crate::session::HostSession;
use std::cell::RefCell;

// Host session state (single-threaded, stdin/stdout based)
thread_local! {
    static SESSION: RefCell<HostSession> = RefCell::new(HostSession::new());
}

pub fn handle_request(request: &Message) -> Message {
    match request.msg_type.as_str() {
        "status" => handle_status(request),
        "unlock" => handle_unlock(request),
        "lock" => handle_lock(request),
        "query" => handle_query(request),
        "get_entry" => handle_get_entry(request),
        "get_totp" => handle_get_totp(request),
        "add_entry" => handle_add_entry(request),
        "delete_entry" => handle_delete_entry(request),
        _ => Message::error(request.id.clone(), "internal_error", "Unknown message type"),
    }
}

fn handle_status(request: &Message) -> Message {
    SESSION.with(|s| {
        let session = s.borrow();
        let vault_exists = session.vault_path.exists();
        Message::success(request.id.clone(), serde_json::json!({
            "locked": session.locked(),
            "vault_exists": vault_exists,
        }))
    })
}

fn handle_unlock(request: &Message) -> Message {
    let password = match request.payload.as_ref().and_then(|p| p["master_password"].as_str()) {
        Some(pw) => pw,
        None => return Message::error(request.id.clone(), "internal_error", "Missing password"),
    };
    
    SESSION.with(|s| {
        let mut session = s.borrow_mut();
        match session.unlock(password) {
            Ok(()) => Message::success(request.id.clone(), serde_json::json!({ "unlocked": true })),
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("decryption failed") || err_msg.contains("wrong password") {
                    Message::error(request.id.clone(), "wrong_password", "Incorrect password")
                } else if err_msg.contains("not found") {
                    Message::error(request.id.clone(), "vault_not_found", "Vault file not found")
                } else {
                    Message::error(request.id.clone(), "internal_error", "Failed to unlock vault")
                }
            }
        }
    })
}

fn handle_lock(request: &Message) -> Message {
    SESSION.with(|s| {
        let mut session = s.borrow_mut();
        session.lock();
        Message::success(request.id.clone(), serde_json::json!({ "locked": true }))
    })
}

/// Extract the host (no path, no port, no userinfo) from a URL string.
fn extract_host(url: &str) -> Option<String> {
    let without_protocol = if url.contains("://") {
        url.split("://").nth(1)?
    } else {
        url
    };

    let host = without_protocol.split('/').next()?;
    let host = host.split(':').next()?;
    let host = if host.contains('@') {
        host.split('@').next_back()?
    } else {
        host
    };

    if host.is_empty() { None } else { Some(host.to_lowercase()) }
}

/// Registrable domain for a host, e.g. `login.github.com` → `github.com`,
/// `foo.example.co.uk` → `example.co.uk`. Uses the Public Suffix List so we don't get
/// fooled by multi-part TLDs.
///
/// `localhost` and bare IPs (v4 or v6 text form) are returned as-is — there's no PSL
/// concept for them, and treating them as multi-label hosts would either over-match
/// (`127.0.0.1` and `192.0.0.1` would both collapse to `0.1`) or under-match.
fn registrable_domain(host: &str) -> Option<String> {
    if host == "localhost" {
        return Some(host.to_string());
    }
    // ipv4 / ipv6 literals (the [::1] form has square brackets stripped by extract_host)
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Some(host.to_string());
    }
    let domain = psl::domain(host.as_bytes())?;
    std::str::from_utf8(domain.as_bytes()).ok().map(|s| s.to_string())
}

/// Public hook for tests: returns the registrable-domain comparator value for a URL.
fn url_registrable_domain(url: &str) -> Option<String> {
    let host = extract_host(url)?;
    registrable_domain(&host)
}

/// Check if two URLs should match for autofill.
///
/// Matches when their registrable domains are equal. The previous implementation took
/// `host.split('.').rev().take(2)` and assumed two labels are always enough — but for
/// any multi-part suffix like `.co.uk`, `.com.au`, `.github.io` that rule collapsed
/// *every* site under the suffix to a single shared key, which would cause autofill
/// to leak credentials across unrelated tenants.
fn urls_match(page_url: &str, entry_url: &str) -> bool {
    match (url_registrable_domain(page_url), url_registrable_domain(entry_url)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn handle_query(request: &Message) -> Message {
    let url = match request.payload.as_ref().and_then(|p| p["url"].as_str()) {
        Some(u) => u,
        None => return Message::error(request.id.clone(), "internal_error", "Missing URL"),
    };
    
    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked() {
            return Message::error(request.id.clone(), "vault_locked", "Vault is locked");
        }

        let vault_data = match session.decrypt_vault() {
            Ok(data) => data,
            Err(e) => return Message::error(request.id.clone(), "internal_error", &e.to_string()),
        };

        // Find entries matching the URL
        let matching: Vec<serde_json::Value> = vault_data.entries.iter()
            .filter(|entry| {
                entry.url.as_deref()
                    .map(|entry_url| urls_match(url, entry_url))
                    .unwrap_or(false)
            })
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "title": entry.title,
                    "username": entry.username,
                    "url": entry.url,
                })
            })
            .collect();
        
        Message::success(request.id.clone(), serde_json::json!({
            "entries": matching,
        }))
    })
}

fn handle_get_entry(request: &Message) -> Message {
    let entry_id = match request.payload.as_ref().and_then(|p| p["entry_id"].as_str()) {
        Some(id) => id,
        None => return Message::error(request.id.clone(), "internal_error", "Missing entry_id"),
    };
    
    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked() {
            return Message::error(request.id.clone(), "vault_locked", "Vault is locked");
        }

        let vault_data = match session.decrypt_vault() {
            Ok(data) => data,
            Err(e) => return Message::error(request.id.clone(), "internal_error", &e.to_string()),
        };

        let entry = match vault_data.entries.iter().find(|e| e.id == entry_id) {
            Some(e) => e,
            None => return Message::error(request.id.clone(), "entry_not_found", "Entry not found"),
        };

        Message::success(request.id.clone(), serde_json::json!({
            "id": entry.id,
            "title": entry.title,
            "username": entry.username,
            "password": entry.password,
            "url": entry.url,
            "totp_secret": entry.totp_secret,
            "totp_issuer": entry.totp_issuer,
            "notes": entry.notes,
            "tags": entry.tags,
            "custom_fields": entry.custom_fields,
        }))
    })
}

fn handle_get_totp(request: &Message) -> Message {
    let entry_id = match request.payload.as_ref().and_then(|p| p["entry_id"].as_str()) {
        Some(id) => id,
        None => return Message::error(request.id.clone(), "internal_error", "Missing entry_id"),
    };
    
    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked() {
            return Message::error(request.id.clone(), "vault_locked", "Vault is locked");
        }

        let vault_data = match session.decrypt_vault() {
            Ok(data) => data,
            Err(e) => return Message::error(request.id.clone(), "internal_error", &e.to_string()),
        };

        let entry = match vault_data.entries.iter().find(|e| e.id == entry_id) {
            Some(e) => e,
            None => return Message::error(request.id.clone(), "entry_not_found", "Entry not found"),
        };

        let totp_secret = match &entry.totp_secret {
            Some(s) => s,
            None => return Message::error(request.id.clone(), "totp_not_configured", "TOTP not configured for this entry"),
        };
        
        match wpw_core::totp::generate_totp(totp_secret, entry.totp_issuer.as_deref()) {
            Ok((code, remaining)) => {
                Message::success(request.id.clone(), serde_json::json!({
                    "code": code,
                    "remaining_seconds": remaining,
                }))
            }
            Err(e) => Message::error(request.id.clone(), "internal_error", &e.to_string()),
        }
    })
}

fn handle_add_entry(request: &Message) -> Message {
    let payload = match request.payload.as_ref() {
        Some(p) => p,
        None => return Message::error(request.id.clone(), "internal_error", "Missing payload"),
    };
    
    let title = match payload["title"].as_str() {
        Some(t) => t.to_string(),
        None => return Message::error(request.id.clone(), "internal_error", "Missing title"),
    };
    
    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked() {
            return Message::error(request.id.clone(), "vault_locked", "Vault is locked");
        }

        let mut vault_data = match session.decrypt_vault() {
            Ok(data) => data,
            Err(e) => return Message::error(request.id.clone(), "internal_error", &e.to_string()),
        };

        // Create new entry
        let mut entry = wpw_core::vault::entry::Entry::new(title);
        entry.url = payload["url"].as_str().map(|s| s.to_string());
        entry.username = payload["username"].as_str().map(|s| s.to_string());
        entry.password = payload["password"].as_str().map(|s| s.to_string());
        entry.notes = payload["notes"].as_str().map(|s| s.to_string());
        
        // Add tags if provided
        if let Some(tags) = payload["tags"].as_array() {
            entry.tags = tags.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect();
        }
        
        // Add TOTP if provided
        entry.totp_secret = payload["totp_secret"].as_str().map(|s| s.to_string());
        entry.totp_issuer = payload["totp_issuer"].as_str().map(|s| s.to_string());
        
        let entry_id = entry.id.clone();
        vault_data.add_entry(entry);
        
        match session.save_vault(&vault_data) {
            Ok(()) => {
                Message::success(request.id.clone(), serde_json::json!({
                    "id": entry_id,
                    "created": true,
                }))
            }
            Err(e) => Message::error(request.id.clone(), "internal_error", &e.to_string()),
        }
    })
}

fn handle_delete_entry(request: &Message) -> Message {
    let entry_id = match request.payload.as_ref().and_then(|p| p["entry_id"].as_str()) {
        Some(id) => id,
        None => return Message::error(request.id.clone(), "internal_error", "Missing entry_id"),
    };

    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked() {
            return Message::error(request.id.clone(), "vault_locked", "Vault is locked");
        }

        let mut vault_data = match session.decrypt_vault() {
            Ok(data) => data,
            Err(e) => return Message::error(request.id.clone(), "internal_error", &e.to_string()),
        };

        match vault_data.remove_entry(entry_id) {
            Some(_) => {
                match session.save_vault(&vault_data) {
                    Ok(()) => Message::success(request.id.clone(), serde_json::json!({
                        "deleted": true,
                    })),
                    Err(e) => Message::error(request.id.clone(), "internal_error", &e.to_string()),
                }
            }
            None => Message::error(request.id.clone(), "entry_not_found", "Entry not found"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrable_domain_handles_multipart_tld() {
        assert_eq!(url_registrable_domain("https://login.github.com/").as_deref(), Some("github.com"));
        assert_eq!(url_registrable_domain("https://foo.example.co.uk/path").as_deref(), Some("example.co.uk"));
        assert_eq!(url_registrable_domain("https://other.co.uk/").as_deref(), Some("other.co.uk"));
        assert_eq!(url_registrable_domain("https://localhost:3000/").as_deref(), Some("localhost"));
        assert_eq!(url_registrable_domain("http://127.0.0.1:8080/").as_deref(), Some("127.0.0.1"));
    }

    #[test]
    fn unrelated_co_uk_sites_do_not_match() {
        // The previous implementation matched these because both collapsed to `co.uk`.
        assert!(!urls_match("https://attacker.co.uk/", "https://victim.co.uk/login"));
    }

    #[test]
    fn subdomain_of_same_registrable_matches() {
        assert!(urls_match("https://login.github.com/", "https://github.com/"));
        assert!(urls_match("https://github.com/", "https://login.github.com/"));
    }
}
