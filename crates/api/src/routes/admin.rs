use crate::{
    app::AppState,
    error::{ApiError, ApiResult},
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use domain::{Contact, PrivacySuppression, Tenant};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::FromRow;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;
const ADMIN_TOKEN_COMPARE_KEY: &[u8] = b"galcard-admin-token-comparison-v1";

fn require_admin(headers: &HeaderMap, state: &AppState) -> ApiResult<()> {
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();

    let provided_mac = token_mac(provided)?;
    let expected_mac = token_mac(state.config.admin_api_token.expose_secret())?;
    if privacy::constant_time_eq(&provided_mac, &expected_mac) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn token_mac(token: &str) -> ApiResult<String> {
    let mut mac = HmacSha256::new_from_slice(ADMIN_TOKEN_COMPARE_KEY)
        .map_err(|_| anyhow::anyhow!("invalid admin token comparison key"))?;
    mac.update(token.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn validate_slug(slug: &str, field: &str) -> ApiResult<()> {
    if slug.is_empty() || slug.len() > 63 {
        return Err(ApiError::BadRequest(format!(
            "{field} must be 1-63 characters"
        )));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ApiError::BadRequest(format!(
            "{field} may only contain lowercase letters, digits, and hyphens"
        )));
    }
    Ok(())
}

fn validate_username(username: &str) -> ApiResult<()> {
    if username.is_empty() || username.len() > 128 {
        return Err(ApiError::BadRequest(
            "username must be 1-128 characters".into(),
        ));
    }
    // A ':' would break HTTP Basic auth parsing (user:password); whitespace and
    // control characters would make the username impossible to type into a client.
    if !username.chars().all(|c| c.is_ascii_graphic() && c != ':') {
        return Err(ApiError::BadRequest(
            "username may only contain printable ASCII characters without spaces or ':'".into(),
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateTenant {
    slug: String,
    display_name: String,
    microsoft_tenant_id: Option<String>,
}
#[derive(Deserialize)]
pub struct CreateCarddavUser {
    username: String,
    addressbook_slug: Option<String>,
}
#[derive(Serialize)]
pub struct PasswordResponse {
    id: Uuid,
    username: String,
    password: String,
}
#[derive(Deserialize)]
pub struct CreateSuppression {
    email: String,
    scope_type: String,
    scope_value: Option<String>,
    reason: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}
#[derive(Deserialize)]
pub struct CreateTestContact {
    display_name: Option<String>,
    given_name: Option<String>,
    surname: Option<String>,
    email: Option<String>,
    user_principal_name: Option<String>,
    business_phone: Option<String>,
    mobile_phone: Option<String>,
    job_title: Option<String>,
    department: Option<String>,
    company_name: Option<String>,
    office_location: Option<String>,
}
#[derive(Serialize)]
pub struct ContactList {
    active: Vec<Contact>,
    suppressed_count: i64,
}
#[derive(Serialize)]
pub struct SyncResult {
    synced: usize,
    suppressed: usize,
    source: &'static str,
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTenant>,
) -> ApiResult<Json<Tenant>> {
    require_admin(&headers, &state)?;
    validate_slug(&req.slug, "slug")?;
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, TenantRow>("INSERT INTO tenants (id, slug, display_name, microsoft_tenant_id) VALUES ($1,$2,$3,$4) RETURNING *")
        .bind(id).bind(req.slug).bind(req.display_name).bind(req.microsoft_tenant_id).fetch_one(&state.db).await?;
    Ok(Json(row.into()))
}

pub async fn list_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Tenant>>> {
    require_admin(&headers, &state)?;
    let rows = sqlx::query_as::<_, TenantRow>("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

pub async fn create_carddav_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<CreateCarddavUser>,
) -> ApiResult<Json<PasswordResponse>> {
    require_admin(&headers, &state)?;
    validate_username(&req.username)?;
    let password = carddav::auth::generate_app_password();
    let hash =
        carddav::auth::hash_password(&password).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let id = Uuid::new_v4();
    let addressbook = req.addressbook_slug.unwrap_or_else(|| "default".into());
    validate_slug(&addressbook, "addressbook_slug")?;
    sqlx::query("INSERT INTO carddav_users (id, tenant_id, username, password_hash, addressbook_slug) VALUES ($1,$2,$3,$4,$5)")
        .bind(id).bind(tenant_id).bind(&req.username).bind(hash).bind(addressbook).execute(&state.db).await?;
    Ok(Json(PasswordResponse {
        id,
        username: req.username,
        password,
    }))
}

pub async fn rotate_carddav_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PasswordResponse>> {
    require_admin(&headers, &state)?;
    let password = carddav::auth::generate_app_password();
    let hash =
        carddav::auth::hash_password(&password).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let rec: (String,) = sqlx::query_as("UPDATE carddav_users SET password_hash=$1, updated_at=now() WHERE tenant_id=$2 AND id=$3 RETURNING username")
        .bind(hash).bind(tenant_id).bind(user_id).fetch_optional(&state.db).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(PasswordResponse {
        id: user_id,
        username: rec.0,
        password,
    }))
}

pub async fn create_test_contact(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<CreateTestContact>,
) -> ApiResult<Json<Contact>> {
    require_admin(&headers, &state)?;
    let email_for_hash = req.email.as_deref().or(req.user_principal_name.as_deref());
    let suppressed = if let Some(email) = email_for_hash {
        is_suppressed(&state, tenant_id, email).await?
    } else {
        false
    };
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, ContactRow>("INSERT INTO contacts (id, tenant_id, source, source_object_id, display_name, given_name, surname, email, user_principal_name, business_phone, mobile_phone, job_title, department, company_name, office_location, is_suppressed, last_seen_at) VALUES ($1,$2,'test',$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,now()) RETURNING *")
        .bind(id).bind(tenant_id).bind(id.to_string()).bind(req.display_name).bind(req.given_name).bind(req.surname).bind(req.email).bind(req.user_principal_name).bind(req.business_phone).bind(req.mobile_phone).bind(req.job_title).bind(req.department).bind(req.company_name).bind(req.office_location).bind(suppressed).fetch_one(&state.db).await?;
    Ok(Json(row.into()))
}

pub async fn list_contacts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<ContactList>> {
    require_admin(&headers, &state)?;
    let rows = sqlx::query_as::<_, ContactRow>("SELECT * FROM contacts WHERE tenant_id=$1 AND is_deleted=false AND is_suppressed=false ORDER BY display_name NULLS LAST").bind(tenant_id).fetch_all(&state.db).await?;
    let suppressed: (i64,) =
        sqlx::query_as("SELECT count(*) FROM contacts WHERE tenant_id=$1 AND is_suppressed=true")
            .bind(tenant_id)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(ContactList {
        active: rows.into_iter().map(Into::into).collect(),
        suppressed_count: suppressed.0,
    }))
}

pub async fn create_suppression(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSuppression>,
) -> ApiResult<Json<PrivacySuppression>> {
    require_admin(&headers, &state)?;
    // Security: Do not log the raw email address. Only the HMAC suppression hash is stored.
    if !matches!(req.scope_type.as_str(), "global" | "tenant") {
        return Err(ApiError::BadRequest(
            "scope_type must be global or tenant".into(),
        ));
    }
    let hash = privacy::SuppressionCandidate::from_email(
        state.config.suppression_secret.expose_secret(),
        &req.email,
    )
    .map_err(anyhow::Error::from)?
    .suppression_hash;
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, SuppressionRow>("INSERT INTO privacy_suppressions (id, scope_type, scope_value, suppression_hash, reason, expires_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING *")
        .bind(id).bind(req.scope_type).bind(req.scope_value).bind(hash).bind(req.reason).bind(req.expires_at).fetch_one(&state.db).await?;
    Ok(Json(row.into()))
}

pub async fn list_suppressions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<PrivacySuppression>>> {
    require_admin(&headers, &state)?;
    let rows = sqlx::query_as::<_, SuppressionRow>(
        "SELECT * FROM privacy_suppressions ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

pub async fn delete_suppression(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&headers, &state)?;
    let result = sqlx::query("DELETE FROM privacy_suppressions WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(Json(serde_json::json!({"deleted": true})))
}

pub async fn sync_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<SyncResult>> {
    require_admin(&headers, &state)?;
    let creds = match (
        &state.config.microsoft_tenant_id,
        &state.config.microsoft_client_id,
        &state.config.microsoft_client_secret,
    ) {
        (Some(t), Some(c), Some(s)) => graph::auth::ClientCredentials {
            tenant_id: t.as_str(),
            client_id: c.as_str(),
            client_secret: s.expose_secret(),
        },
        _ => {
            return Err(ApiError::BadRequest(
                "Microsoft Graph credentials are not configured".into(),
            ))
        }
    };
    let mut tx = state.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(tenant_id.to_string())
        .bind(graph::sync::SOURCE_GRAPH_USERS)
        .execute(&mut *tx)
        .await?;
    let token = graph::auth::acquire_token(state.graph.http(), creds)
        .await
        .map_err(|e| {
            tracing::warn!(tenant_id=%tenant_id, error_code="graph_token_failed");
            e
        })?;
    let users = state.graph.users_full_sync(&token).await.map_err(|e| {
        tracing::warn!(tenant_id=%tenant_id, error_code="graph_sync_failed");
        e
    })?;
    let sync_started_at: (DateTime<Utc>,) =
        sqlx::query_as("SELECT now()").fetch_one(&mut *tx).await?;

    let mut synced = 0usize;
    let mut suppressed = 0usize;
    for u in users {
        let email = u.mail.as_deref().or(u.user_principal_name.as_deref());
        let is_sup = if let Some(e) = email {
            is_suppressed_in_tx(
                &mut tx,
                state.config.suppression_secret.expose_secret(),
                tenant_id,
                e,
            )
            .await?
        } else {
            false
        };
        if is_sup {
            suppressed += 1;
        }
        let id = Uuid::new_v4();
        let business_phone = u.business_phones.first().cloned();
        sqlx::query("INSERT INTO contacts (id, tenant_id, source, source_object_id, display_name, given_name, surname, email, user_principal_name, business_phone, mobile_phone, job_title, department, company_name, office_location, is_suppressed, is_deleted, last_seen_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,false,now()) ON CONFLICT (tenant_id, source, source_object_id) DO UPDATE SET display_name=EXCLUDED.display_name, given_name=EXCLUDED.given_name, surname=EXCLUDED.surname, email=EXCLUDED.email, user_principal_name=EXCLUDED.user_principal_name, business_phone=EXCLUDED.business_phone, mobile_phone=EXCLUDED.mobile_phone, job_title=EXCLUDED.job_title, department=EXCLUDED.department, company_name=EXCLUDED.company_name, office_location=EXCLUDED.office_location, is_suppressed=EXCLUDED.is_suppressed, is_deleted=false, updated_at=now(), last_seen_at=now()")
            .bind(id).bind(tenant_id).bind(graph::sync::SOURCE_GRAPH_USERS).bind(u.id).bind(u.display_name).bind(u.given_name).bind(u.surname).bind(u.mail).bind(u.user_principal_name).bind(business_phone).bind(u.mobile_phone).bind(u.job_title).bind(u.department).bind(u.company_name).bind(u.office_location).bind(is_sup).execute(&mut *tx).await?;
        synced += 1;
    }
    sqlx::query("UPDATE contacts SET is_deleted=true, updated_at=now() WHERE tenant_id=$1 AND source=$2 AND is_deleted=false AND (last_seen_at IS NULL OR last_seen_at < $3)")
        .bind(tenant_id)
        .bind(graph::sync::SOURCE_GRAPH_USERS)
        .bind(sync_started_at.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO sync_state (tenant_id, source, last_success_at) VALUES ($1,$2,now()) ON CONFLICT (tenant_id, source) DO UPDATE SET last_success_at=now(), last_error_at=NULL, last_error_message=NULL")
        .bind(tenant_id).bind(graph::sync::SOURCE_GRAPH_USERS).execute(&mut *tx).await?;
    tx.commit().await?;
    tracing::info!(tenant_id=%tenant_id, count_contacts_synced=synced, count_contacts_suppressed=suppressed);
    Ok(Json(SyncResult {
        synced,
        suppressed,
        source: graph::sync::SOURCE_GRAPH_USERS,
    }))
}

async fn is_suppressed(state: &AppState, tenant_id: Uuid, email: &str) -> ApiResult<bool> {
    let candidate = privacy::SuppressionCandidate::from_email(
        state.config.suppression_secret.expose_secret(),
        email,
    )
    .map_err(anyhow::Error::from)?;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT suppression_hash FROM privacy_suppressions WHERE suppression_hash=$1 AND (scope_type='global' OR (scope_type='tenant' AND scope_value=$2)) AND (expires_at IS NULL OR expires_at > now())")
        .bind(&candidate.suppression_hash).bind(tenant_id.to_string()).fetch_all(&state.db).await?;
    Ok(rows
        .iter()
        .any(|r| privacy::constant_time_eq(&r.0, &candidate.suppression_hash)))
}

async fn is_suppressed_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    suppression_secret: &str,
    tenant_id: Uuid,
    email: &str,
) -> ApiResult<bool> {
    let candidate = privacy::SuppressionCandidate::from_email(suppression_secret, email)
        .map_err(anyhow::Error::from)?;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT suppression_hash FROM privacy_suppressions WHERE suppression_hash=$1 AND (scope_type='global' OR (scope_type='tenant' AND scope_value=$2)) AND (expires_at IS NULL OR expires_at > now())")
        .bind(&candidate.suppression_hash)
        .bind(tenant_id.to_string())
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .iter()
        .any(|r| privacy::constant_time_eq(&r.0, &candidate.suppression_hash)))
}

#[derive(FromRow)]
struct TenantRow {
    id: Uuid,
    slug: String,
    display_name: String,
    microsoft_tenant_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
impl From<TenantRow> for Tenant {
    fn from(r: TenantRow) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            display_name: r.display_name,
            microsoft_tenant_id: r.microsoft_tenant_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
#[derive(FromRow)]
pub(crate) struct ContactRow {
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
impl From<ContactRow> for Contact {
    fn from(r: ContactRow) -> Self {
        Self {
            id: r.id,
            tenant_id: r.tenant_id,
            source: r.source,
            source_object_id: r.source_object_id,
            display_name: r.display_name,
            given_name: r.given_name,
            surname: r.surname,
            email: r.email,
            user_principal_name: r.user_principal_name,
            business_phone: r.business_phone,
            mobile_phone: r.mobile_phone,
            job_title: r.job_title,
            department: r.department,
            company_name: r.company_name,
            office_location: r.office_location,
            etag: r.etag,
            is_deleted: r.is_deleted,
            is_suppressed: r.is_suppressed,
            created_at: r.created_at,
            updated_at: r.updated_at,
            last_seen_at: r.last_seen_at,
        }
    }
}
#[derive(FromRow)]
struct SuppressionRow {
    id: Uuid,
    scope_type: String,
    scope_value: Option<String>,
    suppression_hash: String,
    reason: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}
impl From<SuppressionRow> for PrivacySuppression {
    fn from(r: SuppressionRow) -> Self {
        Self {
            id: r.id,
            scope_type: r.scope_type,
            scope_value: r.scope_value,
            suppression_hash: r.suppression_hash,
            reason: r.reason,
            created_at: r.created_at,
            expires_at: r.expires_at,
        }
    }
}
