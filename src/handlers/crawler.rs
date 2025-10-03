use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{json, Value};
use crate::kafka::{create_producer, produce_json};

use crate::crawler::{CrawlRequest, CrawlerError};

pub async fn crawl_website(
    Json(request): Json<CrawlRequest>,
) -> impl IntoResponse {
    match crate::crawler::crawl_website(&request).await {
        Ok(result) => {
            // Produce to Kafka topic if configured
            if let (Ok(brokers), Ok(topic)) = (std::env::var("KAFKA_BROKERS"), std::env::var("KAFKA_TOPIC_CRAWL")) {
                if let Ok(producer) = create_producer(&brokers) {
                    let payload: Value = serde_json::to_value(&result).unwrap_or(json!({"error":"serialize"}));
                    let _ = produce_json(&producer, &topic, None, &payload).await;
                }
            }
            (StatusCode::OK, Json(result)).into_response()
        },
        Err(err) => {
            let (status, error_message) = match &err {
                CrawlerError::RequestError(e) => (StatusCode::BAD_REQUEST, format!("Request error: {}", e)),
                CrawlerError::UrlError(e) => (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)),
                CrawlerError::SelectorError(e) => (StatusCode::BAD_REQUEST, format!("Selector error: {}", e)),
                CrawlerError::TimeoutError => (StatusCode::OK, "Crawling exceeded the time limit".to_string()),
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