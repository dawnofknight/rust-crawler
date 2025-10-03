use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const TIKHUB_TWITTER_BASE: &str = "https://api.tikhub.io/api/v1/twitter/web/";
const TIKHUB_TIKTOK_BASE: &str = "https://api.tikhub.io/api/v1/tiktok/web/";
const RAPIDAPI_INSTAGRAM_HOST: &str = "instagram-scraper-api2.p.rapidapi.com";
const RAPIDAPI_TWITTER_V24_HOST: &str = "twitter-v24.p.rapidapi.com";

#[derive(Deserialize)]
pub struct ProxyRequest {
    pub path: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
}

#[derive(Serialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub data: Value,
}

fn params_to_query(params: &Option<Value>) -> Vec<(String, String)> {
    let mut query = Vec::new();
    if let Some(Value::Object(map)) = params {
        for (k, v) in map.iter() {
            let val = match v {
                Value::String(s) => s.clone(),
                _ => v.to_string(),
            };
            query.push((k.clone(), val));
        }
    }
    query
}

// For TikHub Twitter, ensure `keyword` is used (map `q` -> `keyword`) and default `search_type=Top`.
fn tikhub_twitter_query(params: &Option<Value>) -> Vec<(String, String)> {
    let mut query: Vec<(String, String)> = Vec::new();
    let mut has_keyword = false;
    let mut has_search_type = false;

    if let Some(Value::Object(map)) = params {
        // Prefer `keyword`, fallback to `q`
        if let Some(v) = map.get("keyword") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("keyword".to_string(), val));
            has_keyword = true;
        } else if let Some(v) = map.get("q") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("keyword".to_string(), val));
            has_keyword = true;
        }

        // Carry `search_type` if provided
        if let Some(v) = map.get("search_type") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("search_type".to_string(), val));
            has_search_type = true;
        }

        // Include remaining params except `q`, `keyword`, `search_type`
        for (k, v) in map.iter() {
            if k == "q" || k == "keyword" || k == "search_type" { continue; }
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push((k.clone(), val));
        }
    }

    // Default search_type
    if !has_search_type {
        query.push(("search_type".to_string(), "Top".to_string()));
    }

    // If no keyword provided, we still return any extra params; TikHub may error accordingly.
    query
}

// For TikHub TikTok (web), ensure `keyword` is used (map `q` -> `keyword`) and default `count=20`, `offset=0`.
fn tikhub_tiktok_query(params: &Option<Value>) -> Vec<(String, String)> {
    let mut query: Vec<(String, String)> = Vec::new();
    let mut has_keyword = false;
    let mut has_count = false;
    let mut has_offset = false;

    if let Some(Value::Object(map)) = params {
        // Prefer `keyword`, fallback to `q`
        if let Some(v) = map.get("keyword") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("keyword".to_string(), val));
            has_keyword = true;
        } else if let Some(v) = map.get("q") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("keyword".to_string(), val));
            has_keyword = true;
        }

        // Carry `count` if provided
        if let Some(v) = map.get("count") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("count".to_string(), val));
            has_count = true;
        }

        // Carry `offset` if provided
        if let Some(v) = map.get("offset") {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push(("offset".to_string(), val));
            has_offset = true;
        }

        // Include remaining params except these mapped ones
        for (k, v) in map.iter() {
            if k == "q" || k == "keyword" || k == "count" || k == "offset" { continue; }
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            query.push((k.clone(), val));
        }
    }

    // Defaults per curl example
    if !has_count { query.push(("count".to_string(), "20".to_string())); }
    if !has_offset { query.push(("offset".to_string(), "0".to_string())); }

    // If no keyword provided, we still return any extra params; TikHub may error accordingly.
    query
}

