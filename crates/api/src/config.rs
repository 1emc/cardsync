use std::{env, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub public_base_url: String,
    pub database_url: String,
    pub admin_api_token: String,
    pub suppression_secret: String,
    pub microsoft_tenant_id: Option<String>,
    pub microsoft_client_id: Option<String>,
    pub microsoft_client_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        match dotenvy::dotenv() {
            Ok(_) | Err(dotenvy::Error::Io(_)) => {}
            Err(e) => return Err(e.into()),
        }

        let admin_api_token = required_secret("ADMIN_API_TOKEN", 32)?;
        let suppression_secret = required_secret("SUPPRESSION_SECRET", 32)?;

        Ok(Self {
            bind_addr: env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:3000".into())
                .parse()?,
            public_base_url: env::var("PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            database_url: required_non_empty("DATABASE_URL")?,
            admin_api_token,
            suppression_secret,
            microsoft_tenant_id: env::var("MICROSOFT_TENANT_ID")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            microsoft_client_id: env::var("MICROSOFT_CLIENT_ID")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            microsoft_client_secret: env::var("MICROSOFT_CLIENT_SECRET")
                .ok()
                .filter(|v| !v.trim().is_empty()),
        })
    }
}

fn required_non_empty(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)?;
    if value.trim().is_empty() {
        anyhow::bail!("{name} must not be empty");
    }
    Ok(value)
}

fn required_secret(name: &str, min_len: usize) -> anyhow::Result<String> {
    let value = required_non_empty(name)?;
    if value.len() < min_len {
        anyhow::bail!("{name} must be at least {min_len} bytes long");
    }
    Ok(value)
}
