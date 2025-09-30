use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use url::Url;
use regex::Regex;

// Add text cleaning functions
fn clean_html_text(html_text: &str) -> String {
    // Simple HTML tag removal
    let re_html = Regex::new(r"<[^>]*>").unwrap_or_else(|_| Regex::new(r"").unwrap());
    let text_without_tags = re_html.replace_all(html_text, " ").to_string();
    
    // Replace HTML entities
    let text_without_entities = text_without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    
    // Normalize whitespace
    let re_whitespace = Regex::new(r"\s+").unwrap_or_else(|_| Regex::new(r"").unwrap());
    let normalized_text = re_whitespace.replace_all(&text_without_entities, " ").to_string();
    
    normalized_text.trim().to_string()
}

fn calculate_relevance_score(keyword: &str, context: &str) -> f32 {
    // Simple relevance scoring based on keyword density
    let context_lower = context.to_lowercase();
    let keyword_lower = keyword.to_lowercase();
    
    // Count occurrences
    let count = context_lower.matches(&keyword_lower).count();
    if count == 0 {
        return 0.0;
    }
    
    // Calculate density (occurrences per 100 characters)
    let density = (count as f32 * 100.0) / context.len() as f32;
    
    // Check if keyword appears in the first third of the context (higher relevance)
    let first_third_len = context.len() / 3;
    let first_third = &context_lower[..first_third_len.min(context_lower.len())];
    let position_boost = if first_third.contains(&keyword_lower) { 0.3 } else { 0.0 };
    
    // Combine factors (density has more weight)
    let score = (density * 0.7) + position_boost;
    
    // Normalize to 0-1 range (capping at 1.0)
    (score * 10.0).min(100.0)
}

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),
    
    #[error("Selector error: {0}")]
    SelectorError(String),
    
    #[error("Timeout error: Crawling exceeded the time limit")]
    TimeoutError,
    
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlResult {
    pub results: Vec<DomainResult>, // Changed to support multiple domains
    pub total_pages_crawled: usize,
    pub total_processing_time_ms: u64,
    pub crawl_timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainResult {
    pub url: String,
    pub title: Option<String>,
    pub matches: Vec<KeywordMatch>,
    pub pages_crawled: usize,
    pub has_more_pages: bool,
    pub metadata: Option<CrawlMetadata>,
    pub error: Option<String>, // To capture domain-specific errors
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlMetadata {
    pub crawl_timestamp: String,
    pub total_processing_time_ms: u64,
    pub content_summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordMatch {
    pub keyword: String,
    pub context: String,
    pub cleaned_text: String,
    pub count: usize,
    pub relevance_score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlRequest {
    pub url: String, // Can contain multiple URLs separated by commas
    pub keywords: Vec<String>,
    pub max_depth: Option<usize>,
    pub max_time_seconds: Option<u64>,
    pub follow_pagination: Option<bool>,
    pub max_pages: Option<usize>,
}

// Helper function to parse multiple URLs from comma-separated string
fn parse_urls(url_string: &str) -> Result<Vec<Url>, CrawlerError> {
    let mut urls = Vec::new();
    
    // Clean the input string by removing backticks and extra whitespace
    let cleaned_input = url_string.trim().replace('`', "");
    
    for url_str in cleaned_input.split(',') {
        let trimmed_url = url_str.trim();
        if !trimmed_url.is_empty() {
            // Additional validation to ensure the URL has a proper scheme
            let url_to_parse = if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
                format!("https://{}", trimmed_url)
            } else {
                trimmed_url.to_string()
            };
            
            match Url::parse(&url_to_parse) {
                Ok(parsed_url) => urls.push(parsed_url),
                Err(e) => {
                    // Log the error but continue processing other URLs
                    eprintln!("Failed to parse URL '{}': {}", trimmed_url, e);
                    continue;
                }
            }
        }
    }
    
    if urls.is_empty() {
        return Err(CrawlerError::Other("No valid URLs provided".to_string()));
    }
    
    Ok(urls)
}

pub async fn crawl_website(request: &CrawlRequest) -> Result<CrawlResult, CrawlerError> {
    let start_processing_time = Instant::now();
    
    // Parse multiple URLs from the comma-separated string
    let urls = parse_urls(&request.url)?;
    
    let mut domain_results = Vec::new();
    let mut total_pages_crawled = 0;
    
    // Process each domain
    for base_url in urls {
        let domain_result = crawl_single_domain(&base_url, request, start_processing_time).await;
        
        match domain_result {
            Ok(mut result) => {
                total_pages_crawled += result.pages_crawled;
                domain_results.push(result);
            }
            Err(err) => {
                // Create an error result for this domain
                let error_result = DomainResult {
                    url: base_url.to_string(),
                    title: None,
                    matches: Vec::new(),
                    pages_crawled: 0,
                    has_more_pages: false,
                    metadata: None,
                    error: Some(err.to_string()),
                };
                domain_results.push(error_result);
            }
        }
    }
    
    // Create metadata
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    
    Ok(CrawlResult {
        results: domain_results,
        total_pages_crawled,
        total_processing_time_ms: start_processing_time.elapsed().as_millis() as u64,
        crawl_timestamp: format!("{}", timestamp),
    })
}

async fn crawl_single_domain(
    base_url: &Url,
    request: &CrawlRequest,
    start_processing_time: Instant,
) -> Result<DomainResult, CrawlerError> {
    let client = Client::new();
    
    // Set up time tracking if max_time_seconds is specified
    let start_time = Instant::now();
    let time_limit = request.max_time_seconds.map(Duration::from_secs);
    
    // Track visited URLs to avoid duplicates
    let mut visited_urls = HashSet::new();
    visited_urls.insert(base_url.to_string());
    
    // Initialize result
    let mut all_matches = Vec::new();
    let mut pages_crawled = 0;
    let mut has_more_pages = false;
    let mut current_url = base_url.clone();
    let mut page_title = None;
    
    // Set max pages to crawl
    let max_pages = request.max_pages.unwrap_or(10);
    
    loop {
        // Check if we've exceeded the time limit
        if let Some(limit) = time_limit {
            if start_time.elapsed() > limit {
                has_more_pages = true;
                break;
            }
        }
        
        // Check if we've reached the max pages
        if pages_crawled >= max_pages {
            has_more_pages = true;
            break;
        }
        
        // Fetch the webpage content
        let response = client.get(current_url.clone()).send().await?;
        let html_content = response.text().await?;
        
        // Parse the HTML
        let document = Html::parse_document(&html_content);
        
        // Extract title (only for the first page)
        if pages_crawled == 0 {
            let title_selector = Selector::parse("title").map_err(|e| CrawlerError::SelectorError(e.to_string()))?;
            page_title = document.select(&title_selector).next().map(|element| element.inner_html());
        }
        
        // Process the current page
        process_page_content(&html_content, &request.keywords, &mut all_matches, time_limit, start_time)?;
        
        pages_crawled += 1;
        
        // If pagination is not enabled, break after the first page
        if !request.follow_pagination.unwrap_or(false) {
            break;
        }
        
        // Try to find pagination links
        if let Some(next_url) = find_next_page_url(&document, &current_url) {
            if !visited_urls.contains(&next_url.to_string()) {
                visited_urls.insert(next_url.to_string());
                current_url = next_url;
            } else {
                // We've already visited this URL, so we're in a loop
                break;
            }
        } else {
            // No more pagination links found
            break;
        }
    }
    
    // Create metadata
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    
    let metadata = CrawlMetadata {
        crawl_timestamp: format!("{}", timestamp),
        total_processing_time_ms: start_processing_time.elapsed().as_millis() as u64,
        content_summary: page_title.clone(),
    };
    
    Ok(DomainResult {
        url: base_url.to_string(),
        title: page_title,
        matches: all_matches,
        pages_crawled,
        has_more_pages,
        metadata: Some(metadata),
        error: None,
    })
}

fn process_page_content(
    html_content: &str,
    keywords: &[String],
    all_matches: &mut Vec<KeywordMatch>,
    time_limit: Option<Duration>,
    start_time: Instant,
) -> Result<Option<String>, CrawlerError> {
    let html_lowercase = html_content.to_lowercase();
    let mut title = None;
    
    // Extract title if this is the first page
    if all_matches.is_empty() {
        let document = Html::parse_document(html_content);
        let title_selector = Selector::parse("title").map_err(|e| CrawlerError::SelectorError(e.to_string()))?;
        title = document.select(&title_selector).next().map(|element| element.inner_html());
    }
    
    for keyword in keywords {
        // Check if we've exceeded the time limit
        if let Some(limit) = time_limit {
            if start_time.elapsed() > limit {
                return Err(CrawlerError::TimeoutError);
            }
        }
        
        let keyword_lowercase = keyword.trim().to_lowercase();
        let count = html_lowercase.matches(&keyword_lowercase).count();
        
        if count > 0 {
            // Extract all contexts around the keyword
            let mut contexts = Vec::new();
            for (i, _) in html_lowercase.match_indices(&keyword_lowercase) {
                // Check time limit again during processing
                if let Some(limit) = time_limit {
                    if start_time.elapsed() > limit {
                        // If we hit the time limit, return what we have so far
                        let context = "Time limit reached during processing".to_string();
                        let cleaned_text = clean_html_text(&context);
                        
                        all_matches.push(KeywordMatch {
                            keyword: keyword.clone(),
                            context,
                            cleaned_text,
                            count,
                            relevance_score: Some(0.0),
                        });
                        
                        return Err(CrawlerError::TimeoutError);
                    }
                }
                
                let start = if i > 50 { i - 50 } else { 0 };
                let end = if i + keyword.len() + 50 < html_content.len() {
                    i + keyword.len() + 50
                } else {
                    html_content.len()
                };
                
                let context = html_content[start..end].to_string();
                contexts.push(context);
            }
            
            // Include all contexts instead of just the first one
            let context = if !contexts.is_empty() {
                contexts.join("\n...\n")
            } else {
                "".to_string()
            };
            
            // Clean the text and calculate relevance score
            let cleaned_text = clean_html_text(&context);
            let relevance_score = calculate_relevance_score(keyword, &context);
            
            all_matches.push(KeywordMatch {
                keyword: keyword.clone(),
                context,
                cleaned_text,
                count,
                relevance_score: Some(relevance_score),
            });
        }
    }
    
    Ok(title)
}

fn find_next_page_url(document: &Html, current_url: &Url) -> Option<Url> {
    // Common pagination selectors
    let pagination_selectors = [
        "a.next", "a.pagination-next", "a[rel='next']", 
        "a:contains('Next')", "a:contains('next')", 
        "a:contains('»')", "a.pagination__next", 
        "li.next a", "div.pagination a:last-child",
        ".pagination a[aria-label='Next']"
    ];
    
    for selector_str in pagination_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(next_link) = document.select(&selector).next() {
                if let Some(href) = next_link.value().attr("href") {
                    // Convert relative URL to absolute
                    if let Ok(next_url) = current_url.join(href) {
                        return Some(next_url);
                    }
                }
            }
        }
    }
    
    None
}