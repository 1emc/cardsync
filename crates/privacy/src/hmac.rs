use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub fn suppression_hmac_hex(secret: &str, normalized_email: &str) -> String {
    // Security: Do not log the raw email address. Suppression hashes are derived with HMAC-SHA256.
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return String::new();
    };
    mac.update(normalized_email.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
