use zeroize::Zeroize;

use argon2::{Algorithm, Argon2, Params, Version};

use crate::crypto::key::EncryptionKey;

pub struct KdfParams {
    pub m_cost: u32, // memory in KiB (default 65536 = 64 MiB)
    pub t_cost: u32, // iterations (default 3)
    pub p_cost: u32, // parallelism (default 4)
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost: 65536,
            t_cost: 3,
            p_cost: 4,
        }
    }
}

pub struct DerivedKeys {
    pub encryption_key: EncryptionKey,
    pub hmac_key: [u8; 32], // reserved for future use
}

/// Derives encryption key and HMAC key from master password and salt using Argon2id.
pub fn derive_keys(
    password: &[u8],
    salt: &[u8],
    params: &KdfParams,
) -> Result<DerivedKeys, argon2::Error> {
    let mut output = [0u8; 64];
    let argon2_params = Params::new(params.m_cost, params.t_cost, params.p_cost, Some(64))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);
    argon2.hash_password_into(password, salt, &mut output)?;

    let mut enc_key_bytes = [0u8; 32];
    let mut hmac_key_bytes = [0u8; 32];
    enc_key_bytes.copy_from_slice(&output[..32]);
    hmac_key_bytes.copy_from_slice(&output[32..]);

    // Zeroize the full output
    output.zeroize();

    Ok(DerivedKeys {
        encryption_key: EncryptionKey::new(enc_key_bytes),
        hmac_key: hmac_key_bytes,
    })
}
