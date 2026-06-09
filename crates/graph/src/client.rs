use crate::models::GraphUsersResponse;
use anyhow::{anyhow, bail, Context};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use url::Url;

const SELECT: &str = "id,displayName,givenName,surname,mail,userPrincipalName,businessPhones,mobilePhone,jobTitle,department,companyName,officeLocation,accountEnabled";

#[derive(Clone)]
pub struct GraphClient {
    http: Client,
}
impl GraphClient {
    pub fn new() -> anyhow::Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()
            .context("failed to build graph HTTP client")?;
        Ok(Self { http })
    }

    pub fn http(&self) -> &Client {
        &self.http
    }
    pub async fn users_full_sync(
        &self,
        access_token: &str,
    ) -> anyhow::Result<Vec<crate::models::GraphUser>> {
        // Privacy: Photos are intentionally not synced in the MVP.
        let mut url = "https://graph.microsoft.com/v1.0/users".to_string();
        let mut users = Vec::new();
        let mut attempts = 0u8;
        loop {
            let req = if url.contains('?') {
                self.http.get(&url)
            } else {
                self.http.get(&url).query(&[("$select", SELECT)])
            };
            let res = req
                .bearer_auth(access_token)
                .send()
                .await
                .context("graph users request failed")?;
            if res.status() == StatusCode::TOO_MANY_REQUESTS || res.status().is_server_error() {
                if attempts >= 3 {
                    return Err(anyhow!(
                        "graph retry limit exceeded with status {}",
                        res.status()
                    ));
                }
                let wait = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2u64.pow(attempts as u32));
                tokio::time::sleep(Duration::from_secs(wait.min(30))).await;
                attempts += 1;
                continue;
            }
            if !res.status().is_success() {
                return Err(anyhow!("graph users returned {}", res.status()));
            }
            attempts = 0;
            let page: GraphUsersResponse = res.json().await?;
            users.extend(page.value.into_iter().filter(|u| u.is_syncable()));
            if let Some(next) = page.next_link {
                validate_graph_next_link(&next)?;
                url = next;
            } else {
                break;
            }
        }
        Ok(users)
    }
}

fn validate_graph_next_link(next_link: &str) -> anyhow::Result<()> {
    let parsed = Url::parse(next_link).context("invalid Graph nextLink URL")?;
    if parsed.scheme() != "https" {
        bail!("Graph nextLink must use HTTPS");
    }
    let Some(host) = parsed.host_str() else {
        bail!("Graph nextLink must include a host");
    };
    if host != "graph.microsoft.com" {
        bail!("Graph nextLink host is not allowed: {host}");
    }
    Ok(())
}
