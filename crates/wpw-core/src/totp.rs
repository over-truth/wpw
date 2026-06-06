use totp_rs::{Algorithm, TOTP, Secret};

#[derive(Debug, thiserror::Error)]
pub enum TotpError {
    #[error("invalid secret: {0}")]
    InvalidSecret(String),
    #[error("TOTP generation failed: {0}")]
    GenerationFailed(String),
}

/// Generate a TOTP code from a Base32-encoded secret.
/// Returns (code, remaining_seconds).
pub fn generate_totp(secret_base32: &str, _issuer: Option<&str>) -> Result<(String, u32), TotpError> {
    let secret_bytes = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| TotpError::InvalidSecret(format!("invalid base32: {e}")))?;
    
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,      // 6 digits
        1,      // 1 step tolerance (±1 window)
        30,     // 30 second period
        secret_bytes,
    ).map_err(|e| TotpError::InvalidSecret(e.to_string()))?;
    
    let code = totp.generate_current()
        .map_err(|e| TotpError::GenerationFailed(e.to_string()))?;
    
    // Calculate remaining seconds
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let step = 30u64;
    let remaining = step - (now % step);
    
    Ok((code, remaining as u32))
}
