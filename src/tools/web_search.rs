use crate::tools::core::{
    WebSearchConfig, WebSearchResult, SearchResultItem, Citation, SearchMetadata, 
    SearchContextSize, UserLocation
};
use anyhow::Result;
use colored::Colorize;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::time::timeout;
use url::Url;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Search performance metrics
#[derive(Debug, Default)]
pub struct SearchMetrics {
    pub total_searches: AtomicUsize,
    pub successful_searches: AtomicUsize,
    pub failed_searches: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub total_results_found: AtomicUsize,
    pub average_response_time_ms: AtomicUsize,
}

/// Search analytics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalytics {
    pub total_searches: usize,
    pub successful_searches: usize,
    pub failed_searches: usize,
    pub success_rate: f64,
    pub cache_hit_rate: f64,
    pub average_results_per_search: f64,
    pub average_response_time_ms: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
}

/// Main WebSearch engine following industry standards (Claude/OpenAI style)
pub struct WebSearchEngine {
    client: Arc<Client>,
    config: WebSearchConfig,
    cache: Arc<tokio::sync::RwLock<HashMap<String, (WebSearchResult, SystemTime)>>>,
    metrics: Arc<SearchMetrics>,
}

/// Query intent classification for intelligent search
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryIntent {
    Factual,        // "What is the capital of France?"
    Tutorial,       // "How to install Docker"
    Comparison,     // "React vs Vue performance"
    Technical,      // "Rust async await syntax"
    News,          // "Latest AI developments"
    Academic,      // "Machine learning research papers"
    Shopping,      // "Best laptops 2024"
    Local,         // "Restaurants near me"
    Troubleshooting, // "Fix Python import error"
    General,       // Default fallback
}

/// Search engine implementations with specializations
#[derive(Debug, Clone)]
pub enum SearchEngine {
    DuckDuckGo,
    Bing,
    Wikipedia,
    StackOverflow,
    GitHub,
    Reddit,
    ArXiv,         // Academic papers
    MDN,           // Web development docs
    RustDocs,      // Rust documentation
}

/// Quality assessment metrics for search results
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    pub content_depth: f64,      // 0.0-1.0
    pub source_authority: f64,   // 0.0-1.0
    pub freshness: f64,          // 0.0-1.0
    pub relevance: f64,          // 0.0-1.0
    pub readability: f64,        // 0.0-1.0
    pub completeness: f64,       // 0.0-1.0
}

/// Progressive search strategy
#[derive(Debug, Clone)]
pub struct SearchStrategy {
    pub intent: QueryIntent,
    pub engines: Vec<SearchEngine>,
    pub max_iterations: usize,
    pub quality_threshold: f64,
    pub enable_refinement: bool,
}