async fn execute_request(client: &Client, req: reqwest::RequestBuilder) -> Response {
    match req.send().await {
        Ok(resp) => {
            let status_u16 = resp.status().as_u16();
            let ct = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|x| x.to_str().ok())
                .unwrap_or("")
                .to_string();

            if ct.contains("application/json") {
                match resp.json::<Value>().await {
                    Ok(data) => (StatusCode::OK, Json(json!({"status": status_u16, "data": data}))).into_response(),
                    Err(_) => (StatusCode::OK, Json(json!({"status": status_u16, "data": Value::Null}))).into_response(),
                }
            } else {
                match resp.text().await {
                    Ok(text) => (StatusCode::OK, Json(json!({"status": status_u16, "data_text": text}))).into_response(),
                    Err(_) => (StatusCode::OK, Json(json!({"status": status_u16, "data_text": ""}))).into_response(),
                }
            }
        }
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Request failed: {}", err)}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct TikHubGenericRequest {
    pub service: String, // e.g., "twitter/web" or "tiktok/app/v3"
    pub path: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
}

pub async fn proxy_tikhub_generic(Json(body): Json<TikHubGenericRequest>) -> impl IntoResponse {
    let token = match std::env::var("TIKHUB_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing TIKHUB_TOKEN in environment"})),
            )
                .into_response();
        }
    };

    let service = body.service.trim_matches('/');
    let url = format!("https://api.tikhub.io/api/v1/{}/{}", service, body.path.trim_start_matches('/'));
    let client = Client::new();

    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&params_to_query(&body.params))
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}

pub async fn proxy_tikhub_twitter(Json(body): Json<ProxyRequest>) -> impl IntoResponse {
    let token = match std::env::var("TIKHUB_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing TIKHUB_TOKEN in environment"})),
            )
                .into_response();
        }
    };

    let url = format!("{}{}", TIKHUB_TWITTER_BASE, body.path.trim_start_matches('/'));
    let client = Client::new();

    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&tikhub_twitter_query(&body.params))
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}

pub async fn proxy_tikhub_tiktok(Json(body): Json<ProxyRequest>) -> impl IntoResponse {
    let token = match std::env::var("TIKHUB_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing TIKHUB_TOKEN in environment"})),
            )
                .into_response();
        }
    };

    let url = format!("{}{}", TIKHUB_TIKTOK_BASE, body.path.trim_start_matches('/'));
    let client = Client::new();

    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&tikhub_tiktok_query(&body.params))
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}

pub async fn proxy_rapidapi_instagram(Json(body): Json<ProxyRequest>) -> impl IntoResponse {
    let key = match std::env::var("RAPIDAPI_KEY") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing RAPIDAPI_KEY in environment"})),
            )
                .into_response();
        }
    };

    let url = format!("https://{}/{}", RAPIDAPI_INSTAGRAM_HOST, body.path.trim_start_matches('/'));
    let client = Client::new();
    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&params_to_query(&body.params))
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", RAPIDAPI_INSTAGRAM_HOST)
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", RAPIDAPI_INSTAGRAM_HOST)
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}

pub async fn proxy_rapidapi_twitter_v24(Json(body): Json<ProxyRequest>) -> impl IntoResponse {
    let key = match std::env::var("RAPIDAPI_KEY") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing RAPIDAPI_KEY in environment"})),
            )
                .into_response();
        }
    };

    let url = format!("https://{}/{}", RAPIDAPI_TWITTER_V24_HOST, body.path.trim_start_matches('/'));
    let client = Client::new();
    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&params_to_query(&body.params))
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", RAPIDAPI_TWITTER_V24_HOST)
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", RAPIDAPI_TWITTER_V24_HOST)
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}

#[derive(Deserialize)]
pub struct RapidApiGenericRequest {
    pub host: String,
    pub path: String,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
}

pub async fn proxy_rapidapi_generic(Json(body): Json<RapidApiGenericRequest>) -> impl IntoResponse {
    let key = match std::env::var("RAPIDAPI_KEY") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing RAPIDAPI_KEY in environment"})),
            )
                .into_response();
        }
    };

    let url = format!("https://{}/{}", body.host, body.path.trim_start_matches('/'));
    let client = Client::new();
    let method = body.method.clone().unwrap_or_else(|| "GET".to_string());
    let rb = if method.eq_ignore_ascii_case("GET") {
        client
            .get(&url)
            .query(&params_to_query(&body.params))
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", body.host.clone())
    } else {
        client
            .post(&url)
            .header("accept", "application/json")
            .header("x-rapidapi-key", key.clone())
            .header("x-rapidapi-host", body.host.clone())
            .json(&body.params.unwrap_or(Value::Null))
    };

    execute_request(&client, rb).await
}