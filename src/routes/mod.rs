use axum::{
    routing::{get, post, put, delete},
    Router,
};
use sqlx::PgPool;

use crate::handlers;

pub fn create_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/users", get(handlers::get_users))
        .route("/users", post(handlers::create_user))
        .route("/users/{id}", get(handlers::get_user_by_id))
        .route("/users/{id}", put(handlers::update_user))
        .route("/users/{id}", delete(handlers::delete_user))
        .route("/crawl", post(handlers::crawl_website))
        .with_state(pool)
}