use anyhow::{anyhow, Result};
use async_trait::async_trait;
use colored::Colorize;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use url::Url;

/// Enhanced configuration for intelligent web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedWebSearchConfig {
    // Core settings
    pub timeout_seconds: u64,
    pub max_results_per_engine: usize,
    pub max_total_results: usize,
    pub max_content_length: usize,
    pub max_scrape_urls: usize,
    
    // User agent and request settings
    pub user_agent: String,
    pub retry_attempts: usize,
    pub retry_delay_ms: u64,
    pub follow_redirects: bool,
    pub respect_robots_txt: bool,
    
    // Content and intelligence settings
    pub enable_content_extraction: bool,
    pub enable_semantic_ranking: bool,
    pub enable_query_expansion: bool,
    pub enable_result_diversification: bool,
    
    // Performance settings
    pub cache_results: bool,
    pub cache_duration_hours: u64,
    pub concurrent_engines: usize,
    pub adaptive_timeouts: bool,
    
    // Quality filters
    pub min_content_quality_score: f64,
    pub min_relevance_threshold: f64,
    pub exclude_low_authority_domains: bool,
}

impl Default for EnhancedWebSearchConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 20,
            max_results_per_engine: 8,
            max_total_results: 15,
            max_content_length: 5000,
            max_scrape_urls: 8,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            retry_attempts: 3,
            retry_delay_ms: 800,
            follow_redirects: true,
            respect_robots_txt: false,
            enable_content_extraction: true,
            enable_semantic_ranking: true,
            enable_query_expansion: true,
            enable_result_diversification: true,
            cache_results: true,
            cache_duration_hours: 6,
            concurrent_engines: 6,
            adaptive_timeouts: true,
            min_content_quality_score: 0.3,
            min_relevance_threshold: 0.2,
            exclude_low_authority_domains: true,
        }
    }
}

/// Represents different types of search queries for intelligent handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryIntent {
    Factual,        // "What is...", "Define...", "How many..."
    Tutorial,       // "How to...", "Guide to...", "Tutorial..."
    Comparison,     // "vs", "compared to", "difference between"
    Technical,      // Programming, code, APIs, documentation
    News,          // Current events, recent news
    Academic,      // Research papers, scholarly articles
    Shopping,      // Product searches, reviews, prices
    Local,         // Location-based queries
    General,       // Catch-all for other queries
}

/// Enhanced search result with richer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSearchResult {
    // Core result data
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
    pub content: Option<String>,
    
    // Source and scoring
    pub source: String,
    pub relevance_score: f64,
    pub authority_score: f64,
    pub quality_score: f64,
    pub diversity_score: f64,
    pub final_score: f64,
    
    // Enhanced metadata
    pub query_intent: QueryIntent,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub word_count: Option<usize>,
    pub reading_time: Option<u32>,
    pub freshness_score: f64,
    pub social_signals: Option<SocialMetrics>,
    pub technical_metrics: Option<TechnicalMetrics>,
    
    // Timestamps and tracking
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub extraction_time: Option<Duration>,
    pub processing_time: Option<Duration>,
}

/// Social media and engagement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMetrics {
    pub estimated_shares: Option<u64>,
    pub backlink_count: Option<u64>,
    pub domain_authority: Option<f64>,
    pub trust_signals: Vec<String>,
}

/// Technical and performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalMetrics {
    pub https_enabled: bool,
    pub mobile_friendly: Option<bool>,
    pub load_speed_estimate: Option<f64>,
    pub accessibility_score: Option<f64>,
    pub structured_data: bool,
}

/// Trait for search engine implementations
#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>>;
    fn get_name(&self) -> &str;
    fn get_priority(&self) -> u8; // 1-10, higher is better
    fn supports_intent(&self, intent: &QueryIntent) -> bool;
    fn get_rate_limit_delay(&self) -> Duration;
}

/// DuckDuckGo search engine implementation
pub struct DuckDuckGoEngine {
    client: Arc<Client>,
}

impl DuckDuckGoEngine {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
    
    fn get_intent_specific_selectors(&self, intent: &QueryIntent) -> Vec<&'static str> {
        match intent {
            QueryIntent::Technical => vec![
                ".result .result__title a",
                ".web-result .result__title a",
                "h2.result__title a",
            ],
            QueryIntent::News => vec![
                ".news-result .result__title a",
                ".result .result__title a",
            ],
            _ => vec![
                ".result .result__title a",
                ".results_links .result__title a",
                ".web-result .result__title a",
                "h2.result__title a",
                ".result__title a",
            ]
        }
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoEngine {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>> {
        let enhanced_query = enhance_query_for_intent(query, intent);
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&enhanced_query));
        
        let response = fetch_with_intelligent_retry(&self.client, &url, config).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let mut results = Vec::new();
        let selectors = self.get_intent_specific_selectors(intent);
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(config.max_results_per_engine * 2) {
                    if let Some(href) = element.value().attr("href") {
                        let title = extract_clean_text(&element.text().collect::<String>());
                        
                        if let Some(clean_url) = clean_duckduckgo_url(href) {
                            if is_quality_result(&title, &clean_url, config) {
                                let snippet = extract_intelligent_snippet(&document, element, intent);
                                
                                results.push(EnhancedSearchResult {
                                    title: title.clone(),
                                    url: clean_url.clone(),
                                    snippet,
                                    content: None,
                                    source: "DuckDuckGo".to_string(),
                                    relevance_score: calculate_semantic_relevance(&title, query, intent),
                                    authority_score: calculate_context_aware_authority(&extract_domain(&clean_url).unwrap_or_default(), intent, query),
                                    quality_score: 0.0, // Will be calculated later
                                    diversity_score: 0.0, // Will be calculated later
                                    final_score: 0.0, // Will be calculated later
                                    query_intent: intent.clone(),
                                    content_type: infer_content_type(&clean_url, &title),
                                    language: Some("en".to_string()),
                                    word_count: None,
                                    reading_time: None,
                                    freshness_score: calculate_freshness_score(&clean_url),
                                    social_signals: None,
                                    technical_metrics: Some(TechnicalMetrics {
                                        https_enabled: clean_url.starts_with("https://"),
                                        mobile_friendly: None,
                                        load_speed_estimate: None,
                                        accessibility_score: None,
                                        structured_data: false,
                                    }),
                                    timestamp: chrono::Utc::now(),
                                    extraction_time: None,
                                    processing_time: None,
                                });
                            }
                        }
                    }
                }
                
