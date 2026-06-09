#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppressionCandidate {
    pub normalized_email: String,
    pub suppression_hash: String,
}

impl SuppressionCandidate {
    pub fn from_email(secret: &str, email: &str) -> Self {
        let normalized_email = crate::normalize_email(email);
        let suppression_hash = crate::suppression_hmac_hex(secret, &normalized_email);
        Self {
            normalized_email,
            suppression_hash,
        }
    }
}
