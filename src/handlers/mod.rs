use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::PgPool;

use crate::models::{ApiError, ApiResponse, CreateUserRequest, UpdateUserRequest, User};

mod crawler;
pub use crawler::crawl_website;
mod social;
pub use social::{
    proxy_tikhub_twitter,
    proxy_tikhub_tiktok,
    proxy_rapidapi_instagram,
    proxy_rapidapi_twitter_v24,
    proxy_rapidapi_generic,
    proxy_tikhub_generic,
};

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "API is running")
}

pub async fn get_users(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool)
        .await
    {
        Ok(users) => (
            StatusCode::OK,
            Json(ApiResponse::success(users)),
        ),
        Err(e) => handle_error(ApiError::from(e)),
    }
}

pub async fn get_user_by_id(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
    {
        Ok(user) => (
            StatusCode::OK,
            Json(ApiResponse::success(user)),
        ),
        Err(e) => handle_error(ApiError::from(e)),
    }
}

pub async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .fetch_one(&pool)
    .await
    {
        Ok(user) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(user)),
        ),
        Err(e) => handle_error(ApiError::from(e)),
    }
}

pub async fn update_user(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    // First check if user exists
    let user_exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(id)
        .fetch_one(&pool)
        .await;

    if let Ok(false) | Err(_) = user_exists {
        return handle_error(ApiError::NotFound(format!("User with id {} not found", id)));
    }

    // Build dynamic update query
    let mut query = String::from("UPDATE users SET updated_at = NOW()");
    let mut bindings = vec![];

    if let Some(name) = &payload.name {
        query.push_str(", name = $1");
        bindings.push(name);
    }

    if let Some(email) = &payload.email {
        if bindings.is_empty() {
            query.push_str(", email = $1");
        } else {
            query.push_str(", email = $2");
        }
        bindings.push(email);
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING *", bindings.len() + 1));

    // Build and execute the query
    let mut db_query = sqlx::query_as::<_, User>(&query);
    
    for binding in bindings {
        db_query = db_query.bind(binding);
    }
    
    db_query = db_query.bind(id);

    match db_query.fetch_one(&pool).await {
        Ok(user) => (
            StatusCode::OK,
            Json(ApiResponse::success(user)),
        ),
        Err(e) => handle_error(ApiError::from(e)),
    }
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, User>("DELETE FROM users WHERE id = $1 RETURNING *")
        .bind(id)
        .fetch_one(&pool)
        .await
    {
        Ok(user) => (
            StatusCode::OK,
            Json(ApiResponse::success(user)),
        ),
        Err(e) => handle_error(ApiError::from(e)),
    }
}

// Error handling function
fn handle_error<T>(err: ApiError) -> (StatusCode, Json<ApiResponse<T>>) {
    let status = match &err {
        ApiError::NotFound(_) => StatusCode::NOT_FOUND,
        ApiError::ValidationError(_) => StatusCode::BAD_REQUEST,
        ApiError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ApiError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, Json(ApiResponse::error(&err.to_string())))
}