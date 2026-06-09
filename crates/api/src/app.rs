use crate::config::Config;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::trace::TraceLayer;

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);
const RATE_LIMIT_REQUESTS_PER_WINDOW: usize = 100;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub graph: graph::client::GraphClient,
    pub rate_limiter: Arc<RateLimiter>,
}

#[derive(Debug)]
pub struct RateLimiter {
    requests: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(VecDeque::new()),
        }
    }

    fn allow(&self, now: Instant) -> Result<bool, ()> {
        let mut requests = self.requests.lock().map_err(|_| ())?;
        while requests
            .front()
            .is_some_and(|seen| now.duration_since(*seen) >= RATE_LIMIT_WINDOW)
        {
            requests.pop_front();
        }
        if requests.len() >= RATE_LIMIT_REQUESTS_PER_WINDOW {
            return Ok(false);
        }
        requests.push_back(now);
        Ok(true)
    }
}

async fn rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match state.rate_limiter.allow(Instant::now()) {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err(StatusCode::TOO_MANY_REQUESTS),
        Err(()) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(crate::routes::health::health))
        .route(
            "/admin/tenants",
            post(crate::routes::admin::create_tenant).get(crate::routes::admin::list_tenants),
        )
        .route(
            "/admin/tenants/:tenant_id/sync",
            post(crate::routes::admin::sync_tenant),
        )
        .route(
            "/admin/tenants/:tenant_id/carddav-users",
            post(crate::routes::admin::create_carddav_user),
        )
        .route(
            "/admin/tenants/:tenant_id/carddav-users/:user_id/rotate-password",
            post(crate::routes::admin::rotate_carddav_password),
        )
        .route(
            "/admin/tenants/:tenant_id/contacts",
            get(crate::routes::admin::list_contacts),
        )
        .route(
            "/admin/tenants/:tenant_id/test-contacts",
            post(crate::routes::admin::create_test_contact),
        )
        .route(
            "/admin/privacy/suppressions",
            post(crate::routes::admin::create_suppression)
                .get(crate::routes::admin::list_suppressions),
        )
        .route(
            "/admin/privacy/suppressions/:id",
            axum::routing::delete(crate::routes::admin::delete_suppression),
        )
        .route(
            "/carddav/:tenant_slug/:addressbook_slug/",
            axum::routing::any(crate::routes::carddav::collection),
        )
        .route(
            "/carddav/:tenant_slug/:addressbook_slug/:contact",
            axum::routing::any(crate::routes::carddav::contact),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
