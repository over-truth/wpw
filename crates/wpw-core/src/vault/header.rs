use thiserror::Error;

#[derive(Error, Debug)]
pub enum HeaderError {
    #[error("invalid magic bytes")]
    InvalidMagic,
    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u16),
    #[error("header too short")]
    TooShort,
}

/// Vault file magic bytes: "WPW\0"
pub const MAGIC: [u8; 4] = [0x57, 0x50, 0x57, 0x00];
pub const CURRENT_VERSION: u16 = 1;
pub const HEADER_LEN: usize = 79;

pub struct VaultHeader {
    pub format_version: u16,
    pub header_length: u32,
    pub payload_length: u32,
    pub salt: [u8; 32],
    pub m_cost_exponent: u8, // actual m_cost = 2^n KiB
    pub t_cost: u8,
    pub p_cost: u8,
    pub nonce: [u8; 12],
}

impl VaultHeader {
    /// Parse header from bytes. Expects at least HEADER_LEN bytes.
    pub fn parse(data: &[u8]) -> Result<Self, HeaderError> {
        if data.len() < HEADER_LEN {
            return Err(HeaderError::TooShort);
        }

        // Check magic bytes
        if data[0..4] != MAGIC {
            return Err(HeaderError::InvalidMagic);
        }

        let format_version = u16::from_le_bytes([data[4], data[5]]);
        if format_version > CURRENT_VERSION {
            return Err(HeaderError::UnsupportedVersion(format_version));
        }

        let header_length = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
        let payload_length = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);

        let mut salt = [0u8; 32];
        salt.copy_from_slice(&data[14..46]);

        let m_cost_exponent = data[46];
        let t_cost = data[47];
        let p_cost = data[48];

        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&data[49..61]);

        Ok(Self {
            format_version,
            header_length,
            payload_length,
            salt,
            m_cost_exponent,
            t_cost,
            p_cost,
            nonce,
        })
    }

    /// Serialize header to bytes (HEADER_LEN bytes).
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4..6].copy_from_slice(&self.format_version.to_le_bytes());
        buf[6..10].copy_from_slice(&self.header_length.to_le_bytes());
        buf[10..14].copy_from_slice(&self.payload_length.to_le_bytes());
        buf[14..46].copy_from_slice(&self.salt);
        buf[46] = self.m_cost_exponent;
        buf[47] = self.t_cost;
        buf[48] = self.p_cost;
        buf[49..61].copy_from_slice(&self.nonce);
        // bytes 61..79 are reserved (zero-filled)
        buf
    }

    /// Get the AAD bytes (immutable header fields): magic + version + salt = 4 + 2 + 32 = 38 bytes
    pub fn aad_bytes(&self) -> [u8; 38] {
        let mut aad = [0u8; 38];
        aad[0..4].copy_from_slice(&MAGIC);
        aad[4..6].copy_from_slice(&self.format_version.to_le_bytes());
        aad[6..38].copy_from_slice(&self.salt);
        aad
    }

    /// Get the m_cost value from exponent (m_cost = 2^exponent KiB)
    pub fn m_cost(&self) -> u32 {
        1u32 << self.m_cost_exponent
    }

    /// Create a new header with random salt and nonce.
    pub fn new_random(m_cost: u32, t_cost: u8, p_cost: u8, payload_length: u32) -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();

        let mut salt = [0u8; 32];
        rng.fill_bytes(&mut salt);

        let mut nonce = [0u8; 12];
        rng.fill_bytes(&mut nonce);

        // m_cost_exponent: find n such that 2^n = m_cost
        let m_cost_exponent = if m_cost == 0 { 0 } else { 31 - m_cost.leading_zeros() as u8 };

        Self {
            format_version: CURRENT_VERSION,
            header_length: HEADER_LEN as u32,
            payload_length,
            salt,
            m_cost_exponent,
            t_cost,
            p_cost,
            nonce,
        }
    }
}
