use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source: String,
    pub source_object_id: String,
    pub display_name: Option<String>,
    pub given_name: Option<String>,
    pub surname: Option<String>,
    pub email: Option<String>,
    pub user_principal_name: Option<String>,
    pub business_phone: Option<String>,
    pub mobile_phone: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub company_name: Option<String>,
    pub office_location: Option<String>,
    pub etag: Option<String>,
    pub is_deleted: bool,
    pub is_suppressed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl Contact {
    pub fn carddav_uid(&self) -> String {
        self.id.to_string()
    }
}
