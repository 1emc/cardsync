use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphUsersResponse {
    pub value: Vec<GraphUser>,
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphUser {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "givenName")]
    pub given_name: Option<String>,
    pub surname: Option<String>,
    pub mail: Option<String>,
    #[serde(rename = "userPrincipalName")]
    pub user_principal_name: Option<String>,
    #[serde(rename = "businessPhones", default)]
    pub business_phones: Vec<String>,
    #[serde(rename = "mobilePhone")]
    pub mobile_phone: Option<String>,
    #[serde(rename = "jobTitle")]
    pub job_title: Option<String>,
    pub department: Option<String>,
    #[serde(rename = "companyName")]
    pub company_name: Option<String>,
    #[serde(rename = "officeLocation")]
    pub office_location: Option<String>,
    #[serde(rename = "accountEnabled")]
    pub account_enabled: Option<bool>,
}

impl GraphUser {
    pub fn is_syncable(&self) -> bool {
        self.account_enabled.unwrap_or(false)
            && (self.mail.as_deref().is_some_and(|v| !v.is_empty())
                || self
                    .user_principal_name
                    .as_deref()
                    .is_some_and(|v| !v.is_empty()))
            && (self.display_name.as_deref().is_some_and(|v| !v.is_empty())
                || self.mail.as_deref().is_some_and(|v| !v.is_empty()))
    }
}
