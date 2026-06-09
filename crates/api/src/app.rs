use crate::config::Config;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub graph: graph::client::GraphClient,
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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
