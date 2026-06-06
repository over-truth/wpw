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
            "locked": session.locked,
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

/// Extract domain from URL, handling various formats.
/// Returns the eTLD+1 domain (e.g., "github.com" from "https://login.github.com/path")
fn extract_domain(url: &str) -> Option<String> {
    // Remove protocol
    let without_protocol = if url.contains("://") {
        url.split("://").nth(1)?
    } else {
        url
    };
    
    // Remove path, query, fragment
    let host = without_protocol.split('/').next()?;
    
    // Remove port
    let host = host.split(':').next()?;
    
    // Remove userinfo
    let host = if host.contains('@') {
        host.split('@').next_back()?
    } else {
        host
    };
    
    if host.is_empty() {
        return None;
    }
    
    // For localhost and IP addresses, return as-is
    if host == "localhost" || host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Some(host.to_lowercase());
    }
    
    // Extract eTLD+1 (last two parts of domain)
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        // Take last two parts (e.g., "github.com")
        let etld1 = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        Some(etld1.to_lowercase())
    } else {
        // Single-part domain (unlikely but handle)
        Some(host.to_lowercase())
    }
}

/// Check if two URLs match based on domain comparison.
/// Supports subdomain matching: "login.github.com" matches "github.com"
fn urls_match(page_url: &str, entry_url: &str) -> bool {
    let page_domain = match extract_domain(page_url) {
        Some(d) => d,
        None => return false,
    };
    
    let entry_domain = match extract_domain(entry_url) {
        Some(d) => d,
        None => return false,
    };
    
    // Exact domain match
    if page_domain == entry_domain {
        return true;
    }
    
    // Subdomain match: page domain ends with entry domain
    // e.g., "login.github.com" ends with ".github.com"
    page_domain.ends_with(&format!(".{}", entry_domain))
}

fn handle_query(request: &Message) -> Message {
    let url = match request.payload.as_ref().and_then(|p| p["url"].as_str()) {
        Some(u) => u,
        None => return Message::error(request.id.clone(), "internal_error", "Missing URL"),
    };
    
    SESSION.with(|s| {
        let session = s.borrow();
        if session.locked {
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
        if session.locked {
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
        if session.locked {
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
        if session.locked {
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
        if session.locked {
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
