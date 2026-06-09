use crate::{
    app::AppState,
    error::{ApiError, ApiResult},
    routes::admin::ContactRow,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use carddav::xml::DavResource;
use domain::Contact;
use uuid::Uuid;
use vcard::VcardOptions;

pub async fn collection(
    State(state): State<AppState>,
    Path((tenant_slug, addressbook_slug)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    _body: Bytes,
) -> ApiResult<Response> {
    if carddav::methods::READ_ONLY_FORBIDDEN.contains(&method.as_str()) {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    if method == Method::OPTIONS {
        return Ok(options_response());
    }
    let auth = authenticate(&state, &headers, &tenant_slug, &addressbook_slug).await?;
    match method.as_str() {
        "PROPFIND" => {
            propfind_collection(
                &state,
                auth.tenant_id,
                &tenant_slug,
                &addressbook_slug,
                depth(&headers),
            )
            .await
        }
        "REPORT" => report_all(&state, auth.tenant_id, &tenant_slug, &addressbook_slug).await,
        "GET" => Ok(StatusCode::OK.into_response()),
        _ => Ok(StatusCode::METHOD_NOT_ALLOWED.into_response()),
    }
}

pub async fn contact(
    State(state): State<AppState>,
    Path((tenant_slug, addressbook_slug, contact)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
    _body: Bytes,
) -> ApiResult<Response> {
    if carddav::methods::READ_ONLY_FORBIDDEN.contains(&method.as_str()) {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    if method == Method::OPTIONS {
        return Ok(options_response());
    }
    let auth = authenticate(&state, &headers, &tenant_slug, &addressbook_slug).await?;
    let uid = contact.strip_suffix(".vcf").unwrap_or(&contact);
    let contact_id = Uuid::parse_str(uid).map_err(|_| ApiError::NotFound)?;
    let c = fetch_contact(&state, auth.tenant_id, contact_id).await?;
    match method.as_str() {
        "GET" => {
            let body = vcard::build_vcard(&c, &VcardOptions::default());
            let etag = vcard::contact_etag(&c, &VcardOptions::default());
            Ok((
                [
                    (header::CONTENT_TYPE, "text/vcard; charset=utf-8"),
                    (header::ETAG, etag.as_str()),
                ],
                body,
            )
                .into_response())
        }
        "PROPFIND" | "REPORT" => report_one(&tenant_slug, &addressbook_slug, &c),
        _ => Ok(StatusCode::METHOD_NOT_ALLOWED.into_response()),
    }
}

struct AuthContext {
    tenant_id: Uuid,
}
async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    tenant_slug: &str,
    addressbook_slug: &str,
) -> ApiResult<AuthContext> {
    let Some(h) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Err(ApiError::Unauthorized);
    };
    let Some(creds) = carddav::auth::parse_basic_auth(h) else {
        return Err(ApiError::Unauthorized);
    };
    let row: Option<(Uuid, String, bool)> = sqlx::query_as("SELECT c.tenant_id, c.password_hash, c.is_active FROM carddav_users c JOIN tenants t ON t.id=c.tenant_id WHERE t.slug=$1 AND c.addressbook_slug=$2 AND c.username=$3")
        .bind(tenant_slug).bind(addressbook_slug).bind(&creds.username).fetch_optional(&state.db).await?;
    let Some((tenant_id, password_hash, is_active)) = row else {
        return Err(ApiError::Unauthorized);
    };
    if !is_active {
        return Err(ApiError::Unauthorized);
    }
    if !carddav::auth::verify_password(&creds.password, &password_hash) {
        return Err(ApiError::Unauthorized);
    }
    Ok(AuthContext { tenant_id })
}

async fn fetch_contacts(state: &AppState, tenant_id: Uuid) -> ApiResult<Vec<Contact>> {
    let rows = sqlx::query_as::<_, ContactRow>("SELECT * FROM contacts WHERE tenant_id=$1 AND is_deleted=false AND is_suppressed=false ORDER BY display_name NULLS LAST")
        .bind(tenant_id).fetch_all(&state.db).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
async fn fetch_contact(state: &AppState, tenant_id: Uuid, contact_id: Uuid) -> ApiResult<Contact> {
    let row = sqlx::query_as::<_, ContactRow>("SELECT * FROM contacts WHERE tenant_id=$1 AND id=$2 AND is_deleted=false AND is_suppressed=false")
        .bind(tenant_id).bind(contact_id).fetch_optional(&state.db).await?.ok_or(ApiError::NotFound)?;
    Ok(row.into())
}

async fn propfind_collection(
    state: &AppState,
    tenant_id: Uuid,
    tenant_slug: &str,
    addressbook_slug: &str,
    depth: u8,
) -> ApiResult<Response> {
    let mut resources = vec![DavResource {
        href: format!("/carddav/{tenant_slug}/{addressbook_slug}/"),
        etag: None,
        content_type: None,
        body: None,
    }];
    if depth > 0 {
        for c in fetch_contacts(state, tenant_id).await? {
            resources.push(resource_for(tenant_slug, addressbook_slug, &c, false));
        }
    }
    xml_response(carddav::xml::multistatus(&resources))
}
async fn report_all(
    state: &AppState,
    tenant_id: Uuid,
    tenant_slug: &str,
    addressbook_slug: &str,
) -> ApiResult<Response> {
    let resources = fetch_contacts(state, tenant_id)
        .await?
        .iter()
        .map(|c| resource_for(tenant_slug, addressbook_slug, c, true))
        .collect::<Vec<_>>();
    xml_response(carddav::xml::multistatus(&resources))
}
fn report_one(tenant_slug: &str, addressbook_slug: &str, c: &Contact) -> ApiResult<Response> {
    xml_response(carddav::xml::multistatus(&[resource_for(
        tenant_slug,
        addressbook_slug,
        c,
        true,
    )]))
}
fn resource_for(
    tenant_slug: &str,
    addressbook_slug: &str,
    c: &Contact,
    include_body: bool,
) -> DavResource {
    let body = include_body.then(|| vcard::build_vcard(c, &VcardOptions::default()));
    DavResource {
        href: format!("/carddav/{tenant_slug}/{addressbook_slug}/{}.vcf", c.id),
        etag: Some(vcard::contact_etag(c, &VcardOptions::default())),
        content_type: Some("text/vcard; charset=utf-8".into()),
        body,
    }
}
fn depth(headers: &HeaderMap) -> u8 {
    headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
fn xml_response(body: String) -> ApiResult<Response> {
    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body,
    )
        .into_response())
}
fn options_response() -> Response {
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut()
        .insert("DAV", HeaderValue::from_static("1, 2, addressbook"));
    res.headers_mut().insert(
        header::ALLOW,
        HeaderValue::from_static("OPTIONS, PROPFIND, REPORT, GET"),
    );
    res
}
