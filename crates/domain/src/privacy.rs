use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySuppression {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_value: Option<String>,
    pub suppression_hash: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
