use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,  // UUID v4
    pub created_at: i64,   // Unix timestamp seconds
    pub modified_at: i64,

    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,

    pub totp_secret: Option<String>,  // Base32 encoded
    pub totp_issuer: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub custom_fields: Vec<CustomField>,
    pub password_history: Vec<PasswordHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub label: String,
    pub value: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistoryEntry {
    pub password: String,
    pub changed_at: i64,
}

impl Entry {
    pub fn new(title: String) -> Self {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            modified_at: now,
            title,
            url: None,
            username: None,
            password: None,
            totp_secret: None,
            totp_issuer: None,
            notes: None,
            tags: Vec::new(),
            custom_fields: Vec::new(),
            password_history: Vec::new(),
        }
    }

    /// Push the current password to history (call before changing password).
    /// Keeps at most 10 recent history entries.
    pub fn push_password_history(&mut self) {
        if let Some(ref pw) = self.password {
            let entry = PasswordHistoryEntry {
                password: pw.clone(),
                changed_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            };
            self.password_history.push(entry);
            // Keep only the last 10 entries
            if self.password_history.len() > 10 {
                self.password_history.remove(0);
            }
        }
    }
}
