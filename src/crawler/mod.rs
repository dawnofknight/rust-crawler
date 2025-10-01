use spider::website::Website;
use spider::page::Page;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use url::Url;
use std::collections::HashSet;
use regex::Regex;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("URL parsing failed: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("URL error: {0}")]
    UrlError(String),
    #[error("Selector error: {0}")]
    SelectorError(String),
    #[error("Crawling timeout")]
    TimeoutError,
    #[error("Crawling failed: {0}")]
    CrawlError(String),
    #[error("Date parsing failed: {0}")]
    DateParsingError(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlRequest {
    pub url: String,
    pub max_pages: Option<u32>,
    pub max_depth: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub include_subdomains: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrawlResult {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub links: Vec<String>,
    pub status_code: Option<u16>,
    pub crawl_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlResponse {
    pub pages: Vec<CrawlResult>,
    pub total_pages_crawled: usize,
    pub crawl_duration_ms: u64,
    pub errors: Vec<String>,
}

pub async fn crawl_website(request: CrawlRequest) -> Result<CrawlResponse, CrawlerError> {
    let start_time = Instant::now();
    let mut errors = Vec::new();
    
    // Parse and validate URL
    let base_url = Url::parse(&request.url)
        .map_err(|e| CrawlerError::UrlParseError(e))?;
    
    // Create spider website instance
    let mut website = Website::new(&request.url);
    
    // Configure spider settings for better performance and reliability
    website.configuration.subdomains = request.include_subdomains.unwrap_or(false);
    website.configuration.depth = request.max_depth.unwrap_or(3) as usize;
    
    // Set timeout with a reasonable default
    let timeout_seconds = request.timeout_seconds.unwrap_or(60);
    website.configuration.crawl_timeout = Some(Duration::from_secs(timeout_seconds));
    
    // Set user agent for better compatibility
    website.configuration.user_agent = Some(Box::new("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into()));
    
    // Set respect robots.txt
    website.configuration.respect_robots_txt = true;
    
    // Configure for better performance and anti-bot evasion
    website.configuration.delay = 1000; // 1 second delay between requests for better stealth
    website.configuration.request_timeout = Some(Box::new(Duration::from_secs(30))); // 30s per request
    website.configuration.http2_prior_knowledge = false; // Disable HTTP/2 for better compatibility
    
    // Subscribe to crawl events for real-time processing
    let mut rx = website.subscribe(16).unwrap();
    
    // Start crawling in a separate task with timeout
    let crawl_handle = tokio::spawn(async move {
        website.crawl().await;
    });
    
    let mut results = Vec::new();
    let max_pages = request.max_pages.unwrap_or(100) as usize;
    
    // Create a timeout for the entire crawling operation
    let overall_timeout = Duration::from_secs(timeout_seconds + 30); // Add 30s buffer
    let timeout_future = tokio::time::sleep(overall_timeout);
    
    println!("Starting crawl for {} with timeout {}s", request.url, timeout_seconds);
    
    tokio::select! {
        _ = timeout_future => {
            println!("Overall timeout reached for {}", request.url);
            return Err(CrawlerError::TimeoutError);
        }
        _ = async {
            // Process pages as they come in
            while let Ok(page) = rx.recv().await {
                println!("Received page: {}", page.get_url());
                
                if results.len() >= max_pages {
                    println!("Max pages ({}) reached for {}", max_pages, request.url);
                    break;
                }
                
                match process_page(page, &base_url).await {
                    Ok(result) => {
                        println!("Successfully processed page: {}", result.url);
                        results.push(result);
                    }
                    Err(e) => {
                        println!("Error processing page: {}", e);
                        errors.push(e.to_string());
                    }
                }
            }
            
            // Wait for crawl to complete
            println!("Waiting for crawl to complete for {}", request.url);
            
            // Add timeout to the crawl handle to prevent hanging
            let crawl_timeout = Duration::from_secs(timeout_seconds);
            match tokio::time::timeout(crawl_timeout, crawl_handle).await {
                Ok(Ok(())) => {
                    println!("Crawl completed successfully for {}", request.url);
                }
                Ok(Err(e)) => {
                    println!("Crawl handle error for {}: {}", request.url, e);
                    errors.push(format!("Crawl error: {}", e));
                }
                Err(_) => {
                    println!("Crawl handle timed out for {}", request.url);
                    errors.push("Crawl handle timed out".to_string());
                }
            }
            
            println!("Crawl process finished for {}", request.url);
            
            Ok::<(), CrawlerError>(())
        } => {
            println!("Crawl process completed normally for {}", request.url);
        }
    }
    
    let crawl_duration = start_time.elapsed();
    
    Ok(CrawlResponse {
        pages: results.clone(),
        total_pages_crawled: results.len(),
        crawl_duration_ms: crawl_duration.as_millis() as u64,
        errors,
    })
}

async fn process_page(page: Page, base_url: &Url) -> Result<CrawlResult, CrawlerError> {
    let url = page.get_url().to_string();
    let html = page.get_html();
    
    // Extract title
    let title = extract_title(&html);
    
    // Extract text content
    let content = extract_content(&html);
    
    // Extract links
    let links = extract_links(&html, base_url);
    
    Ok(CrawlResult {
        url,
        title,
        content,
        links,
        status_code: None, // spider-rs doesn't directly provide status code
        crawl_time: Some(chrono::Utc::now().to_rfc3339()),
    })
}

fn extract_title(html: &str) -> Option<String> {
    let title_regex = Regex::new(r"<title[^>]*>([^<]*)</title>").ok()?;
    title_regex.captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_content(html: &str) -> String {
    // Remove script and style tags
    let script_regex = Regex::new(r"<script[^>]*>.*?</script>").unwrap();
    let style_regex = Regex::new(r"<style[^>]*>.*?</style>").unwrap();
    let mut content = script_regex.replace_all(html, "").to_string();
    content = style_regex.replace_all(&content, "").to_string();
    
    // Remove HTML tags
    let tag_regex = Regex::new(r"<[^>]*>").unwrap();
    content = tag_regex.replace_all(&content, " ").to_string();
    
    // Clean up whitespace
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    content = whitespace_regex.replace_all(&content, " ").trim().to_string();
    
    content
}

fn extract_links(html: &str, base_url: &Url) -> Vec<String> {
    let mut links = Vec::new();
    let link_regex = Regex::new(r#"<a[^>]+href\s*=\s*["']([^"']+)["'][^>]*>"#).unwrap();
    
    for cap in link_regex.captures_iter(html) {
        if let Some(href) = cap.get(1) {
            let href_str = href.as_str();
            
            // Skip non-HTTP links
            if href_str.starts_with("mailto:") || href_str.starts_with("tel:") || href_str.starts_with("javascript:") {
                continue;
            }
            
            // Convert relative URLs to absolute
            if let Ok(absolute_url) = base_url.join(href_str) {
                let url_str = absolute_url.to_string();
                if url_str.starts_with("http://") || url_str.starts_with("https://") {
                    links.push(url_str);
                }
            }
        }
    }
    
    // Remove duplicates
    let mut unique_links: Vec<String> = links.into_iter().collect::<HashSet<_>>().into_iter().collect();
    unique_links.sort();
    unique_links
}

pub async fn crawl_subdomains(
    domain: &str,
    max_pages: Option<u32>,
    timeout_seconds: Option<u64>,
) -> Result<CrawlResponse, CrawlerError> {
    let request = CrawlRequest {
        url: format!("https://{}", domain),
        max_pages,
        max_depth: Some(2),
        timeout_seconds,
        include_subdomains: Some(true),
    };
    
    crawl_website(request).await
}