use serde::{Deserialize, Serialize};
use crate::vault::entry::Entry;

/// The decrypted payload structure, serialized as MessagePack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    /// Data model version for migrations
    pub version: u32,
    pub created_at: i64,
    pub modified_at: i64,
    pub entries: Vec<Entry>,
}

impl VaultData {
    pub fn new() -> Self {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Self {
            version: 1,
            created_at: now,
            modified_at: now,
            entries: Vec::new(),
        }
    }

    pub fn touch(&mut self) {
        self.modified_at = time::OffsetDateTime::now_utc().unix_timestamp();
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.touch();
        self.entries.push(entry);
    }

    pub fn find_entry(&self, id_or_title: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id_or_title || e.title == id_or_title)
    }

    pub fn find_entry_mut(&mut self, id_or_title: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id_or_title || e.title == id_or_title)
    }

    pub fn remove_entry(&mut self, id_or_title: &str) -> Option<Entry> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id_or_title || e.title == id_or_title) {
            self.touch();
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// Serialize to MessagePack bytes.
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

impl Default for VaultData {
    fn default() -> Self {
        Self::new()
    }
}
