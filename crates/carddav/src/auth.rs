use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD, Engine};
use password_hash::SaltString;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicCredentials {
    pub username: String,
    pub password: String,
}

pub fn generate_app_password() -> String {
    OsRng
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

pub fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    // Security: CardDAV passwords are stored as Argon2id hashes only. Plaintext is returned once on creation.
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub fn verify_password(password: &str, encoded_hash: &str) -> bool {
    PasswordHash::new(encoded_hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

pub fn parse_basic_auth(header: &str) -> Option<BasicCredentials> {
    const MAX_BASIC_AUTH_VALUE_LEN: usize = 8 * 1024;

    let (scheme, value) = header.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Basic") {
        return None;
    }
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_BASIC_AUTH_VALUE_LEN {
        return None;
    }
    let decoded = STANDARD.decode(value).ok()?;
    if decoded.len() > MAX_BASIC_AUTH_VALUE_LEN {
        return None;
    }
    let text = String::from_utf8(decoded).ok()?;
    let (username, password) = text.split_once(':')?;
    if username.is_empty() {
        return None;
    }
    Some(BasicCredentials {
        username: username.to_string(),
        password: password.to_string(),
    })
}
