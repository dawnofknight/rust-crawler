use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::crawler::{CrawlRequest, CrawlerError};

pub async fn crawl_website(
    Json(request): Json<CrawlRequest>,
) -> impl IntoResponse {
    match crate::crawler::crawl_website(request).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => {
            let (status, error_message) = match &err {
                CrawlerError::HttpError(e) => (StatusCode::BAD_REQUEST, format!("HTTP error: {}", e)),
                CrawlerError::RequestError(e) => (StatusCode::BAD_REQUEST, format!("Request error: {}", e)),
                CrawlerError::UrlParseError(e) => (StatusCode::BAD_REQUEST, format!("URL parse error: {}", e)),
                CrawlerError::UrlError(e) => (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)),
                CrawlerError::SelectorError(e) => (StatusCode::BAD_REQUEST, format!("Selector error: {}", e)),
                CrawlerError::TimeoutError => (StatusCode::OK, "Crawling exceeded the time limit".to_string()),
                CrawlerError::CrawlError(e) => (StatusCode::BAD_REQUEST, format!("Crawl error: {}", e)),
                CrawlerError::DateParsingError(e) => (StatusCode::BAD_REQUEST, format!("Date parsing error: {}", e)),
                CrawlerError::Other(e) => (StatusCode::BAD_REQUEST, format!("Other error: {}", e)),
            };
            
            (
                status,
                Json(json!({
                    "error": error_message
                })),
            )
                .into_response()
        }
    }
}