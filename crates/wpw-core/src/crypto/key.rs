use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    key_bytes: [u8; 32],
}

impl EncryptionKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self { key_bytes: bytes }
    }

    pub fn expose_key(&self) -> &[u8; 32] {
        &self.key_bytes
    }
}