                if !results.is_empty() {
                    break; // Found results with this selector
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_name(&self) -> &str { "DuckDuckGo" }
    fn get_priority(&self) -> u8 { 8 }
    fn supports_intent(&self, _intent: &QueryIntent) -> bool { true }
    fn get_rate_limit_delay(&self) -> Duration { Duration::from_millis(500) }
}

/// Bing search engine implementation
pub struct BingEngine {
    client: Arc<Client>,
}

impl BingEngine {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for BingEngine {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>> {
        let enhanced_query = enhance_query_for_intent(query, intent);
        let url = format!("https://www.bing.com/search?q={}", urlencoding::encode(&enhanced_query));
        
        let response = fetch_with_intelligent_retry(&self.client, &url, config).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let mut results = Vec::new();
        let selectors = [
            "li.b_algo h2 a",
            ".b_algo .b_title a", 
            "h2 a[href^='http']",
            ".b_title a[href^='http']",
            ".b_algo a[href^='http']",
        ];
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(config.max_results_per_engine * 2) {
                    if let Some(href) = element.value().attr("href") {
                        let title = extract_clean_text(&element.text().collect::<String>());
                        
                        if is_quality_result(&title, href, config) && href.starts_with("http") {
                            let snippet = extract_intelligent_snippet(&document, element, intent);
                            
                            results.push(EnhancedSearchResult {
                                title: title.clone(),
                                url: href.to_string(),
                                snippet,
                                content: None,
                                source: "Bing".to_string(),
                                relevance_score: calculate_semantic_relevance(&title, query, intent),
                                authority_score: calculate_context_aware_authority(&extract_domain(href).unwrap_or_default(), intent, query),
                                quality_score: 0.0,
                                diversity_score: 0.0,
                                final_score: 0.0,
                                query_intent: intent.clone(),
                                content_type: infer_content_type(href, &title),
                                language: Some("en".to_string()),
                                word_count: None,
                                reading_time: None,
                                freshness_score: calculate_freshness_score(href),
                                social_signals: None,
                                technical_metrics: Some(TechnicalMetrics {
                                    https_enabled: href.starts_with("https://"),
                                    mobile_friendly: None,
                                    load_speed_estimate: None,
                                    accessibility_score: None,
                                    structured_data: false,
                                }),
                                timestamp: chrono::Utc::now(),
                                extraction_time: None,
                                processing_time: None,
                            });
                        }
                    }
                }
                
                if !results.is_empty() {
                    break;
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_name(&self) -> &str { "Bing" }
    fn get_priority(&self) -> u8 { 7 }
    fn supports_intent(&self, _intent: &QueryIntent) -> bool { true }
    fn get_rate_limit_delay(&self) -> Duration { Duration::from_millis(600) }
}

/// Wikipedia specialized engine
pub struct WikipediaEngine {
    client: Arc<Client>,
}

impl WikipediaEngine {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
    
    /// Determine if Wikipedia is appropriate for this query
    fn is_suitable_for_query(&self, query: &str, intent: &QueryIntent) -> bool {
        let query_lower = query.to_lowercase();
        
        // Wikipedia is NOT suitable for:
        // 1. Current events, news, breaking information
        if query_lower.contains("latest") || query_lower.contains("recent") || 
           query_lower.contains("breaking") || query_lower.contains("news") ||
           query_lower.contains("2024") || query_lower.contains("today") ||
           query_lower.contains("currently") || query_lower.contains("now") {
            return false;
        }
        
        // 2. Technical implementation details that change frequently
        if (query_lower.contains("api") && query_lower.contains("how to")) ||
           query_lower.contains("tutorial") || query_lower.contains("installation") ||
           query_lower.contains("setup") || query_lower.contains("configure") {
            return false;
        }
        
        // 3. Shopping, pricing, product reviews
        if matches!(intent, QueryIntent::Shopping) {
            return false;
        }
        
        // 4. Local/location-specific queries
        if matches!(intent, QueryIntent::Local) {
            return false;
        }
        
        // 5. Highly technical programming questions
        if matches!(intent, QueryIntent::Technical) && 
           (query_lower.contains("error") || query_lower.contains("bug") || 
            query_lower.contains("implementation") || query_lower.contains("code")) {
            return false;
        }
        
        // Wikipedia IS suitable for:
        // 1. General factual information
        // 2. Historical information
        // 3. Definitions and concepts
        // 4. Academic/scientific topics (but not latest research)
        matches!(intent, QueryIntent::Factual | QueryIntent::General | QueryIntent::Academic) ||
        query_lower.contains("history") || query_lower.contains("definition") ||
        query_lower.contains("concept") || query_lower.contains("theory")
    }
}

#[async_trait]
impl SearchEngine for WikipediaEngine {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>> {
        // Check if Wikipedia is suitable for this query
        if !self.is_suitable_for_query(query, intent) {
            return Ok(Vec::new());
        }
        
        let search_url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&format=json&list=search&srsearch={}&srlimit={}",
            urlencoding::encode(query),
            config.max_results_per_engine
        );
        
        let response = fetch_with_intelligent_retry(&self.client, &search_url, config).await?;
        let json: serde_json::Value = response.json().await?;
        
        let mut results = Vec::new();
        
