use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

/// Configuration for web search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    pub timeout_seconds: u64,
    pub max_results: usize,
    pub max_content_length: usize,
    pub max_scrape_urls: usize,
    pub user_agent: String,
    pub retry_attempts: usize,
    pub retry_delay_ms: u64,
    pub enable_content_extraction: bool,
    pub enable_javascript_sites: bool,
    pub follow_redirects: bool,
    pub respect_robots_txt: bool,
    pub cache_results: bool,
    pub cache_duration_hours: u64,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 15,
            max_results: 10,
            max_content_length: 3000,
            max_scrape_urls: 5,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            retry_attempts: 2,
            retry_delay_ms: 1000,
            enable_content_extraction: true,
            enable_javascript_sites: false,
            follow_redirects: true,
            respect_robots_txt: false,
            cache_results: true,
            cache_duration_hours: 24,
        }
    }
}

/// Represents a search result from any search engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
    pub source: String,
    pub relevance_score: f64,
    pub authority_score: f64,
    pub content: Option<String>,
    pub metadata: Option<SearchMetadata>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub word_count: Option<usize>,
}

/// Additional metadata for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    pub domain: String,
    pub has_https: bool,
    pub estimated_reading_time: Option<u32>,
    pub social_signals: Option<SocialSignals>,
    pub technical_metrics: Option<TechnicalMetrics>,
}

/// Social media and engagement signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialSignals {
    pub share_count: Option<u64>,
    pub like_count: Option<u64>,
    pub comment_count: Option<u64>,
}

/// Technical performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalMetrics {
    pub load_time_ms: Option<u64>,
    pub mobile_friendly: Option<bool>,
    pub has_schema: Option<bool>,
    pub ssl_score: Option<f64>,
}

/// Quality metrics for search results
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    pub title_quality: f64,
    pub url_quality: f64,
    pub content_quality: f64,
    pub source_authority: f64,
}

/// Main web search orchestrator
pub struct WebSearchEngine {
    client: Client,
    config: Arc<WebSearchConfig>,
}