impl WebSearchEngine {
    /// Create a new WebSearch engine with configuration
    pub fn new(config: WebSearchConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.max_uses as u64 * 10)) // Generous timeout
            .user_agent("Mozilla/5.0 (compatible; AI-Assistant/1.0)")
            .build()
            .unwrap_or_default();

        Self {
            client: Arc::new(client),
            config,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            metrics: Arc::new(SearchMetrics::default()),
        }
    }

    /// Main search method - industry standard interface with progressive search
    pub async fn search(
        &self,
        query: &str,
        max_uses: Option<usize>,
        allowed_domains: Option<&[String]>,
        blocked_domains: Option<&[String]>,
        user_location: Option<&UserLocation>,
    ) -> Result<WebSearchResult> {
        let start_time = Instant::now();
        
        // Update metrics
        self.metrics.total_searches.fetch_add(1, Ordering::Relaxed);
        
        // Check cache first
        if let Some(cached) = self.get_cached_result(query).await {
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }

        // Detect query intent and create search strategy
        let intent = self.detect_query_intent(query);
        let strategy = self.create_search_strategy(&intent, max_uses);
        
        println!("{} Intelligent search for: \"{}\" (Intent: {:?})", "🔍".cyan(), query, intent);

        // Execute progressive search
        self.execute_progressive_search(query, strategy, allowed_domains, blocked_domains, user_location, start_time).await

    }

    /// Execute progressive search with intent-aware strategy
    async fn execute_progressive_search(
        &self,
        query: &str,
        strategy: SearchStrategy,
        allowed_domains: Option<&[String]>,
        blocked_domains: Option<&[String]>,
        user_location: Option<&UserLocation>,
        start_time: Instant,
    ) -> Result<WebSearchResult> {
        let mut all_results = Vec::new();
        let mut search_queries_used = vec![query.to_string()];
        let mut searches_performed = 0;
        let mut refined_queries: Vec<String> = Vec::new();

        // Phase 1: Initial broad search across primary engines
        println!("{} Phase 1: Broad search across {} engines", "📡".blue(), strategy.engines.len());
        
        for engine in &strategy.engines {
            if searches_performed >= strategy.max_iterations {
                break;
            }

            if let Ok(results) = self.search_engine(engine, query, user_location).await {
                let filtered_results = self.filter_results(
                    results, 
                    allowed_domains, 
                    blocked_domains
                );
                all_results.extend(filtered_results);
                searches_performed += 1;
                
                println!("{} {} search completed: {} results", "✓".green(), self.engine_name(engine), all_results.len());
            }
        }

        // Phase 2: Quality assessment and potential refinement
        if strategy.enable_refinement && searches_performed < strategy.max_iterations {
            let avg_quality = self.assess_result_quality(&all_results, &strategy.intent);
            
            if avg_quality < strategy.quality_threshold {
                println!("{} Phase 2: Refining search (quality: {:.2})", "🔄".yellow(), avg_quality);
                
                // Generate refined queries based on initial results
                refined_queries = self.generate_refined_queries(query, &all_results, &strategy.intent);
                
                for refined_query in &refined_queries {
                    if searches_performed >= strategy.max_iterations {
                        break;
                    }
                    
                    // Search with refined query on best performing engines
                    for engine in strategy.engines.iter().take(2) {
                        if let Ok(results) = self.search_engine(engine, refined_query, user_location).await {
                            let filtered_results = self.filter_results(
                                results, 
                                allowed_domains, 
                                blocked_domains
                            );
                            all_results.extend(filtered_results);
                            searches_performed += 1;
                            search_queries_used.push(refined_query.to_string());
                        }
                    }
                }
            }
        }

        // Phase 3: Content enhancement and final processing
        println!("{} Phase 3: Content enhancement and ranking", "⚡".yellow());
        
        // Enhance results with content extraction
        if self.config.include_citations {
            self.extract_content_for_results(&mut all_results).await;
        }

        // Advanced scoring with quality metrics
        self.score_results_advanced(&mut all_results, query, &strategy.intent);
        all_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Remove duplicates and apply diversity
        all_results = self.apply_result_diversity(all_results);
        
        // Take top results based on context size
        let result_limit = match self.config.search_context_size {
            SearchContextSize::Low => 5,
            SearchContextSize::Medium => 10,
            SearchContextSize::High => 20,
        };
        all_results.truncate(result_limit);

        // Create enhanced citations
        let citations = self.create_enhanced_citations(&all_results);

        let result = WebSearchResult {
            query_used: query.to_string(),
            results: all_results,
            citations,
            search_metadata: SearchMetadata {
                total_searches_performed: searches_performed,
                search_queries_used,
                timestamp: SystemTime::now(),
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            },
        };

        // Update final metrics
        self.metrics.successful_searches.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_results_found.fetch_add(result.results.len(), Ordering::Relaxed);
        self.metrics.average_response_time_ms.store(result.search_metadata.processing_time_ms as usize, Ordering::Relaxed);
        
        // Cache the result
        self.cache_result(query, &result).await;

        println!("{} Search completed: {} results in {}ms", "🎯".green(), result.results.len(), result.search_metadata.processing_time_ms);
        
        Ok(result)
    }

    /// Detect query intent using pattern matching and keywords
    fn detect_query_intent(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();
        
        // Tutorial/How-to patterns
        if query_lower.contains("how to") || query_lower.contains("tutorial") || 
           query_lower.contains("guide") || query_lower.contains("install") ||
           query_lower.contains("setup") || query_lower.contains("configure") {
            return QueryIntent::Tutorial;
        }
        
        // Technical patterns
        if query_lower.contains("error") || query_lower.contains("fix") ||
           query_lower.contains("debug") || query_lower.contains("troubleshoot") {
            return QueryIntent::Troubleshooting;
        }
        
        // Comparison patterns
        if query_lower.contains(" vs ") || query_lower.contains(" versus ") ||
           query_lower.contains("compare") || query_lower.contains("difference") ||
           query_lower.contains("better") {
            return QueryIntent::Comparison;
        }
        
        // Technical development patterns
        if query_lower.contains("rust") || query_lower.contains("python") ||
           query_lower.contains("javascript") || query_lower.contains("api") ||
           query_lower.contains("code") || query_lower.contains("programming") {
            return QueryIntent::Technical;
        }
        
        // News patterns
        if query_lower.contains("latest") || query_lower.contains("news") ||
           query_lower.contains("2024") || query_lower.contains("recent") ||
           query_lower.contains("update") {
            return QueryIntent::News;
        }
        
        // Academic patterns
        if query_lower.contains("research") || query_lower.contains("paper") ||
           query_lower.contains("study") || query_lower.contains("analysis") {
            return QueryIntent::Academic;
        }
        
        // Shopping patterns
        if query_lower.contains("best") || query_lower.contains("buy") ||
           query_lower.contains("price") || query_lower.contains("review") ||
           query_lower.contains("cheap") {
            return QueryIntent::Shopping;
        }
        
        // Factual question patterns
        if query_lower.starts_with("what") || query_lower.starts_with("who") ||
           query_lower.starts_with("when") || query_lower.starts_with("where") ||
           query_lower.starts_with("why") {
            return QueryIntent::Factual;
        }
        
        QueryIntent::General
    }
    
    /// Create search strategy based on query intent
    fn create_search_strategy(&self, intent: &QueryIntent, max_uses: Option<usize>) -> SearchStrategy {
        let max_iterations = max_uses.unwrap_or(self.config.max_uses);
        
        match intent {
            QueryIntent::Technical | QueryIntent::Troubleshooting => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::StackOverflow, SearchEngine::GitHub, SearchEngine::DuckDuckGo, SearchEngine::RustDocs],
                max_iterations,
                quality_threshold: 0.7,
                enable_refinement: true,
            },
            QueryIntent::Tutorial => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::DuckDuckGo, SearchEngine::Bing, SearchEngine::GitHub, SearchEngine::MDN],
                max_iterations,
                quality_threshold: 0.6,
                enable_refinement: true,
            },
            QueryIntent::Academic => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::ArXiv, SearchEngine::Wikipedia, SearchEngine::DuckDuckGo],
                max_iterations,
                quality_threshold: 0.8,
                enable_refinement: false,
            },
            QueryIntent::News => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::DuckDuckGo, SearchEngine::Bing, SearchEngine::Reddit],
                max_iterations,
                quality_threshold: 0.5,
                enable_refinement: true,
            },
            QueryIntent::Factual => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::Wikipedia, SearchEngine::DuckDuckGo, SearchEngine::Bing],
                max_iterations,
                quality_threshold: 0.7,
                enable_refinement: false,
            },
            _ => SearchStrategy {
                intent: intent.clone(),
                engines: vec![SearchEngine::DuckDuckGo, SearchEngine::Bing, SearchEngine::Wikipedia],
                max_iterations,
                quality_threshold: 0.6,
                enable_refinement: true,
            },
        }
    }
    
    /// Get engine name for logging
    fn engine_name(&self, engine: &SearchEngine) -> &'static str {
        match engine {
            SearchEngine::DuckDuckGo => "DuckDuckGo",
            SearchEngine::Bing => "Bing",
            SearchEngine::Wikipedia => "Wikipedia",
            SearchEngine::StackOverflow => "StackOverflow",
            SearchEngine::GitHub => "GitHub",
            SearchEngine::Reddit => "Reddit",
            SearchEngine::ArXiv => "ArXiv",
            SearchEngine::MDN => "MDN",
            SearchEngine::RustDocs => "Rust Docs",
        }
    }

    /// Search a specific engine with retry logic and error handling
    async fn search_engine(
        &self,
        engine: &SearchEngine,
        query: &str,
        user_location: Option<&UserLocation>,
    ) -> Result<Vec<SearchResultItem>> {
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            let result = match engine {
                SearchEngine::DuckDuckGo => self.search_duckduckgo(query, user_location).await,
                SearchEngine::Bing => self.search_bing(query, user_location).await,
                SearchEngine::Wikipedia => self.search_wikipedia(query).await,
                SearchEngine::StackOverflow => self.search_stackoverflow(query).await,
                SearchEngine::GitHub => self.search_github(query).await,
                SearchEngine::Reddit => self.search_reddit(query).await,
                SearchEngine::ArXiv => self.search_arxiv(query).await,
                SearchEngine::MDN => self.search_mdn(query).await,
                SearchEngine::RustDocs => self.search_rust_docs(query).await,
            };
            
            match result {
                Ok(results) => {
                    if attempt > 1 {
                        println!("{} {} search succeeded on attempt {}", "✓".green(), self.engine_name(engine), attempt);
                    }
                    return Ok(results);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(500 * attempt as u64);
                        println!("{} {} search failed (attempt {}), retrying in {}ms...", 
                                "⚠".yellow(), self.engine_name(engine), attempt, delay.as_millis());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        // If all retries failed, return the last error
        if let Some(error) = last_error {
            println!("{} {} search failed after {} attempts: {}", 
                    "✗".red(), self.engine_name(engine), max_retries, error);
            Err(error)
        } else {
            Err(anyhow::anyhow!("Search failed for unknown reasons"))
        }
    }

    /// DuckDuckGo search implementation with enhanced error handling
    async fn search_duckduckgo(
        &self,
        query: &str,
        user_location: Option<&UserLocation>,
    ) -> Result<Vec<SearchResultItem>> {
        let mut url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        
        if let Some(location) = user_location {
            if let Some(country) = &location.country {
                url.push_str(&format!("&kl={}", country.to_lowercase()));
            }
        }

        let response = timeout(
            Duration::from_secs(15),
            self.client.get(&url)
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("DNT", "1")
                .header("Connection", "keep-alive")
                .send()
        ).await
        .map_err(|_| anyhow::anyhow!("DuckDuckGo search timeout"))?
        .map_err(|e| anyhow::anyhow!("DuckDuckGo request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("DuckDuckGo responded with status: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| anyhow::anyhow!("Failed to read DuckDuckGo response: {}", e))?;
            
        if html.len() < 100 {
            return Err(anyhow::anyhow!("DuckDuckGo returned insufficient content"));
        }
            
        self.parse_duckduckgo_results(&html)
    }

    /// Bing search implementation
    async fn search_bing(
        &self,
        query: &str,
        user_location: Option<&UserLocation>,
    ) -> Result<Vec<SearchResultItem>> {
        let mut url = format!("https://www.bing.com/search?q={}", urlencoding::encode(query));
        
        if let Some(location) = user_location {
            if let Some(country) = &location.country {
                url.push_str(&format!("&cc={}", country.to_uppercase()));
            }
        }

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let html = response.text().await?;
        self.parse_bing_results(&html)
    }

    /// Wikipedia search implementation
    async fn search_wikipedia(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit=5&format=json",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let json: serde_json::Value = response.json().await?;
        self.parse_wikipedia_results(&json)
    }

    /// StackOverflow search implementation
    async fn search_stackoverflow(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://api.stackexchange.com/2.3/search/advanced?order=desc&sort=relevance&q={}&site=stackoverflow",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let json: serde_json::Value = response.json().await?;
        self.parse_stackoverflow_results(&json)
    }

    /// GitHub search implementation
    async fn search_github(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=stars&order=desc",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let json: serde_json::Value = response.json().await?;
        self.parse_github_results(&json)
    }

    /// Reddit search implementation  
    async fn search_reddit(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://www.reddit.com/search.json?q={}&sort=relevance&limit=10",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let json: serde_json::Value = response.json().await?;
        self.parse_reddit_results(&json)
    }

    /// ArXiv search implementation
    async fn search_arxiv(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results=10",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let xml = response.text().await?;
        self.parse_arxiv_results(&xml)
    }

    /// MDN search implementation
    async fn search_mdn(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://developer.mozilla.org/api/v1/search?q={}",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let json: serde_json::Value = response.json().await?;
        self.parse_mdn_results(&json)
    }

    /// Rust documentation search implementation
    async fn search_rust_docs(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let url = format!(
            "https://doc.rust-lang.org/search.html?search={}",
            urlencoding::encode(query)
        );

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await??;

        let html = response.text().await?;
        self.parse_rust_docs_results(&html)
    }

    /// Parse DuckDuckGo HTML results with enhanced error handling
    fn parse_duckduckgo_results(&self, html: &str) -> Result<Vec<SearchResultItem>> {
        let document = Html::parse_document(html);
        
        // Try multiple selectors for robustness
        let selectors = vec![
            (".result", ".result__title a", ".result__snippet"),
            (".web-result", ".result__a", ".result__snippet"),
            (".links_main", "h2 a", ".snippet"),
        ];
        
        let mut results = Vec::new();
        
        for (result_sel, title_sel, snippet_sel) in selectors {
            if let (Ok(result_selector), Ok(title_selector), Ok(snippet_selector)) = (
                Selector::parse(result_sel),
                Selector::parse(title_sel),
                Selector::parse(snippet_sel)
            ) {
                for result in document.select(&result_selector) {
                    if let Some(title_elem) = result.select(&title_selector).next() {
                        let title = title_elem.text().collect::<String>().trim().to_string();
                        let url = title_elem.value().attr("href").unwrap_or("").to_string();
                        
                        let snippet = result.select(&snippet_selector)
                            .next()
                            .map(|elem| elem.text().collect::<String>().trim().to_string())
                            .filter(|s| !s.is_empty());

                        if !title.is_empty() && !url.is_empty() && (url.starts_with("http") || url.starts_with("//")) {
                            let normalized_url = if url.starts_with("//") {
                                format!("https:{}", url)
                            } else {
                                url
                            };
                            
                            let domain = Url::parse(&normalized_url)
                                .map(|u| u.host_str().unwrap_or("unknown").to_string())
                                .unwrap_or_else(|_| "unknown".to_string());

                            // Skip if we already have this URL
                            if !results.iter().any(|r: &SearchResultItem| r.url == normalized_url) {
                                results.push(SearchResultItem {
                                    title,
                                    url: normalized_url,
                                    snippet,
                                    content: None,
                                    relevance_score: 0.8,  // Will be recalculated
                                    source_domain: domain,
                                });
                            }
                        }
                    }
                }
                
                if !results.is_empty() {
                    break; // Found results with this selector set
                }
            }
        }
        
        if results.is_empty() {
            return Err(anyhow::anyhow!("No results found in DuckDuckGo response"));
        }

        Ok(results)
    }

    /// Parse Bing HTML results
    fn parse_bing_results(&self, html: &str) -> Result<Vec<SearchResultItem>> {
        let document = Html::parse_document(html);
        let result_selector = Selector::parse(".b_algo").unwrap();
        let title_selector = Selector::parse("h2 a").unwrap();
        let snippet_selector = Selector::parse(".b_caption p").unwrap();

        let mut results = Vec::new();

        for result in document.select(&result_selector) {
            if let Some(title_elem) = result.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();
                let url = title_elem.value().attr("href").unwrap_or("").to_string();
                
                let snippet = result.select(&snippet_selector)
                    .next()
                    .map(|elem| elem.text().collect::<String>().trim().to_string());

                if !title.is_empty() && !url.is_empty() && url.starts_with("http") {
                    let domain = Url::parse(&url)
                        .map(|u| u.host_str().unwrap_or("unknown").to_string())
                        .unwrap_or_else(|_| "unknown".to_string());

                    results.push(SearchResultItem {
                        title,
                        url,
                        snippet,
                        content: None,
                        relevance_score: 0.8,  // Will be recalculated
                        source_domain: domain,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse Wikipedia API results
    fn parse_wikipedia_results(&self, json: &serde_json::Value) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();

        if let Some(array) = json.as_array() {
            if array.len() >= 4 {
                if let (Some(titles), Some(descriptions), Some(urls)) = (
                    array[1].as_array(),
                    array[2].as_array(),
                    array[3].as_array(),
                ) {
                    for ((title, description), url) in titles
                        .iter()
                        .zip(descriptions.iter())
                        .zip(urls.iter())
                    {
                        if let (Some(title), Some(url)) = (title.as_str(), url.as_str()) {
                            let snippet = description.as_str().map(|s| s.to_string());
                            
                            results.push(SearchResultItem {
                                title: title.to_string(),
                                url: url.to_string(),
                                snippet,
                                content: None,
                                relevance_score: 0.9,  // Wikipedia gets high base score
                                source_domain: "wikipedia.org".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Parse StackOverflow API results
    fn parse_stackoverflow_results(&self, json: &serde_json::Value) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();

        if let Some(items) = json["items"].as_array() {
            for item in items.iter().take(10) {
                if let (Some(title), Some(question_id)) = (
                    item["title"].as_str(),
                    item["question_id"].as_u64(),
                ) {
                    let url = format!("https://stackoverflow.com/questions/{}", question_id);
                    let snippet = item["excerpt"].as_str().map(|s| s.to_string());
                    let score = item["score"].as_f64().unwrap_or(0.0) / 100.0 + 0.7;

                    results.push(SearchResultItem {
                        title: title.to_string(),
                        url,
                        snippet,
                        content: None,
                        relevance_score: score.min(1.0),
                        source_domain: "stackoverflow.com".to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse GitHub API results
    fn parse_github_results(&self, json: &serde_json::Value) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();

        if let Some(items) = json["items"].as_array() {
            for item in items.iter().take(10) {
                if let (Some(name), Some(html_url)) = (
                    item["name"].as_str(),
                    item["html_url"].as_str(),
                ) {
                    let description = item["description"].as_str().unwrap_or("");
                    let stars = item["stargazers_count"].as_u64().unwrap_or(0);
                    let score = (stars as f64).log10() / 6.0 + 0.6; // Logarithmic scaling

                    results.push(SearchResultItem {
                        title: format!("{} (GitHub Repository)", name),
                        url: html_url.to_string(),
                        snippet: Some(description.to_string()),
                        content: None,
                        relevance_score: score.min(1.0),
                        source_domain: "github.com".to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse Reddit API results
    fn parse_reddit_results(&self, json: &serde_json::Value) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();

        if let Some(data) = json["data"]["children"].as_array() {
            for item in data.iter().take(10) {
                let post = &item["data"];
                if let (Some(title), Some(permalink)) = (
                    post["title"].as_str(),
                    post["permalink"].as_str(),
                ) {
                    let url = format!("https://reddit.com{}", permalink);
                    let selftext = post["selftext"].as_str().unwrap_or("");
                    let score = post["score"].as_f64().unwrap_or(0.0) / 100.0 + 0.5;
                    
                    let snippet = if selftext.len() > 200 {
                        Some(format!("{}...", &selftext[..200]))
                    } else if !selftext.is_empty() {
                        Some(selftext.to_string())
                    } else {
                        None
                    };

                    results.push(SearchResultItem {
                        title: title.to_string(),
                        url,
                        snippet,
                        content: None,
                        relevance_score: score.min(1.0),
                        source_domain: "reddit.com".to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse ArXiv XML results (simplified)
    fn parse_arxiv_results(&self, xml: &str) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();
        
        // Simple regex-based parsing for ArXiv XML
        let title_regex = Regex::new(r"<title>(.*?)</title>").unwrap();
        let id_regex = Regex::new(r"<id>(.*?)</id>").unwrap();
        let summary_regex = Regex::new(r"<summary>(.*?)</summary>").unwrap();
        
        let titles: Vec<_> = title_regex.captures_iter(xml).collect();
        let ids: Vec<_> = id_regex.captures_iter(xml).collect();
        let summaries: Vec<_> = summary_regex.captures_iter(xml).collect();
        
        for i in 0..titles.len().min(10) {
            if let (Some(title), Some(id)) = (titles.get(i), ids.get(i)) {
                let title_text = title[1].trim();
                let id_text = &id[1];
                let summary_text = summaries.get(i).map(|s| s[1].trim().to_string());
                
                if !title_text.is_empty() && title_text != "ArXiv Query Interface" {
                    results.push(SearchResultItem {
                        title: title_text.to_string(),
                        url: id_text.to_string(),
                        snippet: summary_text,
                        content: None,
                        relevance_score: 0.85,
                        source_domain: "arxiv.org".to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse MDN API results
    fn parse_mdn_results(&self, json: &serde_json::Value) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();

        if let Some(documents) = json["documents"].as_array() {
            for doc in documents.iter().take(10) {
                if let (Some(title), Some(mdn_url)) = (
                    doc["title"].as_str(),
                    doc["mdn_url"].as_str(),
                ) {
                    let summary = doc["summary"].as_str().unwrap_or("");
                    let url = format!("https://developer.mozilla.org{}", mdn_url);

                    results.push(SearchResultItem {
                        title: title.to_string(),
                        url,
                        snippet: Some(summary.to_string()),
                        content: None,
                        relevance_score: 0.85,
                        source_domain: "developer.mozilla.org".to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse Rust documentation results (simplified HTML parsing)
    fn parse_rust_docs_results(&self, html: &str) -> Result<Vec<SearchResultItem>> {
        let mut results = Vec::new();
        let document = Html::parse_document(html);
        
        // This is simplified - real implementation would need proper selectors
        if let Ok(link_selector) = Selector::parse("a[href*='doc.rust-lang.org']") {
            for element in document.select(&link_selector).take(10) {
                if let Some(href) = element.value().attr("href") {
                    let title = element.text().collect::<String>().trim().to_string();
                    if !title.is_empty() {
                        results.push(SearchResultItem {
                            title,
                            url: href.to_string(),
                            snippet: Some("Rust documentation".to_string()),
                            content: None,
                            relevance_score: 0.8,
                            source_domain: "doc.rust-lang.org".to_string(),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Filter results based on domain restrictions
    fn filter_results(
        &self,
        results: Vec<SearchResultItem>,
        allowed_domains: Option<&[String]>,
        blocked_domains: Option<&[String]>,
    ) -> Vec<SearchResultItem> {
        results
            .into_iter()
            .filter(|result| {
                // Check blocked domains first
                if let Some(blocked) = blocked_domains {
                    if blocked.iter().any(|domain| result.source_domain.contains(domain)) {
                        return false;
                    }
                }

                // Check allowed domains if specified
                if let Some(allowed) = allowed_domains {
                    if !allowed.is_empty() {
                        return allowed.iter().any(|domain| result.source_domain.contains(domain));
                    }
                }

                true
            })
            .collect()
    }

    /// Extract content from result URLs for better context
    async fn extract_content_for_results(&self, results: &mut [SearchResultItem]) {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(3)); // Limit concurrent requests
        let mut tasks = Vec::new();

        for result in results.iter_mut() {
            let client = self.client.clone();
            let url = result.url.clone();
            let permit = semaphore.clone();

            tasks.push(async move {
                let _permit = permit.acquire().await;
                Self::extract_content(&client, &url).await
            });
        }

        let contents = futures::future::join_all(tasks).await;
        
        for (result, content) in results.iter_mut().zip(contents.into_iter()) {
            if let Ok(Some(extracted_content)) = content {
                result.content = Some(extracted_content);
            }
        }
    }

    /// Extract text content from a webpage
    async fn extract_content(client: &Client, url: &str) -> Result<Option<String>> {
        let response = timeout(
            Duration::from_secs(8),
            client.get(url).send()
        ).await??;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Try multiple content selectors
        let content_selectors = [
            "main", "article", ".content", "#content", ".post-content", 
            ".article-content", "p", ".entry-content"
        ];

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let content: String = document
                    .select(&selector)
                    .map(|elem| elem.text().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ");

                if content.len() > 100 {
                    let truncated = if content.len() > 1500 {
                        format!("{}...", &content[..1500])
                    } else {
                        content
                    };
                    return Ok(Some(truncated.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Score and rank search results
    fn score_results(&self, results: &mut [SearchResultItem], query: &str) {
        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower.split_whitespace().collect();

        for result in results.iter_mut() {
            let mut score = 0.5; // Base score

            // Title relevance (most important)
            let title_lower = result.title.to_lowercase();
            let title_words: HashSet<&str> = title_lower.split_whitespace().collect();
            let title_intersection = query_words.intersection(&title_words).count();
            score += (title_intersection as f64 / query_words.len() as f64) * 0.4;

            // Snippet relevance
            if let Some(snippet) = &result.snippet {
                let snippet_lower = snippet.to_lowercase();
                let snippet_words: HashSet<&str> = snippet_lower.split_whitespace().collect();
                let snippet_intersection = query_words.intersection(&snippet_words).count();
                score += (snippet_intersection as f64 / query_words.len() as f64) * 0.2;
            }

            // Domain authority boost
            if result.source_domain.contains("wikipedia.org") {
                score += 0.2;
            } else if result.source_domain.contains("github.com") || 
                     result.source_domain.contains("stackoverflow.com") {
                score += 0.15;
            } else if result.source_domain.contains("mozilla.org") || 
                     result.source_domain.contains("w3.org") {
                score += 0.1;
            }

            result.relevance_score = score.min(1.0);
        }
    }

    /// Advanced scoring with quality metrics and intent awareness
    fn score_results_advanced(&self, results: &mut [SearchResultItem], query: &str, intent: &QueryIntent) {
        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower.split_whitespace().collect();

        for result in results.iter_mut() {
            let quality_metrics = self.assess_individual_quality(result, intent);
            let mut score = 0.3; // Base score

            // Title relevance (weighted by intent)
            let title_lower = result.title.to_lowercase();
            let title_words: HashSet<&str> = title_lower.split_whitespace().collect();
            let title_intersection = query_words.intersection(&title_words).count();
            let title_score = (title_intersection as f64 / query_words.len() as f64) * 0.35;
            score += title_score;

            // Snippet relevance
            if let Some(snippet) = &result.snippet {
                let snippet_lower = snippet.to_lowercase();
                let snippet_words: HashSet<&str> = snippet_lower.split_whitespace().collect();
                let snippet_intersection = query_words.intersection(&snippet_words).count();
                score += (snippet_intersection as f64 / query_words.len() as f64) * 0.2;
            }

            // Intent-specific domain boosting
            score += self.get_intent_domain_boost(&result.source_domain, intent);

            // Quality metrics integration
            score += quality_metrics.source_authority * 0.15;
            score += quality_metrics.content_depth * 0.1;
            score += quality_metrics.freshness * 0.05;

            result.relevance_score = score.min(1.0);
        }
    }

    /// Assess quality of individual search result
    fn assess_individual_quality(&self, result: &SearchResultItem, intent: &QueryIntent) -> QualityMetrics {
        let mut content_depth: f64 = 0.5;
        let mut source_authority: f64 = 0.5;
        let freshness: f64 = 0.7; // Default
        let relevance: f64 = result.relevance_score;
        let readability: f64 = 0.6;
        let completeness: f64 = 0.5;

        // Content depth assessment
        if let Some(snippet) = &result.snippet {
            content_depth = (snippet.len() as f64 / 500.0).min(1.0);
            
            // Check for technical indicators
            if snippet.contains("example") || snippet.contains("code") {
                content_depth += 0.2;
            }
        }

        // Source authority based on domain
        source_authority = match result.source_domain.as_str() {
            "wikipedia.org" => 0.95,
            "stackoverflow.com" => 0.9,
            "github.com" => 0.85,
            "mozilla.org" | "developer.mozilla.org" => 0.9,
            "doc.rust-lang.org" => 0.9,
            "arxiv.org" => 0.95,
            _ if result.source_domain.ends_with(".edu") => 0.8,
            _ if result.source_domain.ends_with(".gov") => 0.85,
            _ => 0.5,
        };

        // Intent-specific quality adjustments
        match intent {
            QueryIntent::Academic => {
                if result.source_domain.contains("arxiv") || result.source_domain.ends_with(".edu") {
                    source_authority += 0.1;
                }
            },
            QueryIntent::Technical => {
                if result.source_domain.contains("stackoverflow") || result.source_domain.contains("github") {
                    source_authority += 0.1;
                }
            },
            _ => {}
        }

        QualityMetrics {
            content_depth: content_depth.min(1.0),
            source_authority: source_authority.min(1.0),
            freshness,
            relevance,
            readability,
            completeness,
        }
    }

    /// Get domain boost based on query intent
    fn get_intent_domain_boost(&self, domain: &str, intent: &QueryIntent) -> f64 {
        match intent {
            QueryIntent::Technical | QueryIntent::Troubleshooting => {
                if domain.contains("stackoverflow.com") { 0.3 }
                else if domain.contains("github.com") { 0.25 }
                else if domain.contains("docs.rs") || domain.contains("doc.rust-lang.org") { 0.2 }
                else { 0.0 }
            },
            QueryIntent::Academic => {
                if domain.contains("arxiv.org") { 0.3 }
                else if domain.ends_with(".edu") { 0.25 }
                else if domain.contains("wikipedia.org") { 0.2 }
                else { 0.0 }
            },
            QueryIntent::Tutorial => {
                if domain.contains("github.com") { 0.2 }
                else if domain.contains("mozilla.org") { 0.25 }
                else { 0.0 }
            },
            QueryIntent::News => {
                if domain.contains("reddit.com") { 0.2 }
                else { 0.0 }
            },
            _ => 0.0
        }
    }

    /// Assess overall result quality for a set of results
    fn assess_result_quality(&self, results: &[SearchResultItem], intent: &QueryIntent) -> f64 {
        if results.is_empty() {
            return 0.0;
        }

        let total_quality: f64 = results.iter()
            .map(|result| {
                let metrics = self.assess_individual_quality(result, intent);
                (metrics.source_authority + metrics.content_depth + metrics.relevance) / 3.0
            })
            .sum();

        total_quality / results.len() as f64
    }

    /// Generate refined queries based on initial results
    fn generate_refined_queries(&self, original_query: &str, results: &[SearchResultItem], intent: &QueryIntent) -> Vec<String> {
        let mut refined_queries = Vec::new();
        
        // Extract common terms from good results
        let mut term_frequency: HashMap<String, usize> = HashMap::new();
        
        for result in results.iter().take(3) {
            if result.relevance_score > 0.7 {
                // Extract terms from title and snippet
                let text = format!("{} {}", result.title, result.snippet.as_deref().unwrap_or(""));
                let words: Vec<&str> = text.split_whitespace().collect();
                
                for word in words {
                    let clean_word = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                    if clean_word.len() > 3 && !original_query.to_lowercase().contains(&clean_word) {
                        *term_frequency.entry(clean_word).or_insert(0) += 1;
                    }
                }
            }
        }
        
        // Create refined queries with most common terms
        let mut common_terms: Vec<_> = term_frequency.into_iter().collect();
        common_terms.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (term, _) in common_terms.iter().take(2) {
            refined_queries.push(format!("{} {}", original_query, term));
        }
        
        // Intent-specific query refinements
        match intent {
            QueryIntent::Tutorial => {
                refined_queries.push(format!("{} tutorial guide", original_query));
                refined_queries.push(format!("how to {} step by step", original_query));
            },
            QueryIntent::Troubleshooting => {
                refined_queries.push(format!("{} solution fix", original_query));
                refined_queries.push(format!("{} error resolved", original_query));
            },
            QueryIntent::Technical => {
                refined_queries.push(format!("{} documentation", original_query));
                refined_queries.push(format!("{} API reference", original_query));
            },
            _ => {}
        }
        
        refined_queries.truncate(3);
        refined_queries
    }

    /// Apply diversity to results to avoid domain over-representation
    fn apply_result_diversity(&self, mut results: Vec<SearchResultItem>) -> Vec<SearchResultItem> {
        let mut diverse_results = Vec::new();
        let mut domain_count: HashMap<String, usize> = HashMap::new();
        let max_per_domain = 3;
        
        for result in results.drain(..) {
            let count = domain_count.get(&result.source_domain).unwrap_or(&0);
            if *count < max_per_domain {
                domain_count.insert(result.source_domain.clone(), count + 1);
                diverse_results.push(result);
            }
        }
        
        diverse_results
    }

    /// Create enhanced citations with better metadata
    fn create_enhanced_citations(&self, results: &[SearchResultItem]) -> Vec<Citation> {
        results
            .iter()
            .take(8) // More citations for comprehensive referencing
            .enumerate()
            .map(|(i, result)| Citation {
                url: result.url.clone(),
                title: format!("[{}] {}", i + 1, result.title),
                domain: result.source_domain.clone(),
                excerpt: result.snippet.clone(),
            })
            .collect()
    }

    /// Create citations from search results
    fn create_citations(&self, results: &[SearchResultItem]) -> Vec<Citation> {
        results
            .iter()
            .take(5) // Limit citations to top 5 results
            .map(|result| Citation {
                url: result.url.clone(),
                title: result.title.clone(),
                domain: result.source_domain.clone(),
                excerpt: result.snippet.clone(),
            })
            .collect()
    }

    /// Get cached search result
    async fn get_cached_result(&self, query: &str) -> Option<WebSearchResult> {
        let cache = self.cache.read().await;
        if let Some((result, timestamp)) = cache.get(query) {
            if timestamp.elapsed().unwrap_or(Duration::from_secs(3600)).as_secs() < 3600 {
                return Some(result.clone());
            }
        }
        None
    }

    /// Cache search result
    async fn cache_result(&self, query: &str, result: &WebSearchResult) {
        let mut cache = self.cache.write().await;
        cache.insert(query.to_string(), (result.clone(), SystemTime::now()));
        
        // Simple cache cleanup - keep only last 100 entries
        if cache.len() > 100 {
            let oldest_key = cache
                .iter()
                .min_by_key(|(_, (_, timestamp))| timestamp)
                .map(|(key, _)| key.clone());
            
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }
    }

    /// Legacy method for backward compatibility
    pub async fn enhanced_search(&self, query: &str, _include_specialized: bool) -> Result<WebSearchResult> {
        self.search(query, None, None, None, None).await
    }

    /// Extract content from a single page URL
    pub async fn extract_page_content(&self, url: &str) -> Result<String> {
        match Self::extract_content(&self.client, url).await {
            Ok(Some(content)) => Ok(content),
            Ok(None) => Ok("No content could be extracted from this page.".to_string()),
            Err(e) => Ok(format!("Error extracting content: {}", e)),
        }
    }
    
    /// Get comprehensive search analytics
    pub fn get_analytics(&self) -> SearchAnalytics {
        let total = self.metrics.total_searches.load(Ordering::Relaxed);
        let successful = self.metrics.successful_searches.load(Ordering::Relaxed);
        let failed = self.metrics.failed_searches.load(Ordering::Relaxed);
        let cache_hits = self.metrics.cache_hits.load(Ordering::Relaxed);
        let total_results = self.metrics.total_results_found.load(Ordering::Relaxed);
        let avg_time = self.metrics.average_response_time_ms.load(Ordering::Relaxed);
        
        SearchAnalytics {
            total_searches: total,
            successful_searches: successful,
            failed_searches: failed,
            success_rate: if total > 0 { successful as f64 / total as f64 } else { 0.0 },
            cache_hit_rate: if total > 0 { cache_hits as f64 / total as f64 } else { 0.0 },
            average_results_per_search: if successful > 0 { total_results as f64 / successful as f64 } else { 0.0 },
            average_response_time_ms: avg_time as u64,
        }
    }
    
    /// Print detailed analytics report
    pub fn print_analytics_report(&self) {
        let analytics = self.get_analytics();
        
        println!("\n{} WebSearch Analytics Report", "📈".cyan().bold());
        println!("{}", "=".repeat(40));
        println!("{} Total Searches: {}", "🔍", analytics.total_searches);
        println!("{} Successful: {} ({:.1}%)", "✓".green(), analytics.successful_searches, analytics.success_rate * 100.0);
        println!("{} Failed: {}", "✗".red(), analytics.failed_searches);
        println!("{} Cache Hit Rate: {:.1}%", "💾", analytics.cache_hit_rate * 100.0);
        println!("{} Avg Results/Search: {:.1}", "🎯", analytics.average_results_per_search);
        println!("{} Avg Response Time: {}ms", "⚡", analytics.average_response_time_ms);
        println!("{}", "=".repeat(40));
    }
    
    /// Reset analytics counters
    pub fn reset_analytics(&self) {
        self.metrics.total_searches.store(0, Ordering::Relaxed);
        self.metrics.successful_searches.store(0, Ordering::Relaxed);
        self.metrics.failed_searches.store(0, Ordering::Relaxed);
        self.metrics.cache_hits.store(0, Ordering::Relaxed);
        self.metrics.total_results_found.store(0, Ordering::Relaxed);
        self.metrics.average_response_time_ms.store(0, Ordering::Relaxed);
    }
    
    /// Clear search cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        println!("{} Cleared {} cached search results", "🗑".yellow(), count);
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let _now = SystemTime::now();
        let mut expired_count = 0;
        
        for (_, (_, timestamp)) in cache.iter() {
            if timestamp.elapsed().unwrap_or(Duration::from_secs(3600)).as_secs() >= 3600 {
                expired_count += 1;
            }
        }
        
        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired_count,
            valid_entries: cache.len() - expired_count,
        }
    }
    
    /// Cleanup expired cache entries
    pub async fn cleanup_expired_cache(&self) {
        let mut cache = self.cache.write().await;
        let _now = SystemTime::now();
        let initial_count = cache.len();
        
        cache.retain(|_, (_, timestamp)| {
            timestamp.elapsed().unwrap_or(Duration::from_secs(3600)).as_secs() < 3600
        });
        
        let removed = initial_count - cache.len();
        if removed > 0 {
            println!("{} Cleaned up {} expired cache entries", "🧯".yellow(), removed);
        }
    }
}

/// Convenience function for formatted search results
pub fn format_search_results(result: &WebSearchResult) -> String {
    let mut output = String::new();
    
    output.push_str(&format!(
        "{} Search Results for: \"{}\"\n\n",
        "🔍".green(),
        result.query_used
    ));

    for (i, item) in result.results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} {}\n   {}\n   {}\n",
            i + 1,
            item.title.bold(),
            format!("(Score: {:.2})", item.relevance_score).dimmed(),
            item.url.blue().underline(),
            item.snippet.as_deref().unwrap_or("No description available").italic()
        ));

        if let Some(content) = &item.content {
            let preview = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.clone()
            };
            output.push_str(&format!("   📄 {}\n", preview.dimmed()));
        }
        output.push('\n');
    }

    if !result.citations.is_empty() {
        output.push_str(&format!("\n{} Sources:\n", "📚".cyan()));
        for citation in &result.citations {
            output.push_str(&format!(
                "• {} - {}\n",
                citation.title.bold(),
                citation.url.blue()
            ));
        }
    }

    output.push_str(&format!(
        "\n{} Searched {} engines in {}ms\n",
        "⚡".yellow(),
        result.search_metadata.total_searches_performed,
        result.search_metadata.processing_time_ms
    ));

    output
}

/// Get fallback resources when search fails
pub fn get_fallback_resources(query: &str) -> Vec<SearchResultItem> {
    vec![
        SearchResultItem {
            title: "DuckDuckGo Search".to_string(),
            url: format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
            snippet: Some("Search DuckDuckGo directly for your query".to_string()),
            content: None,
            relevance_score: 0.6,
            source_domain: "duckduckgo.com".to_string(),
        },
        SearchResultItem {
            title: "Wikipedia Search".to_string(),
            url: format!("https://en.wikipedia.org/wiki/Special:Search?search={}", urlencoding::encode(query)),
            snippet: Some("Search Wikipedia for encyclopedia articles".to_string()),
            content: None,
            relevance_score: 0.7,
            source_domain: "wikipedia.org".to_string(),
        },
    ]
}