        if let Some(search_results) = json["query"]["search"].as_array() {
            for item in search_results.iter().take(config.max_results_per_engine) {
                if let (Some(title), Some(_pageid)) = (item["title"].as_str(), item["pageid"].as_u64()) {
                    let url = format!("https://en.wikipedia.org/wiki/{}", urlencoding::encode(title));
                    let snippet = item["snippet"].as_str()
                        .map(|s| clean_html_entities(s))
                        .filter(|s| !s.is_empty());
                    
                    results.push(EnhancedSearchResult {
                        title: title.to_string(),
                        url,
                        snippet,
                        content: None,
                        source: "Wikipedia".to_string(),
                        relevance_score: calculate_semantic_relevance(title, query, intent),
                        authority_score: calculate_context_aware_authority("wikipedia.org", intent, query),
                        quality_score: 0.9,
                        diversity_score: 0.0,
                        final_score: 0.0,
                        query_intent: intent.clone(),
                        content_type: Some("encyclopedia".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                        reading_time: None,
                        freshness_score: 0.8, // Wikipedia is well-maintained
                        social_signals: Some(SocialMetrics {
                            estimated_shares: Some(1000), // Wikipedia articles are widely shared
                            backlink_count: None,
                            domain_authority: Some(0.98),
                            trust_signals: vec!["verified".to_string(), "collaborative".to_string()],
                        }),
                        technical_metrics: Some(TechnicalMetrics {
                            https_enabled: true,
                            mobile_friendly: Some(true),
                            load_speed_estimate: Some(0.8),
                            accessibility_score: Some(0.9),
                            structured_data: true,
                        }),
                        timestamp: chrono::Utc::now(),
                        extraction_time: None,
                        processing_time: None,
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_name(&self) -> &str { "Wikipedia" }
    fn get_priority(&self) -> u8 { 9 }
    fn supports_intent(&self, _intent: &QueryIntent) -> bool {
        // Wikipedia support is now determined dynamically in the search method
        true
    }
    fn get_rate_limit_delay(&self) -> Duration { Duration::from_millis(200) }
}

/// Stack Overflow specialized engine for technical queries
pub struct StackOverflowEngine {
    client: Arc<Client>,
}

impl StackOverflowEngine {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for StackOverflowEngine {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>> {
        // Stack Overflow is excellent for technical queries
        if !matches!(intent, QueryIntent::Technical | QueryIntent::Tutorial) {
            return Ok(Vec::new());
        }
        
        let url = format!("https://stackoverflow.com/search?q={}", urlencoding::encode(query));
        
        let response = fetch_with_intelligent_retry(&self.client, &url, config).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let mut results = Vec::new();
        let question_selector = Selector::parse(".question-summary .question-hyperlink")
            .map_err(|e| anyhow!("Invalid selector: {}", e))?;
        
        for (i, element) in document.select(&question_selector).enumerate() {
            if i >= config.max_results_per_engine { break; }
            
            if let Some(href) = element.value().attr("href") {
                let title = extract_clean_text(&element.text().collect::<String>());
                let full_url = if href.starts_with("/") {
                    format!("https://stackoverflow.com{}", href)
                } else {
                    href.to_string()
                };
                
                if is_quality_result(&title, &full_url, config) {
                    let snippet = extract_intelligent_snippet(&document, element, intent);
                    
                    results.push(EnhancedSearchResult {
                        title: title.clone(),
                        url: full_url.clone(),
                        snippet,
                        content: None,
                        source: "Stack Overflow".to_string(),
                        relevance_score: calculate_semantic_relevance(&title, query, intent),
                        authority_score: calculate_context_aware_authority("stackoverflow.com", intent, query),
                        quality_score: 0.0,
                        diversity_score: 0.0,
                        final_score: 0.0,
                        query_intent: intent.clone(),
                        content_type: Some("qa_forum".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                        reading_time: None,
                        freshness_score: calculate_freshness_score(&full_url),
                        social_signals: None,
                        technical_metrics: Some(TechnicalMetrics {
                            https_enabled: full_url.starts_with("https://"),
                            mobile_friendly: Some(true),
                            load_speed_estimate: Some(0.7),
                            accessibility_score: Some(0.8),
                            structured_data: true,
                        }),
                        timestamp: chrono::Utc::now(),
                        extraction_time: None,
                        processing_time: None,
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_name(&self) -> &str { "Stack Overflow" }
    fn get_priority(&self) -> u8 { 9 } // Very high for technical queries
    fn supports_intent(&self, intent: &QueryIntent) -> bool {
        matches!(intent, QueryIntent::Technical | QueryIntent::Tutorial)
    }
    fn get_rate_limit_delay(&self) -> Duration { Duration::from_millis(800) }
}

/// Reddit specialized engine for discussions and current topics
pub struct RedditEngine {
    client: Arc<Client>,
}

impl RedditEngine {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchEngine for RedditEngine {
    async fn search(&self, query: &str, intent: &QueryIntent, config: &EnhancedWebSearchConfig) -> Result<Vec<EnhancedSearchResult>> {
        // Reddit is excellent for current discussions, opinions, and recent topics
        if !matches!(intent, QueryIntent::News | QueryIntent::General | QueryIntent::Comparison) {
            return Ok(Vec::new());
        }
        
        let query_lower = query.to_lowercase();
        // Prioritize Reddit for queries about current events or discussions
        if !query_lower.contains("latest") && !query_lower.contains("opinion") && 
           !query_lower.contains("discussion") && !query_lower.contains("vs") && 
           !query_lower.contains("experience") && !query_lower.contains("reddit") {
            return Ok(Vec::new());
        }
        
        let url = format!("https://www.reddit.com/search/?q={}&type=link&sort=relevance", urlencoding::encode(query));
        
        let response = fetch_with_intelligent_retry(&self.client, &url, config).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let mut results = Vec::new();
        // Reddit's new layout selectors
        let selectors = [
            "a[data-testid='post-title']",
            ".Post h3 a",
            "[data-click-id='body'] h3 a",
        ];
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for (i, element) in document.select(&selector).enumerate() {
                    if i >= config.max_results_per_engine { break; }
                    
                    if let Some(href) = element.value().attr("href") {
                        let title = extract_clean_text(&element.text().collect::<String>());
                        let full_url = if href.starts_with("/") {
                            format!("https://www.reddit.com{}", href)
                        } else {
                            href.to_string()
                        };
                        
                        if is_quality_result(&title, &full_url, config) {
                            let snippet = extract_intelligent_snippet(&document, element, intent);
                            
                            results.push(EnhancedSearchResult {
                                title: title.clone(),
                                url: full_url.clone(),
                                snippet,
                                content: None,
                                source: "Reddit".to_string(),
                                relevance_score: calculate_semantic_relevance(&title, query, intent),
                                authority_score: calculate_context_aware_authority("reddit.com", intent, query),
                                quality_score: 0.0,
                                diversity_score: 0.0,
                                final_score: 0.0,
                                query_intent: intent.clone(),
                                content_type: Some("discussion".to_string()),
                                language: Some("en".to_string()),
                                word_count: None,
                                reading_time: None,
                                freshness_score: calculate_freshness_score(&full_url),
                                social_signals: Some(SocialMetrics {
                                    estimated_shares: Some(100),
                                    backlink_count: None,
                                    domain_authority: Some(0.85),
                                    trust_signals: vec!["community_moderated".to_string()],
                                }),
                                technical_metrics: Some(TechnicalMetrics {
                                    https_enabled: full_url.starts_with("https://"),
                                    mobile_friendly: Some(true),
                                    load_speed_estimate: Some(0.6),
                                    accessibility_score: Some(0.7),
                                    structured_data: false,
                                }),
                                timestamp: chrono::Utc::now(),
                                extraction_time: None,
                                processing_time: None,
                            });
                        }
                    }
                }
                
                if !results.is_empty() {
                    break;
                }
            }
        }
        
        Ok(results)
    }
    
    fn get_name(&self) -> &str { "Reddit" }
    fn get_priority(&self) -> u8 { 6 } // Medium-high priority for current discussions
    fn supports_intent(&self, intent: &QueryIntent) -> bool {
        matches!(intent, QueryIntent::News | QueryIntent::General | QueryIntent::Comparison)
    }
    fn get_rate_limit_delay(&self) -> Duration { Duration::from_millis(1000) }
}

/// Main enhanced web search orchestrator
pub struct EnhancedWebSearchEngine {
    engines: Vec<Box<dyn SearchEngine>>,
    client: Arc<Client>,
    config: Arc<EnhancedWebSearchConfig>,
    cache: Arc<tokio::sync::RwLock<HashMap<String, (Vec<EnhancedSearchResult>, Instant)>>>,
}

impl EnhancedWebSearchEngine {
    pub fn new(config: EnhancedWebSearchConfig) -> Self {
        let client = Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(config.timeout_seconds))
                .user_agent(&config.user_agent)
                .build()
                .expect("Failed to create HTTP client")
        );
        
        let mut engines: Vec<Box<dyn SearchEngine>> = Vec::new();
        engines.push(Box::new(DuckDuckGoEngine::new(client.clone())));
        engines.push(Box::new(BingEngine::new(client.clone())));
        engines.push(Box::new(WikipediaEngine::new(client.clone())));
        engines.push(Box::new(StackOverflowEngine::new(client.clone())));
        engines.push(Box::new(RedditEngine::new(client.clone())));
        
        Self {
            engines,
            client,
            config: Arc::new(config),
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Intelligent search with intent recognition and adaptive processing
    pub async fn intelligent_search(&self, query: &str) -> Result<Vec<EnhancedSearchResult>> {
        let start_time = Instant::now();
        
        // Step 1: Classify query intent
        let intent = classify_query_intent(query);
        println!("{} Classified query intent: {:?}", "🧠".cyan(), intent);
        
        // Step 2: Check cache if enabled
        if self.config.cache_results {
            let cache_key = format!("{}:{:?}", query, intent);
            if let Some(cached_results) = self.get_cached_results(&cache_key).await {
                println!("{} Using cached results", "💾".green());
                return Ok(cached_results);
            }
        }
        
        // Step 3: Expand query if enabled
        let enhanced_query = if self.config.enable_query_expansion {
            expand_query(query, &intent)
        } else {
            query.to_string()
        };
        
        println!("{} Enhanced query: {}", "✨".yellow(), enhanced_query);
        
        // Step 4: Select and execute search engines based on intent
        let suitable_engines: Vec<_> = self.engines.iter()
            .filter(|engine| engine.supports_intent(&intent))
            .collect();
        
        if suitable_engines.is_empty() {
            return Err(anyhow!("No suitable search engines for intent: {:?}", intent));
        }
        
        // Step 5: Execute searches concurrently with intelligent load balancing
        let search_futures: Vec<_> = suitable_engines.into_iter()
            .take(self.config.concurrent_engines)
            .map(|engine| {
                let query = enhanced_query.clone();
                let intent = intent.clone();
                let config = self.config.clone();
                async move {
                    let engine_start = Instant::now();
                    tokio::time::sleep(engine.get_rate_limit_delay()).await;
                    
                    match engine.search(&query, &intent, &config).await {
                        Ok(mut results) => {
                            let engine_time = engine_start.elapsed();
                            for result in &mut results {
                                result.processing_time = Some(engine_time);
                            }
                            println!("{} {} found {} results in {:?}", 
                                "✓".green(), engine.get_name(), results.len(), engine_time);
                            Ok((engine.get_name(), results))
                        }
                        Err(e) => {
                            println!("{} {} failed: {}", "✗".red(), engine.get_name(), e);
                            Err(e)
                        }
                    }
                }
            })
            .collect();
        
        let search_results = futures::future::join_all(search_futures).await;
        
        // Step 6: Aggregate and process results
        let mut all_results = Vec::new();
        for result in search_results {
            if let Ok((_engine_name, mut results)) = result {
                if !results.is_empty() {
                    all_results.append(&mut results);
                }
            }
        }
        
        if all_results.is_empty() {
            return Err(anyhow!("No search results found from any engine"));
        }
        
        // Step 7: Intelligent processing pipeline
        let processed_results = self.intelligent_processing_pipeline(all_results, &enhanced_query, &intent).await;
        
        // Step 8: Cache results if enabled
        if self.config.cache_results {
            let cache_key = format!("{}:{:?}", query, intent);
            self.cache_results(&cache_key, &processed_results).await;
        }
        
        let total_time = start_time.elapsed();
        println!("{} Search completed in {:?} with {} final results", 
            "🎯".green(), total_time, processed_results.len());
        
        Ok(processed_results)
    }
    
    /// Intelligent processing pipeline for results
    async fn intelligent_processing_pipeline(
        &self, 
        mut results: Vec<EnhancedSearchResult>, 
        query: &str, 
        intent: &QueryIntent
    ) -> Vec<EnhancedSearchResult> {
        
        // Step 1: Advanced deduplication with fuzzy matching
        results = advanced_deduplication(results);
        
        // Step 2: Enhanced scoring with multiple factors
        self.calculate_enhanced_scores(&mut results, query, intent).await;
        
        // Step 3: Quality filtering
        results.retain(|r| r.quality_score >= self.config.min_content_quality_score);
        results.retain(|r| r.relevance_score >= self.config.min_relevance_threshold);
        
        // Step 4: Diversification if enabled
        if self.config.enable_result_diversification {
            results = diversify_results(results, &self.config);
        }
        
        // Step 5: Final ranking
        results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Step 6: Content extraction for top results
        let top_results: Vec<_> = results.into_iter().take(self.config.max_total_results).collect();
        let final_results = self.extract_content_intelligently(top_results).await;
        
        final_results
    }
    
    /// Calculate enhanced scores using multiple factors
    async fn calculate_enhanced_scores(
        &self, 
        results: &mut [EnhancedSearchResult], 
        _query: &str, 
        intent: &QueryIntent
    ) {
        for result in results.iter_mut() {
            // Semantic relevance (already calculated)
            let relevance = result.relevance_score;
            
            // Enhanced authority scoring
            let authority = calculate_enhanced_authority(&result.url);
            
            // Quality scoring based on multiple factors
            let quality = calculate_content_quality_score(result);
            
            // Intent-specific boosting
            let intent_boost = calculate_intent_specific_boost(result, intent);
            
            // Freshness scoring
            let freshness = result.freshness_score;
            
            // Final composite score
            result.quality_score = quality;
            result.authority_score = authority;
            result.final_score = (relevance * 0.35) + 
                                (authority * 0.25) + 
                                (quality * 0.20) + 
                                (intent_boost * 0.15) + 
                                (freshness * 0.05);
        }
    }
    
    /// Extract content intelligently with adaptive strategies
    async fn extract_content_intelligently(&self, mut results: Vec<EnhancedSearchResult>) -> Vec<EnhancedSearchResult> {
        let content_futures: Vec<_> = results.iter()
            .take(self.config.max_scrape_urls)
            .map(|result| {
                let client = self.client.clone();
                let config = self.config.clone();
                let url = result.url.clone();
                let intent = result.query_intent.clone();
                
                async move {
                    extract_content_with_adaptive_strategy(&client, &url, &intent, &config).await
                }
            })
            .collect();
        
        let content_results = futures::future::join_all(content_futures).await;
        
        for (index, content_result) in content_results.into_iter().enumerate() {
            if index < results.len() {
                match content_result {
                    Ok(content) => {
                        if is_high_quality_content(&content, &self.config) {
                            results[index].content = Some(content);
                            results[index].word_count = results[index].content.as_ref()
                                .map(|c| c.split_whitespace().count());
                            results[index].reading_time = results[index].word_count
                                .map(|wc| (wc as f64 / 200.0).ceil() as u32); // ~200 WPM
                            
                            println!("{} Extracted content from: {}", "📄".cyan(), results[index].url);
                        } else {
                            println!("{} Low quality content from: {}", "⚠".yellow(), results[index].url);
                        }
                    }
                    Err(e) => {
                        println!("{} Failed to extract content from {}: {}", "✗".red(), results[index].url, e);
                    }
                }
            }
        }
        
        results
    }
    
    /// Get cached results if available and not expired
    async fn get_cached_results(&self, cache_key: &str) -> Option<Vec<EnhancedSearchResult>> {
        let cache = self.cache.read().await;
        if let Some((results, timestamp)) = cache.get(cache_key) {
            let age = timestamp.elapsed();
            if age < Duration::from_secs(self.config.cache_duration_hours * 3600) {
                return Some(results.clone());
            }
        }
        None
    }
    
    /// Cache search results
    async fn cache_results(&self, cache_key: &str, results: &[EnhancedSearchResult]) {
        let mut cache = self.cache.write().await;
        cache.insert(cache_key.to_string(), (results.to_vec(), Instant::now()));
        
        // Clean up old entries (simple LRU-style cleanup)
        if cache.len() > 100 {
            let cutoff = Instant::now() - Duration::from_secs(self.config.cache_duration_hours * 3600);
            cache.retain(|_, (_, timestamp)| *timestamp > cutoff);
        }
    }
}

// ============ Helper Functions ============

/// Classify the intent of a search query
fn classify_query_intent(query: &str) -> QueryIntent {
    let query_lower = query.to_lowercase();
    
    // Factual queries
    if query_lower.starts_with("what is") || query_lower.starts_with("define") || 
       query_lower.contains("definition") || query_lower.starts_with("who is") ||
       query_lower.starts_with("where is") || query_lower.starts_with("when") {
        return QueryIntent::Factual;
    }
    
    // Tutorial queries
    if query_lower.starts_with("how to") || query_lower.contains("tutorial") ||
       query_lower.contains("guide") || query_lower.contains("learn") ||
       query_lower.starts_with("how do") {
        return QueryIntent::Tutorial;
    }
    
    // Comparison queries
    if query_lower.contains(" vs ") || query_lower.contains(" versus ") ||
       query_lower.contains("compared to") || query_lower.contains("difference between") ||
       query_lower.contains("compare") {
        return QueryIntent::Comparison;
    }
    
    // Technical queries
    if query_lower.contains("api") || query_lower.contains("documentation") ||
       query_lower.contains("programming") || query_lower.contains("code") ||
       query_lower.contains("github") || query_lower.contains("stackoverflow") ||
       query_lower.contains("library") || query_lower.contains("framework") {
        return QueryIntent::Technical;
    }
    
    // News queries
    if query_lower.contains("news") || query_lower.contains("latest") ||
       query_lower.contains("recent") || query_lower.contains("today") ||
       query_lower.contains("2024") || query_lower.contains("breaking") {
        return QueryIntent::News;
    }
    
    // Academic queries
    if query_lower.contains("research") || query_lower.contains("study") ||
       query_lower.contains("paper") || query_lower.contains("journal") ||
       query_lower.contains("academic") || query_lower.contains("scholar") {
        return QueryIntent::Academic;
    }
    
    // Shopping queries
    if query_lower.contains("buy") || query_lower.contains("price") ||
       query_lower.contains("review") || query_lower.contains("product") ||
       query_lower.contains("store") || query_lower.contains("shop") {
        return QueryIntent::Shopping;
    }
    
    // Local queries
    if query_lower.contains("near me") || query_lower.contains("location") ||
       query_lower.contains("address") || query_lower.contains("map") ||
       query_lower.contains("directions") {
        return QueryIntent::Local;
    }
    
    QueryIntent::General
}

/// Enhance query based on intent
fn enhance_query_for_intent(query: &str, intent: &QueryIntent) -> String {
    match intent {
        QueryIntent::Technical => {
            if !query.contains("documentation") && !query.contains("tutorial") {
                format!("{} documentation tutorial", query)
            } else {
                query.to_string()
            }
        }
        QueryIntent::Academic => {
            if !query.contains("research") && !query.contains("study") {
                format!("{} research study", query)
            } else {
                query.to_string()
            }
        }
        QueryIntent::News => {
            if !query.contains("2024") && !query.contains("latest") {
                format!("{} latest 2024", query)
            } else {
                query.to_string()
            }
        }
        _ => query.to_string()
    }
}

/// Expand query with related terms
fn expand_query(query: &str, intent: &QueryIntent) -> String {
    let expansions = match intent {
        QueryIntent::Technical => vec!["documentation", "examples", "guide"],
        QueryIntent::Academic => vec!["research", "study", "analysis"],
        QueryIntent::Tutorial => vec!["tutorial", "how-to", "step-by-step"],
        QueryIntent::Factual => vec!["explanation", "definition", "overview"],
        _ => vec![]
    };
    
    if expansions.is_empty() {
        query.to_string()
    } else {
        format!("{} {}", query, expansions.join(" "))
    }
}

/// Fetch with intelligent retry and adaptive timeouts
async fn fetch_with_intelligent_retry(
    client: &Client, 
    url: &str, 
    config: &EnhancedWebSearchConfig
) -> Result<reqwest::Response> {
    let mut last_error = None;
    let base_delay = Duration::from_millis(config.retry_delay_ms);
    
    for attempt in 0..=config.retry_attempts {
        let timeout_duration = if config.adaptive_timeouts {
            Duration::from_secs(config.timeout_seconds + (attempt as u64 * 2))
        } else {
            Duration::from_secs(config.timeout_seconds)
        };
        
        match timeout(timeout_duration, client.get(url).send()).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    return Ok(response);
                } else {
                    last_error = Some(anyhow!("HTTP error: {}", response.status()));
                }
            }
            Ok(Err(e)) => last_error = Some(anyhow!("Request error: {}", e)),
            Err(_) => last_error = Some(anyhow!("Request timeout after {:?}", timeout_duration)),
        }
        
        if attempt < config.retry_attempts {
            let delay = base_delay * (2_u32.pow(attempt as u32)); // Exponential backoff
            tokio::time::sleep(delay).await;
        }
    }
    
    Err(last_error.unwrap_or_else(|| anyhow!("Unknown fetch error")))
}

/// Calculate semantic relevance with intent awareness
fn calculate_semantic_relevance(title: &str, query: &str, intent: &QueryIntent) -> f64 {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();
    let query_words: HashSet<_> = query_lower.split_whitespace().collect();
    let title_words: HashSet<_> = title_lower.split_whitespace().collect();
    
    // Base word overlap score
    let overlap = query_words.intersection(&title_words).count();
    let base_score = if query_words.is_empty() { 0.0 } else { overlap as f64 / query_words.len() as f64 };
    
    // Intent-specific scoring adjustments
    let intent_multiplier = match intent {
        QueryIntent::Tutorial => {
            if title_lower.contains("tutorial") || title_lower.contains("how to") || title_lower.contains("guide") {
                1.3
            } else { 1.0 }
        }
        QueryIntent::Technical => {
            if title_lower.contains("documentation") || title_lower.contains("api") || title_lower.contains("reference") {
                1.2
            } else { 1.0 }
        }
        QueryIntent::Academic => {
            if title_lower.contains("research") || title_lower.contains("study") || title_lower.contains("analysis") {
                1.2
            } else { 1.0 }
        }
        _ => 1.0
    };
    
    // Exact phrase bonus
    let phrase_bonus = if title_lower.contains(&query_lower) { 0.3 } else { 0.0 };
    
    ((base_score * intent_multiplier) + phrase_bonus).min(1.0)
}

/// Calculate context-aware authority score based on domain, intent, and query
fn calculate_context_aware_authority(domain: &str, intent: &QueryIntent, query: &str) -> f64 {
    let domain_lower = domain.to_lowercase();
    let query_lower = query.to_lowercase();
    
    // Base authority scores
    let base_score = match domain_lower.as_str() {
        d if d.contains("wikipedia.org") => 0.85, // Reduced from 0.95
        d if d.contains(".gov") => 0.90,
        d if d.contains(".edu") => 0.85,
        d if d.contains("stackoverflow.com") => 0.80,
        d if d.contains("github.com") => 0.75,
        d if d.contains("mozilla.org") || d.contains("w3.org") => 0.75,
        d if d.contains("reddit.com") => 0.65,
        d if d.contains("reuters.com") || d.contains("bbc.com") || d.contains("cnn.com") => 0.70,
        d if d.contains("medium.com") || d.contains("dev.to") => 0.60,
        d if d.contains("blogspot") || d.contains("wordpress.com") => 0.30,
        d if d.contains("spam") || d.contains("ad.") => 0.10,
        _ => 0.50
    };
    
    // Intent-specific adjustments
    let intent_modifier = match intent {
        QueryIntent::News => {
            if domain_lower.contains("reuters") || domain_lower.contains("bbc") || 
               domain_lower.contains("cnn") || domain_lower.contains("reddit") {
                1.2 // Boost news sources and discussion platforms for news queries
            } else if domain_lower.contains("wikipedia") {
                0.7 // Reduce Wikipedia authority for news queries
            } else {
                1.0
            }
        },
        QueryIntent::Technical => {
            if domain_lower.contains("stackoverflow") || domain_lower.contains("github") {
                1.3 // Strong boost for technical resources
            } else if domain_lower.contains("dev.to") || domain_lower.contains("medium") {
                1.1 // Moderate boost for dev blogs
            } else if domain_lower.contains("wikipedia") {
                0.8 // Reduce Wikipedia for technical implementation questions
            } else {
                1.0
            }
        },
        QueryIntent::Tutorial => {
            if domain_lower.contains("dev.to") || domain_lower.contains("medium") ||
               domain_lower.contains("tutorial") {
                1.2 // Boost tutorial-focused sites
            } else if domain_lower.contains("wikipedia") {
                0.6 // Wikipedia is rarely good for tutorials
            } else {
                1.0
            }
        },
        QueryIntent::Academic => {
            if domain_lower.contains(".edu") || domain_lower.contains("wikipedia") {
                1.1 // Moderate boost for academic content
            } else {
                1.0
            }
        },
        QueryIntent::Comparison => {
            if domain_lower.contains("reddit") || domain_lower.contains("vs") {
                1.2 // Boost discussion platforms for comparisons
            } else if domain_lower.contains("wikipedia") {
                0.9 // Wikipedia comparisons can be outdated
            } else {
                1.0
            }
        },
        _ => 1.0
    };
    
    // Query-specific adjustments
    let query_modifier = if query_lower.contains("latest") || query_lower.contains("recent") || 
                           query_lower.contains("2024") || query_lower.contains("current") {
        if domain_lower.contains("wikipedia") {
            0.6 // Heavily penalize Wikipedia for current events
        } else if domain_lower.contains("news") || domain_lower.contains("reddit") {
            1.3 // Boost current event sources
        } else {
            1.0
        }
    } else if query_lower.contains("how to") || query_lower.contains("tutorial") {
        if domain_lower.contains("wikipedia") {
            0.5 // Wikipedia is poor for tutorials
        } else if domain_lower.contains("dev.to") || domain_lower.contains("medium") {
            1.2 // Boost tutorial platforms
        } else {
            1.0
        }
    } else {
        1.0
    };
    
    // Apply modifiers and clamp to [0.1, 1.0]
    let final_score: f64 = base_score * intent_modifier * query_modifier;
    final_score.max(0.1).min(1.0)
}

/// Calculate enhanced authority score (legacy function for compatibility)
fn calculate_enhanced_authority(url: &str) -> f64 {
    // Extract domain and use default intent/query
    if let Some(domain) = extract_domain(url) {
        calculate_context_aware_authority(&domain, &QueryIntent::General, "")
    } else {
        0.50
    }
}

/// Calculate content quality score
fn calculate_content_quality_score(result: &EnhancedSearchResult) -> f64 {
    let mut score = 0.5; // Base score
    
    // Title quality
    if result.title.len() >= 10 && result.title.len() <= 100 {
        score += 0.1;
    }
    
    // Snippet quality
    if let Some(snippet) = &result.snippet {
        if snippet.len() >= 50 && snippet.len() <= 300 {
            score += 0.2;
        }
    }
    
    // URL quality
    if result.url.len() < 100 && !result.url.contains('?') {
        score += 0.1;
    }
    
    // HTTPS bonus
    if result.url.starts_with("https://") {
        score += 0.1;
    }
    
    (score as f64).min(1.0)
}

/// Calculate intent-specific boost
fn calculate_intent_specific_boost(result: &EnhancedSearchResult, intent: &QueryIntent) -> f64 {
    match intent {
        QueryIntent::Academic => {
            if result.source == "Wikipedia" || result.url.contains(".edu") || result.url.contains("scholar") {
                1.0
            } else {
                0.5
            }
        }
        QueryIntent::Technical => {
            if result.source == "Stack Overflow" || result.url.contains("github.com") || 
               result.url.contains("documentation") {
                1.0
            } else {
                0.6
            }
        }
        QueryIntent::News => {
            if result.url.contains("news") || result.freshness_score > 0.8 {
                1.0
            } else {
                0.4
            }
        }
        _ => 0.7
    }
}

/// Calculate freshness score based on URL patterns
fn calculate_freshness_score(url: &str) -> f64 {
    let url_lower = url.to_lowercase();
    
    // Date patterns in URL
    if url_lower.contains("2024") { return 1.0; }
    if url_lower.contains("2023") { return 0.8; }
    if url_lower.contains("2022") { return 0.6; }
    
    // News and blog indicators
    if url_lower.contains("news") || url_lower.contains("blog") {
        return 0.7;
    }
    
    // Static documentation tends to be less fresh but still valuable
    if url_lower.contains("documentation") || url_lower.contains("manual") {
        return 0.5;
    }
    
    0.6 // Default freshness
}

/// Infer content type from URL and title
fn infer_content_type(url: &str, title: &str) -> Option<String> {
    let url_lower = url.to_lowercase();
    let title_lower = title.to_lowercase();
    
    if url_lower.contains("wikipedia.org") {
        Some("encyclopedia".to_string())
    } else if url_lower.contains("github.com") {
        Some("code_repository".to_string())
    } else if url_lower.contains("stackoverflow.com") {
        Some("qa_forum".to_string())
    } else if title_lower.contains("tutorial") || title_lower.contains("guide") {
        Some("tutorial".to_string())
    } else if title_lower.contains("documentation") || title_lower.contains("manual") {
        Some("documentation".to_string())
    } else if url_lower.contains("news") || url_lower.contains("article") {
        Some("news_article".to_string())
    } else {
        Some("webpage".to_string())
    }
}

/// Advanced deduplication with fuzzy matching
fn advanced_deduplication(results: Vec<EnhancedSearchResult>) -> Vec<EnhancedSearchResult> {
    let mut unique_results = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut seen_titles = HashSet::new();
    
    for result in results {
        let normalized_url = normalize_url(&result.url);
        let normalized_title = normalize_title(&result.title);
        
        // Check for near-duplicates
        let is_duplicate = seen_urls.iter().any(|existing_url: &String| {
            url_similarity(existing_url, &normalized_url) > 0.8
        }) || seen_titles.iter().any(|existing_title: &String| {
            title_similarity(existing_title, &normalized_title) > 0.9
        });
        
        if !is_duplicate {
            seen_urls.insert(normalized_url);
            seen_titles.insert(normalized_title);
            unique_results.push(result);
        }
    }
    
    unique_results
}

/// Diversify results to ensure variety in sources and perspectives
fn diversify_results(mut results: Vec<EnhancedSearchResult>, config: &EnhancedWebSearchConfig) -> Vec<EnhancedSearchResult> {
    // Group by domain
    let mut domain_counts: HashMap<String, usize> = HashMap::new();
    let mut diversified = Vec::new();
    
    // Sort by score first
    results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
    
    for result in results {
        let domain = extract_domain(&result.url).unwrap_or_default();
        let count = domain_counts.get(&domain).unwrap_or(&0);
        
        // Limit results per domain to ensure diversity
        if *count < 3 || diversified.len() < config.max_total_results / 2 {
            domain_counts.insert(domain, count + 1);
            diversified.push(result);
        }
        
        if diversified.len() >= config.max_total_results {
            break;
        }
    }
    
    diversified
}

/// Extract content with adaptive strategy based on intent
async fn extract_content_with_adaptive_strategy(
    client: &Client, 
    url: &str, 
    intent: &QueryIntent, 
    config: &EnhancedWebSearchConfig
) -> Result<String> {
    let response = fetch_with_intelligent_retry(client, url, config).await?;
    let html = response.text().await?;
    let document = Html::parse_document(&html);
    
    // Intent-specific content selectors
    let content_selectors = match intent {
        QueryIntent::Technical => vec![
            "article", ".content", "#content", ".documentation", ".readme",
            ".markdown-body", ".wiki-content", "main", ".main-content",
        ],
        QueryIntent::Tutorial => vec![
            ".tutorial", ".guide", ".lesson", "article", ".content",
            "#content", ".post-content", ".entry-content", "main",
        ],
        QueryIntent::Academic => vec![
            ".abstract", ".article-body", ".content", "article",
            ".paper-content", ".study-content", "main", "#content",
        ],
        _ => vec![
            "article", "[role='main']", ".content", "#content",
            ".post-content", ".entry-content", ".article-body",
            ".story-body", ".main-content", "main", ".markdown-body",
        ]
    };
    
    for selector_str in &content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let content = extract_clean_text(&element.text().collect::<String>());
                if content.len() > 100 {
                    return Ok(limit_content_length(content, config.max_content_length));
                }
            }
        }
    }
    
    // Fallback: extract from paragraphs
    if let Ok(p_selector) = Selector::parse("p") {
        let paragraphs: Vec<String> = document
            .select(&p_selector)
            .take(15)
            .map(|p| extract_clean_text(&p.text().collect::<String>()))
            .filter(|p| p.len() > 20)
            .collect();
        
        if !paragraphs.is_empty() {
            return Ok(limit_content_length(paragraphs.join(" "), config.max_content_length));
        }
    }
    
    Err(anyhow!("No quality content found"))
}

/// Check if content meets quality standards
fn is_high_quality_content(content: &str, _config: &EnhancedWebSearchConfig) -> bool {
    if content.len() < 50 {
        return false;
    }
    
    let words: Vec<&str> = content.split_whitespace().collect();
    if words.len() < 15 {
        return false;
    }
    
    // Check word diversity
    let unique_words: HashSet<&str> = words.iter().cloned().collect();
    let diversity = unique_words.len() as f64 / words.len() as f64;
    
    if diversity < 0.3 {
        return false;
    }
    
    // Check for spam patterns
    let spam_patterns = [
        "click here", "buy now", "limited time", "act now", "subscribe",
        "advertisement", "sponsored", "affiliate", "cookie consent"
    ];
    let spam_count = spam_patterns.iter()
        .filter(|&pattern| content.to_lowercase().contains(pattern))
        .count();
    
    spam_count < 3
}

/// Check if result meets quality standards
fn is_quality_result(title: &str, url: &str, config: &EnhancedWebSearchConfig) -> bool {
    // Title validation
    if title.len() < 5 || title.len() > 200 {
        return false;
    }
    
    // URL validation
    if !url.starts_with("http") || url.len() > 2000 {
        return false;
    }
    
    // Check for metadata patterns
    if is_likely_metadata(title) {
        return false;
    }
    
    // Check for low-quality domains if enabled
    if config.exclude_low_authority_domains {
        let low_quality_patterns = [
            "ads.", "ad.", "advertising", "affiliate", "promo",
            "spam", "malware", "virus", "porn", "xxx", "casino",
            "loan", "payday", "clickbait", "fake-news"
        ];
        
        for pattern in &low_quality_patterns {
            if url.to_lowercase().contains(pattern) {
                return false;
            }
        }
    }
    
    true
}

/// Check if text is likely metadata or UI elements
fn is_likely_metadata(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    
    // Language codes
    if text.len() <= 3 && text.chars().all(|c| c.is_alphabetic()) {
        return true;
    }
    
    // Common metadata patterns
    let metadata_patterns = [
        "home", "login", "register", "search", "menu", "nav",
        "header", "footer", "sidebar", "loading", "error", "404",
        "javascript", "css", "html", "xml", "json", "api", "rss",
        "cookie", "privacy", "terms", "contact", "about",
        "en", "de", "fr", "es", "it", "pt", "ru", "zh", "ja", "ko",
    ];
    
    metadata_patterns.iter().any(|&pattern| text_lower == pattern)
}

// ============ Utility Functions ============

/// Extract clean text from raw HTML text
fn extract_clean_text(text: &str) -> String {
    text.trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?;:()[]{}\"'-".contains(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract intelligent snippet based on intent
fn extract_intelligent_snippet(_document: &Html, element: scraper::ElementRef, intent: &QueryIntent) -> Option<String> {
    // Intent-specific snippet selectors
    let snippet_selectors = match intent {
        QueryIntent::Technical => vec![".description", ".summary", ".excerpt", ".b_caption p", ".result__snippet"],
        QueryIntent::Academic => vec![".abstract", ".summary", ".excerpt", ".description"],
        _ => vec![".b_caption p", ".result__snippet", ".st", ".s", ".description", ".summary"]
    };
    
    if let Some(parent) = element.parent() {
        if let Some(parent_element) = scraper::ElementRef::wrap(parent) {
            for selector_str in &snippet_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    if let Some(snippet_elem) = parent_element.select(&selector).next() {
                        let snippet = extract_clean_text(&snippet_elem.text().collect::<String>());
                        if snippet.len() > 15 && snippet.len() < 400 {
                            return Some(snippet);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Clean DuckDuckGo redirect URLs
fn clean_duckduckgo_url(href: &str) -> Option<String> {
    if href.starts_with("/l/?uddg=") {
        urlencoding::decode(&href[9..]).ok().map(|s| s.to_string())
    } else if href.starts_with("//duckduckgo.com/l/?uddg=") {
        urlencoding::decode(&href[25..]).ok().map(|s| s.to_string())
    } else if href.starts_with("http") {
        Some(href.to_string())
    } else {
        None
    }
}

/// Clean HTML entities from text
fn clean_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&hellip;", "...")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
}

/// Limit content to specified maximum length
fn limit_content_length(content: String, max_length: usize) -> String {
    if content.len() > max_length {
        format!("{}...", content.chars().take(max_length).collect::<String>())
    } else {
        content
    }
}

/// Normalize URL for comparison
fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/')
        .replace("http://", "https://")
        .replace("www.", "")
        .to_lowercase()
}

/// Normalize title for comparison
fn normalize_title(title: &str) -> String {
    title.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate URL similarity for deduplication
fn url_similarity(url1: &str, url2: &str) -> f64 {
    let url1_parts: Vec<&str> = url1.split('/').collect();
    let url2_parts: Vec<&str> = url2.split('/').collect();
    
    let common_parts = url1_parts.iter()
        .zip(url2_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();
    
    let max_parts = url1_parts.len().max(url2_parts.len());
    if max_parts == 0 { 0.0 } else { common_parts as f64 / max_parts as f64 }
}

/// Calculate title similarity for deduplication
fn title_similarity(title1: &str, title2: &str) -> f64 {
    let words1: HashSet<_> = title1.split_whitespace().collect();
    let words2: HashSet<_> = title2.split_whitespace().collect();
    
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();
    
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = Url::parse(url) {
        parsed_url.host_str().map(|h| h.to_string())
    } else {
        None
    }
}

/// Format enhanced search results for display
pub fn format_enhanced_search_results(results: &[EnhancedSearchResult], query: &str) -> String {
    if results.is_empty() {
        return format!("No search results found for '{}'", query);
    }
    
    let mut output = Vec::new();
    output.push(format!("🔍 Enhanced Search Results for '{}' ({} results):\n", query, results.len()));
    
    for (index, result) in results.iter().enumerate() {
        let mut result_text = Vec::new();
        
        result_text.push(format!("{}. 🔗 **{}**", index + 1, result.title));
        result_text.push(format!("   URL: {}", result.url));
        result_text.push(format!("   Source: {} | Intent: {:?}", result.source, result.query_intent));
        result_text.push(format!("   Scores: Relevance {:.2} | Authority {:.2} | Quality {:.2} | Final {:.2}", 
            result.relevance_score, result.authority_score, result.quality_score, result.final_score));
        
        if let Some(content_type) = &result.content_type {
            result_text.push(format!("   Type: {}", content_type));
        }
        
        if let Some(snippet) = &result.snippet {
            result_text.push(format!("   Snippet: {}", snippet));
        }
        
        if let Some(content) = &result.content {
            let preview = if content.len() > 300 {
                format!("{}...", content.chars().take(300).collect::<String>())
            } else {
                content.clone()
            };
            result_text.push(format!("   Content: {}", preview));
        }
        
        if let Some(reading_time) = result.reading_time {
            result_text.push(format!("   Reading time: {} min", reading_time));
        }
        
        output.push(result_text.join("\n"));
    }
    
    output.join("\n\n")
}