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
        // Social media proxy endpoints
        .route("/social/tikhub/generic", post(handlers::proxy_tikhub_generic))
        .route("/social/tikhub/twitter", post(handlers::proxy_tikhub_twitter))
        .route("/social/tikhub/tiktok", post(handlers::proxy_tikhub_tiktok))
        .route("/social/rapidapi/instagram", post(handlers::proxy_rapidapi_instagram))
        .route("/social/rapidapi/twitter-v24", post(handlers::proxy_rapidapi_twitter_v24))
        .route("/social/rapidapi/generic", post(handlers::proxy_rapidapi_generic))
        .with_state(pool)
}