impl WebSearchEngine {
    pub fn new(config: WebSearchConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config: Arc::new(config),
        }
    }

    /// Perform a comprehensive web search across multiple engines
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        println!("{} Starting comprehensive web search for: {}", "🔍".cyan(), query.yellow());

        let mut all_results = Vec::new();
        // Execute all searches concurrently
        let (duckduckgo_results, bing_results, wikipedia_results) = futures::future::join3(
            self.search_duckduckgo(query),
            self.search_bing(query),
            self.search_wikipedia(query),
        ).await;
        
        let search_results = vec![duckduckgo_results, bing_results, wikipedia_results];

        for (engine_name, result) in ["DuckDuckGo", "Bing", "Wikipedia"].iter().zip(search_results) {
            match result {
                Ok(mut results) => {
                    if !results.is_empty() {
                        println!("{} Found {} results from {}", "✓".green(), results.len(), engine_name);
                        all_results.append(&mut results);
                    } else {
                        println!("{} No results from {}", "⚠".yellow(), engine_name);
                    }
                }
                Err(e) => {
                    println!("{} {} search failed: {}", "✗".red(), engine_name, e);
                }
            }
        }

        if all_results.is_empty() {
            return Err(anyhow!("No search results found from any search engine"));
        }

        // Process and rank results
        let processed_results = self.process_and_rank_results(all_results, query).await;
        
        // Extract content from top results
        let final_results = self.extract_content_from_results(processed_results).await;

        Ok(final_results)
    }

    /// Search DuckDuckGo with robust parsing
    async fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        
        let response = self.fetch_with_retry(&url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();
        
        // Try multiple selectors for better coverage
        let selectors = [
            ".result .result__title a",
            ".results_links .result__title a",
            ".web-result .result__title a",
            "h2.result__title a",
            ".result__title a",
        ];

        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(self.config.max_results * 2) {
                    if let Some(href) = element.value().attr("href") {
                        let title = self.extract_clean_text(element.text().collect::<String>());
                        
                        if let Some(clean_url) = self.clean_duckduckgo_url(href) {
                            if self.is_valid_result(&title, &clean_url) {
                                let snippet = self.extract_snippet_near_element(&document, element);
                                
                                let relevance_score = self.calculate_relevance_score(&title, query);
                                let authority_score = self.calculate_authority_score(&clean_url);
                                
                                results.push(SearchResult {
                                    title,
                                    url: clean_url,
                                    snippet,
                                    source: "DuckDuckGo".to_string(),
                                    relevance_score,
                                    authority_score,
                                    content: None,
                                    metadata: None,
                                    timestamp: chrono::Utc::now(),
                                    content_type: None,
                                    language: None,
                                    word_count: None,
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

        self.deduplicate_results(results)
    }

    /// Search Bing with robust parsing
    async fn search_bing(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://www.bing.com/search?q={}", urlencoding::encode(query));
        
        let response = self.fetch_with_retry(&url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();
        
        // Multiple selectors for Bing
        let selectors = [
            "li.b_algo h2 a",
            ".b_algo .b_title a",
            "h2 a[href^='http']",
            ".b_title a[href^='http']",
            ".b_algo a[href^='http']",
        ];

        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(self.config.max_results * 2) {
                    if let Some(href) = element.value().attr("href") {
                        let title = self.extract_clean_text(element.text().collect::<String>());
                        
                        if self.is_valid_result(&title, href) && href.starts_with("http") {
                            let snippet = self.extract_snippet_near_element(&document, element);
                            
                            let relevance_score = self.calculate_relevance_score(&title, query);
                            let authority_score = self.calculate_authority_score(href);
                            
                            results.push(SearchResult {
                                title,
                                url: href.to_string(),
                                snippet,
                                source: "Bing".to_string(),
                                relevance_score,
                                authority_score,
                                content: None,
                                metadata: None,
                                timestamp: chrono::Utc::now(),
                                content_type: None,
                                language: None,
                                word_count: None,
                            });
                        }
                    }
                }
                
                if !results.is_empty() {
                    break;
                }
            }
        }

        self.deduplicate_results(results)
    }

    /// Search Wikipedia API
    async fn search_wikipedia(&self, query: &str) -> Result<Vec<SearchResult>> {
        let search_url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&format=json&list=search&srsearch={}&srlimit={}",
            urlencoding::encode(query),
            self.config.max_results
        );

        let response = self.fetch_with_retry(&search_url).await?;
        let json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        if let Some(search_results) = json["query"]["search"].as_array() {
            for item in search_results.iter().take(self.config.max_results) {
                if let (Some(title), Some(_pageid)) = (item["title"].as_str(), item["pageid"].as_u64()) {
                    let url = format!("https://en.wikipedia.org/wiki/{}", urlencoding::encode(title));
                    let snippet = item["snippet"].as_str()
                        .map(|s| self.clean_html_entities(s))
                        .filter(|s| !s.is_empty());

                    results.push(SearchResult {
                        title: title.to_string(),
                        url,
                        snippet,
                        source: "Wikipedia".to_string(),
                        relevance_score: self.calculate_relevance_score(title, query),
                        authority_score: 0.95, // Wikipedia has high authority
                        content: None,
                        metadata: None,
                        timestamp: chrono::Utc::now(),
                        content_type: Some("encyclopedia".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Fetch URL with retry mechanism
    async fn fetch_with_retry(&self, url: &str) -> Result<reqwest::Response> {
        let mut last_error = None;
        
        for attempt in 0..=self.config.retry_attempts {
            match timeout(
                Duration::from_secs(self.config.timeout_seconds),
                self.client.get(url).send()
            ).await {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else {
                        last_error = Some(anyhow!("HTTP error: {}", response.status()));
                    }
                }
                Ok(Err(e)) => last_error = Some(anyhow!("Request error: {}", e)),
                Err(_) => last_error = Some(anyhow!("Request timeout")),
            }
            
            if attempt < self.config.retry_attempts {
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Unknown fetch error")))
    }

    /// Process and rank all results
    async fn process_and_rank_results(&self, mut results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
        // Remove duplicates
        results = self.deduplicate_results(results).unwrap_or_default();
        
        // Calculate combined scores
        for result in &mut results {
            let quality = self.assess_result_quality(result, query);
            result.relevance_score = quality.title_quality * 0.4 + 
                                   quality.url_quality * 0.2 + 
                                   quality.content_quality * 0.2 +
                                   quality.source_authority * 0.2;
        }
        
        // Sort by combined score
        results.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Take top results
        results.into_iter().take(self.config.max_results).collect()
    }

    /// Extract content from top search results
    async fn extract_content_from_results(&self, mut results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut content_futures = Vec::new();
        
        for (index, result) in results.iter().enumerate() {
            if index < self.config.max_scrape_urls {
                content_futures.push(self.extract_page_content(&result.url));
            }
        }
        
        let content_results = futures::future::join_all(content_futures).await;
        
        for (index, content_result) in content_results.into_iter().enumerate() {
            if index < results.len() && index < self.config.max_scrape_urls {
                match content_result {
                    Ok(content) => {
                        if self.is_quality_content(&content) {
                            results[index].content = Some(content);
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

    /// Extract clean, readable content from a webpage
    pub async fn extract_page_content(&self, url: &str) -> Result<String> {
        let response = self.fetch_with_retry(url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Try multiple content selectors in order of preference
        let content_selectors = [
            "article",
            "[role='main']",
            ".content",
            "#content",
            ".post-content",
            ".entry-content",
            ".article-body",
            ".story-body",
            ".main-content",
            "main",
            ".markdown-body",
            ".document",
        ];

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let content = self.extract_clean_text(element.text().collect::<String>());
                    if content.len() > 100 {
                        return Ok(self.limit_content_length(content));
                    }
                }
            }
        }

        // Fallback: extract from paragraphs
        if let Ok(p_selector) = Selector::parse("p") {
            let paragraphs: Vec<String> = document
                .select(&p_selector)
                .take(10)
                .map(|p| self.extract_clean_text(p.text().collect::<String>()))
                .filter(|p| p.len() > 20)
                .collect();
            
            if !paragraphs.is_empty() {
                return Ok(self.limit_content_length(paragraphs.join(" ")));
            }
        }

        Err(anyhow!("No quality content found"))
    }

    /// Clean DuckDuckGo redirect URLs
    fn clean_duckduckgo_url(&self, href: &str) -> Option<String> {
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

    /// Extract clean text from raw HTML text
    fn extract_clean_text(&self, text: String) -> String {
        text.trim()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?;:()[]{}\"'-".contains(*c))
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Extract snippet text near a search result element
    fn extract_snippet_near_element(&self, _document: &Html, element: scraper::ElementRef) -> Option<String> {
        // Look for snippet in parent containers
        if let Some(parent) = element.parent() {
            if let Some(parent_element) = scraper::ElementRef::wrap(parent) {
                // Try to find description/snippet elements
                let snippet_selectors = [".b_caption p", ".result__snippet", ".st", ".s"];
                
                for selector_str in &snippet_selectors {
                    if let Ok(selector) = Selector::parse(selector_str) {
                        if let Some(snippet_elem) = parent_element.select(&selector).next() {
                            let snippet = self.extract_clean_text(snippet_elem.text().collect::<String>());
                            if snippet.len() > 10 && snippet.len() < 300 {
                                return Some(snippet);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if a result is valid (not metadata, ads, etc.)
    fn is_valid_result(&self, title: &str, url: &str) -> bool {
        // Title validation
        if title.len() < 5 || title.len() > 200 {
            return false;
        }

        // Check for metadata patterns
        if self.is_likely_metadata(title) {
            return false;
        }

        // URL validation
        if !url.starts_with("http") || url.len() > 2000 {
            return false;
        }

        // Check for low-quality domains
        let low_quality_patterns = [
            "ads.", "ad.", "advertising", "affiliate", "promo",
            "spam", "malware", "virus", "porn", "xxx",
        ];

        for pattern in &low_quality_patterns {
            if url.to_lowercase().contains(pattern) {
                return false;
            }
        }

        true
    }

    /// Check if text is likely metadata or UI elements
    fn is_likely_metadata(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        // Language codes
        if text.len() <= 3 && text.chars().all(|c| c.is_alphabetic()) {
            return true;
        }

        // Common metadata
        let metadata_patterns = [
            "home", "login", "register", "search", "menu", "nav",
            "header", "footer", "sidebar", "loading", "error", "404",
            "javascript", "css", "html", "xml", "json", "api",
            "en", "de", "fr", "es", "it", "pt", "ru", "zh", "ja", "ko",
        ];

        metadata_patterns.iter().any(|&pattern| text_lower == pattern)
    }

    /// Calculate relevance score based on title and query
    fn calculate_relevance_score(&self, title: &str, query: &str) -> f64 {
        let title_lower = title.to_lowercase();
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        let mut score = 0.0;
        let mut total_possible = 0.0;

        for word in &query_words {
            total_possible += 1.0;
            if title_lower.contains(word) {
                score += 1.0;
                // Bonus for exact word boundaries
                if title_lower.split_whitespace().any(|w| w == *word) {
                    score += 0.5;
                }
            }
        }

        if total_possible > 0.0 {
            score / total_possible
        } else {
            0.0
        }
    }

    /// Calculate authority score based on domain
    fn calculate_authority_score(&self, url: &str) -> f64 {
        let url_lower = url.to_lowercase();
        
        if url_lower.contains("wikipedia.org") { return 0.95; }
        if url_lower.contains(".gov") { return 0.90; }
        if url_lower.contains(".edu") { return 0.85; }
        if url_lower.contains("stackoverflow.com") { return 0.80; }
        if url_lower.contains("github.com") { return 0.75; }
        if url_lower.contains("mozilla.org") || url_lower.contains("w3.org") { return 0.75; }
        if url_lower.contains("britannica.com") { return 0.70; }
        if url_lower.contains("reuters.com") || url_lower.contains("bbc.com") { return 0.70; }
        if url_lower.contains("medium.com") || url_lower.contains("dev.to") { return 0.60; }
        
        // Penalty for low-quality indicators
        if url_lower.contains("blogspot") || url_lower.contains("wordpress.com") { return 0.30; }
        
        0.50 // Default score
    }

    /// Assess overall quality of a search result
    fn assess_result_quality(&self, result: &SearchResult, query: &str) -> QualityMetrics {
        QualityMetrics {
            title_quality: self.calculate_relevance_score(&result.title, query),
            url_quality: if result.url.len() < 100 && !result.url.contains('?') { 0.8 } else { 0.5 },
            content_quality: if result.snippet.is_some() { 0.7 } else { 0.3 },
            source_authority: result.authority_score,
        }
    }

    /// Check if extracted content is high quality
    fn is_quality_content(&self, content: &str) -> bool {
        if content.len() < 50 {
            return false;
        }

        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() < 10 {
            return false;
        }

        // Check word diversity
        let unique_words: HashSet<&str> = words.iter().cloned().collect();
        let diversity = unique_words.len() as f64 / words.len() as f64;
        
        if diversity < 0.3 {
            return false;
        }

        // Check for spam indicators
        let spam_patterns = ["click here", "buy now", "limited time", "act now", "subscribe"];
        let spam_count = spam_patterns.iter()
            .filter(|&pattern| content.to_lowercase().contains(pattern))
            .count();

        spam_count < 2
    }

    /// Remove duplicate results
    fn deduplicate_results(&self, results: Vec<SearchResult>) -> Result<Vec<SearchResult>> {
        let mut unique_results = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut seen_titles = HashSet::new();

        for result in results {
            let normalized_url = self.normalize_url_basic(&result.url);
            let normalized_title = result.title.to_lowercase().trim().to_string();

            if !seen_urls.contains(&normalized_url) && !seen_titles.contains(&normalized_title) {
                seen_urls.insert(normalized_url);
                seen_titles.insert(normalized_title);
                unique_results.push(result);
            }
        }

        Ok(unique_results)
    }

    /// Normalize URL for deduplication (basic version)
    fn normalize_url_basic(&self, url: &str) -> String {
        if let Ok(parsed) = Url::parse(url) {
            format!("{}://{}{}", 
                parsed.scheme(), 
                parsed.host_str().unwrap_or_default(),
                parsed.path()
            )
        } else {
            url.to_string()
        }
    }

    /// Clean HTML entities from text
    fn clean_html_entities(&self, text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
    }

    /// Limit content to configured maximum length
    fn limit_content_length(&self, content: String) -> String {
        if content.len() > self.config.max_content_length {
            format!("{}...", content.chars().take(self.config.max_content_length).collect::<String>())
        } else {
            content
        }
    }
}

/// Format search results for display
pub fn format_search_results(results: &[SearchResult], query: &str) -> String {
    if results.is_empty() {
        return format!("No search results found for '{}'", query);
    }

    let mut output = Vec::new();
    output.push(format!("🔍 Search Results for '{}' ({} results):\n", query, results.len()));

    for (index, result) in results.iter().enumerate() {
        let mut result_text = Vec::new();
        
        result_text.push(format!("{}. 🔗 **{}**", index + 1, result.title));
        result_text.push(format!("   URL: {}", result.url));
        result_text.push(format!("   Source: {} (Score: {:.2})", result.source, result.relevance_score));
        
        if let Some(snippet) = &result.snippet {
            result_text.push(format!("   Snippet: {}", snippet));
        }
        
        if let Some(content) = &result.content {
            let preview = if content.len() > 200 {
                format!("{}...", content.chars().take(200).collect::<String>())
            } else {
                content.clone()
            };
            result_text.push(format!("   Content: {}", preview));
        }
        
        output.push(result_text.join("\n"));
    }

    output.join("\n\n")
}

/// Get fallback resources when search fails
pub fn get_fallback_resources(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut resources = Vec::new();

    // Programming and technical resources
    if query_lower.contains("rust") {
        resources.extend(vec![
            "📘 The Rust Programming Language: https://doc.rust-lang.org/book/".to_string(),
            "📚 Rust by Example: https://doc.rust-lang.org/rust-by-example/".to_string(),
            "🦀 Rust Standard Library: https://doc.rust-lang.org/std/".to_string(),
        ]);
    }

    if query_lower.contains("python") {
        resources.extend(vec![
            "🐍 Python Documentation: https://docs.python.org/3/".to_string(),
            "📖 Python Tutorial: https://docs.python.org/3/tutorial/".to_string(),
        ]);
    }

    if query_lower.contains("javascript") || query_lower.contains("js") {
        resources.extend(vec![
            "📝 MDN JavaScript Guide: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide".to_string(),
            "⚡ JavaScript Reference: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference".to_string(),
        ]);
    }

    if query_lower.contains("git") {
        resources.extend(vec![
            "📚 Pro Git Book: https://git-scm.com/book".to_string(),
            "🔧 Git Documentation: https://git-scm.com/docs".to_string(),
        ]);
    }

    // General resources if nothing specific found
    if resources.is_empty() {
        resources.extend(vec![
            "🎓 Stack Overflow: https://stackoverflow.com/".to_string(),
            "📚 MDN Web Docs: https://developer.mozilla.org/".to_string(),
            "💻 GitHub: https://github.com/".to_string(),
            "🔍 Wikipedia: https://en.wikipedia.org/".to_string(),
        ]);
    }

    resources
}

/// Enhanced web search engine with additional search providers
impl WebSearchEngine {
    /// Search Google Scholar for academic papers
    pub async fn search_google_scholar(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://scholar.google.com/scholar?q={}", urlencoding::encode(query));
        
        let response = self.fetch_with_retry(&url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();
        
        // Google Scholar specific selectors
        let title_selector = Selector::parse(".gs_rt a").map_err(|e| anyhow!("Invalid selector: {}", e))?;
        let _snippet_selector = Selector::parse(".gs_rs").map_err(|e| anyhow!("Invalid selector: {}", e))?;

        for (i, element) in document.select(&title_selector).enumerate() {
            if i >= self.config.max_results { break; }
            
            if let Some(href) = element.value().attr("href") {
                let title = self.extract_clean_text(element.text().collect::<String>());
                
                if self.is_valid_result(&title, href) {
                    let snippet = self.extract_snippet_near_element(&document, element);
                    
                    let relevance_score = self.calculate_relevance_score(&title, query);
                    let authority_score = self.calculate_authority_score(href) + 0.2; // Academic boost
                    
                    results.push(SearchResult {
                        title,
                        url: href.to_string(),
                        snippet,
                        source: "Google Scholar".to_string(),
                        relevance_score,
                        authority_score,
                        content: None,
                        metadata: Some(SearchMetadata {
                            domain: self.extract_domain(href).unwrap_or_default(),
                            has_https: href.starts_with("https://"),
                            estimated_reading_time: None,
                            social_signals: None,
                            technical_metrics: None,
                        }),
                        timestamp: chrono::Utc::now(),
                        content_type: Some("academic".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Search GitHub repositories
    pub async fn search_github(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://github.com/search?q={}&type=repositories", urlencoding::encode(query));
        
        let response = self.fetch_with_retry(&url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();
        
        // GitHub repository selectors
        let repo_selector = Selector::parse("a[data-testid='results-list']").map_err(|e| anyhow!("Invalid selector: {}", e))?;

        for (i, element) in document.select(&repo_selector).enumerate() {
            if i >= self.config.max_results { break; }
            
            if let Some(href) = element.value().attr("href") {
                let title = self.extract_clean_text(element.text().collect::<String>());
                let full_url = if href.starts_with("/") {
                    format!("https://github.com{}", href)
                } else {
                    href.to_string()
                };
                
                if self.is_valid_result(&title, &full_url) {
                    let snippet = self.extract_snippet_near_element(&document, element);
                    
                    let relevance_score = self.calculate_relevance_score(&title, query);
                    let authority_score = self.calculate_authority_score(&full_url) + 0.1; // Code boost
                    
                    results.push(SearchResult {
                        title,
                        url: full_url,
                        snippet,
                        source: "GitHub".to_string(),
                        relevance_score,
                        authority_score,
                        content: None,
                        metadata: Some(SearchMetadata {
                            domain: "github.com".to_string(),
                            has_https: true,
                            estimated_reading_time: None,
                            social_signals: None,
                            technical_metrics: None,
                        }),
                        timestamp: chrono::Utc::now(),
                        content_type: Some("code".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Search Stack Overflow for technical questions
    pub async fn search_stackoverflow(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://stackoverflow.com/search?q={}", urlencoding::encode(query));
        
        let response = self.fetch_with_retry(&url).await?;
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();
        
        // Stack Overflow specific selectors
        let question_selector = Selector::parse(".question-summary .question-hyperlink").map_err(|e| anyhow!("Invalid selector: {}", e))?;

        for (i, element) in document.select(&question_selector).enumerate() {
            if i >= self.config.max_results { break; }
            
            if let Some(href) = element.value().attr("href") {
                let title = self.extract_clean_text(element.text().collect::<String>());
                let full_url = if href.starts_with("/") {
                    format!("https://stackoverflow.com{}", href)
                } else {
                    href.to_string()
                };
                
                if self.is_valid_result(&title, &full_url) {
                    let snippet = self.extract_snippet_near_element(&document, element);
                    
                    let relevance_score = self.calculate_relevance_score(&title, query);
                    let authority_score = self.calculate_authority_score(&full_url) + 0.15; // Technical boost
                    
                    results.push(SearchResult {
                        title,
                        url: full_url,
                        snippet,
                        source: "Stack Overflow".to_string(),
                        relevance_score,
                        authority_score,
                        content: None,
                        metadata: Some(SearchMetadata {
                            domain: "stackoverflow.com".to_string(),
                            has_https: true,
                            estimated_reading_time: None,
                            social_signals: None,
                            technical_metrics: None,
                        }),
                        timestamp: chrono::Utc::now(),
                        content_type: Some("qa".to_string()),
                        language: Some("en".to_string()),
                        word_count: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Enhanced comprehensive search including specialized sources
    pub async fn enhanced_search(&self, query: &str, include_specialized: bool) -> Result<Vec<SearchResult>> {
        println!("{} Starting enhanced web search for: {}", "🔍".cyan(), query.yellow());

        let mut all_results = Vec::new();
        
        // Core search engines
        let core_searches = futures::future::join3(
            self.search_duckduckgo(query),
            self.search_bing(query),
            self.search_wikipedia(query),
        );

        let (duckduckgo_results, bing_results, wikipedia_results) = core_searches.await;
        
        let core_results = vec![
            ("DuckDuckGo", duckduckgo_results),
            ("Bing", bing_results),
            ("Wikipedia", wikipedia_results),
        ];

        // Process core results
        for (engine_name, result) in core_results {
            match result {
                Ok(mut results) => {
                    if !results.is_empty() {
                        println!("{} Found {} results from {}", "✓".green(), results.len(), engine_name);
                        all_results.append(&mut results);
                    }
                }
                Err(e) => {
                    println!("{} {} search failed: {}", "✗".red(), engine_name, e);
                }
            }
        }

        // Specialized search engines if requested
        if include_specialized {
            let specialized_searches = futures::future::join3(
                self.search_google_scholar(query),
                self.search_github(query),
                self.search_stackoverflow(query),
            );

            let (scholar_results, github_results, stackoverflow_results) = specialized_searches.await;
            
            let specialized_results = vec![
                ("Google Scholar", scholar_results),
                ("GitHub", github_results),
                ("Stack Overflow", stackoverflow_results),
            ];

            for (engine_name, result) in specialized_results {
                match result {
                    Ok(mut results) => {
                        if !results.is_empty() {
                            println!("{} Found {} results from {}", "✓".green(), results.len(), engine_name);
                            all_results.append(&mut results);
                        }
                    }
                    Err(e) => {
                        println!("{} {} search failed: {}", "⚠".yellow(), engine_name, e);
                    }
                }
            }
        }

        if all_results.is_empty() {
            return Err(anyhow!("No search results found from any search engine"));
        }

        // Process and rank results with enhanced scoring
        let processed_results = self.enhanced_process_and_rank_results(all_results, query).await;
        
        // Extract content from top results
        let final_results = self.extract_content_from_results(processed_results).await;

        Ok(final_results)
    }

    /// Enhanced result processing with better ranking algorithms
    async fn enhanced_process_and_rank_results(&self, mut results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
        // Remove duplicates
        results = self.advanced_deduplication(results);

        // Enhanced scoring
        for result in &mut results {
            result.relevance_score = self.enhanced_relevance_scoring(&result.title, query, result.snippet.as_deref());
            result.authority_score = self.enhanced_authority_scoring(&result.url, &result.source);
        }

        // Sort by combined score
        results.sort_by(|a, b| {
            let score_a = (a.relevance_score * 0.7) + (a.authority_score * 0.3);
            let score_b = (b.relevance_score * 0.7) + (b.authority_score * 0.3);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top results
        results.into_iter().take(self.config.max_results).collect()
    }

    /// Advanced deduplication with fuzzy matching
    fn advanced_deduplication(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut unique_results = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut seen_titles = HashSet::new();

        for result in results {
            let normalized_url = self.normalize_url_basic(&result.url);
            let normalized_title = self.normalize_title(&result.title);

            if !seen_urls.contains(&normalized_url) && !seen_titles.contains(&normalized_title) {
                seen_urls.insert(normalized_url);
                seen_titles.insert(normalized_title);
                unique_results.push(result);
            }
        }

        unique_results
    }

    /// Enhanced relevance scoring considering multiple factors
    fn enhanced_relevance_scoring(&self, title: &str, query: &str, snippet: Option<&str>) -> f64 {
        let mut score = 0.0;
        let query_lower = query.to_lowercase();
        let title_lower = title.to_lowercase();

        // Exact match bonus
        if title_lower.contains(&query_lower) {
            score += 1.0;
        }

        // Word overlap scoring
        let query_words: HashSet<_> = query_lower.split_whitespace().collect();
        let title_words: HashSet<_> = title_lower.split_whitespace().collect();
        let overlap = query_words.intersection(&title_words).count();
        score += (overlap as f64 / query_words.len() as f64) * 0.8;

        // Snippet relevance if available
        if let Some(snippet) = snippet {
            let snippet_lower = snippet.to_lowercase();
            if snippet_lower.contains(&query_lower) {
                score += 0.3;
            }
        }

        // Title length penalty (very long titles are often less relevant)
        if title.len() > 100 {
            score *= 0.9;
        }

        (score as f64).min(1.0)
    }

    /// Enhanced authority scoring with domain recognition
    fn enhanced_authority_scoring(&self, url: &str, source: &str) -> f64 {
        let mut score = 0.5; // Base score

        // Source-specific bonuses
        match source {
            "Wikipedia" => score += 0.3,
            "Google Scholar" => score += 0.4,
            "Stack Overflow" => score += 0.3,
            "GitHub" => score += 0.2,
            _ => {}
        }

        // Domain authority bonuses
        if let Some(domain) = self.extract_domain(url) {
            let domain_lower = domain.to_lowercase();
            match domain_lower.as_str() {
                d if d.contains("edu") => score += 0.2,
                d if d.contains("gov") => score += 0.2,
                d if d.contains("org") => score += 0.1,
                "github.com" => score += 0.15,
                "stackoverflow.com" => score += 0.15,
                "wikipedia.org" => score += 0.2,
                "mozilla.org" => score += 0.15,
                _ => {}
            }
        }

        // HTTPS bonus
        if url.starts_with("https://") {
            score += 0.05;
        }

        (score as f64).min(1.0)
    }

    /// Normalize URL for deduplication
    fn normalize_url(&self, url: &str) -> String {
        url.trim_end_matches('/')
            .replace("http://", "https://")
            .replace("www.", "")
            .to_lowercase()
    }

    /// Normalize title for deduplication
    fn normalize_title(&self, title: &str) -> String {
        title.trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> Option<String> {
        if let Ok(parsed_url) = Url::parse(url) {
            parsed_url.host_str().map(|h| h.to_string())
        } else {
            None
        }
    }
}