use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error)]
pub enum SuppressionHashError {
    #[error("suppression secret must not be empty")]
    EmptySecret,
    #[error("suppression secret has an invalid HMAC length")]
    InvalidKeyLength,
}

pub fn suppression_hmac_hex(
    secret: &str,
    normalized_email: &str,
) -> Result<String, SuppressionHashError> {
    // Security: Do not log the raw email address. Suppression hashes are derived with HMAC-SHA256.
    if secret.is_empty() {
        return Err(SuppressionHashError::EmptySecret);
    }
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| SuppressionHashError::InvalidKeyLength)?;
    mac.update(normalized_email.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let mut left = Sha256::new();
    left.update(a.as_bytes());
    let left = left.finalize();

    let mut right = Sha256::new();
    right.update(b.as_bytes());
    let right = right.finalize();

    left.ct_eq(&right).into()
}
