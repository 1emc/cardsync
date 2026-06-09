use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Copy)]
pub struct ClientCredentials<'a> {
    pub tenant_id: &'a str,
    pub client_id: &'a str,
    pub client_secret: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn acquire_token(
    client: &Client,
    creds: ClientCredentials<'_>,
) -> anyhow::Result<String> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        creds.tenant_id
    );
    let params = [
        ("client_id", creds.client_id),
        ("client_secret", creds.client_secret),
        ("scope", "https://graph.microsoft.com/.default"),
        ("grant_type", "client_credentials"),
    ];
    let res = client
        .post(url)
        .form(&params)
        .send()
        .await
        .context("token request failed")?;
    if !res.status().is_success() {
        return Err(anyhow!("token endpoint returned {}", res.status()));
    }
    Ok(res.json::<TokenResponse>().await?.access_token)
}
