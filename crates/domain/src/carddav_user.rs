use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarddavUser {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub addressbook_slug: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
