use super::core::{ToolExecutor, ToolResult};
use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::{Client, Response};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use url::Url;

/// Enhanced web performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPerformanceMetrics {
    pub dns_lookup_time: Option<Duration>,
    pub connection_time: Option<Duration>,
    pub ssl_handshake_time: Option<Duration>,
    pub first_byte_time: Option<Duration>,
    pub download_time: Duration,
    pub total_time: Duration,
    pub response_size: usize,
    pub redirects: u32,
}

/// Enhanced web scraping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebScrapingConfig {
    pub max_concurrent_requests: usize,
    pub request_delay_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub timeout_seconds: u64,
    pub user_agent: String,
    pub respect_robots_txt: bool,
    pub extract_metadata: bool,
    pub extract_images: bool,
    pub extract_links: bool,
    pub validate_ssl: bool,
}

impl Default for WebScrapingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 5,
            request_delay_ms: 1000,
            follow_redirects: true,
            max_redirects: 5,
            timeout_seconds: 30,
            user_agent: "Mozilla/5.0 (Compatible Web Scraper)".to_string(),
            respect_robots_txt: true,
            extract_metadata: true,
            extract_images: false,
            extract_links: true,
            validate_ssl: true,
        }
    }
}

/// Enhanced web page content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedWebContent {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: String,
    pub cleaned_content: String,
    pub word_count: usize,
    pub reading_time_minutes: u32,
    pub metadata: HashMap<String, String>,
    pub links: Vec<ExtractedLink>,
    pub images: Vec<ExtractedImage>,
    pub performance: WebPerformanceMetrics,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub cache_control: Option<String>,
}

/// Extracted link information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLink {
    pub url: String,
    pub text: String,
    pub link_type: LinkType,
    pub is_external: bool,
}

/// Extracted image information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    pub url: String,
    pub alt_text: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size_bytes: Option<u64>,
}

/// Type of link found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    Navigation,
    Content,
    External,
    Download,
    Email,
    Phone,
}

impl ToolExecutor {
    /// Enhanced web scraping with comprehensive content extraction
    pub async fn enhanced_web_scrape(
        &self,
        url: &str,
        config: Option<WebScrapingConfig>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_default();
        println!("{} Enhanced web scraping: {}", "🌐".cyan(), url.yellow());

        let start_time = Instant::now();
        
        // Validate URL
        let parsed_url = Url::parse(url)?;
        
        // Check robots.txt if required
        if config.respect_robots_txt {
            if let Err(e) = self.check_robots_txt(&parsed_url).await {
                println!("{} Robots.txt check failed: {}", "⚠".yellow(), e);
            }
        }

        // Build request with enhanced headers
        let client = self.build_enhanced_client(&config)?;
        let response = self.make_timed_request(&client, url, &config).await?;
        
        let total_time = start_time.elapsed();
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        
        // Extract response metadata
        let content_type = headers.get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
            
        let last_modified = headers.get("last-modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));
            
        let cache_control = headers.get("cache-control")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Get response body
        let html_content = response.text().await?;
        let response_size = html_content.len();
        
        // Parse HTML content
        let document = Html::parse_document(&html_content);
        
        // Extract comprehensive content
        let enhanced_content = self.extract_enhanced_content(
            &document,
            url,
            &config,
            WebPerformanceMetrics {
                dns_lookup_time: None,
                connection_time: None,
                ssl_handshake_time: None,
                first_byte_time: None,
                download_time: total_time,
                total_time,
                response_size,
                redirects: 0,
            },
            status_code,
            content_type,
            last_modified,
            cache_control,
        ).await?;

        Ok(ToolResult {
            success: true,
            output: self.format_enhanced_content(&enhanced_content),
            error: None,
            metadata: Some(serde_json::to_value(&enhanced_content)?),
            web_search_result: None,
        })
    }

    /// Build enhanced HTTP client with comprehensive configuration
    fn build_enhanced_client(&self, config: &WebScrapingConfig) -> Result<Client> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(config.max_redirects as usize)
            } else {
                reqwest::redirect::Policy::none()
            });

        if !config.validate_ssl {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        client_builder.build().map_err(|e| anyhow!("Failed to build HTTP client: {}", e))
    }

    /// Make timed HTTP request with performance tracking
    async fn make_timed_request(
        &self,
        client: &Client,
        url: &str,
        config: &WebScrapingConfig,
    ) -> Result<Response> {
        let request = client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1");

        let response = timeout(
            Duration::from_secs(config.timeout_seconds),
            request.send()
        ).await??;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP request failed with status: {}", response.status()));
        }

        Ok(response)
    }

    /// Check robots.txt compliance
    async fn check_robots_txt(&self, url: &Url) -> Result<()> {
        let robots_url = format!("{}://{}/robots.txt", url.scheme(), url.host_str().unwrap_or(""));
        
        match self.web_client.get(&robots_url).send().await {
            Ok(response) if response.status().is_success() => {
                let robots_content = response.text().await?;
                
                // Basic robots.txt parsing (simplified)
                for line in robots_content.lines() {
                    let line = line.trim();
                    if line.starts_with("Disallow:") {
                        let disallowed = line.strip_prefix("Disallow:").unwrap_or("").trim();
                        if !disallowed.is_empty() && url.path().starts_with(disallowed) {
                            return Err(anyhow!("Path disallowed by robots.txt: {}", disallowed));
                        }
                    }
                }
            }
            _ => {
                // If robots.txt doesn't exist or is inaccessible, we proceed
            }
        }

        Ok(())
    }

    /// Extract enhanced content from HTML document
    async fn extract_enhanced_content(
        &self,
        document: &Html,
        url: &str,
        config: &WebScrapingConfig,
        performance: WebPerformanceMetrics,
        status_code: u16,
        content_type: Option<String>,
        last_modified: Option<chrono::DateTime<chrono::Utc>>,
        cache_control: Option<String>,
    ) -> Result<EnhancedWebContent> {
        // Extract title
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|element| element.text().collect::<String>().trim().to_string());

        // Extract meta description
        let description = document
            .select(&Selector::parse("meta[name='description']").unwrap())
            .next()
            .and_then(|element| element.value().attr("content"))
            .map(|s| s.to_string());

        // Extract main content
        let content = self.extract_main_content(document);
        let cleaned_content = self.clean_content(&content);
        let word_count = cleaned_content.split_whitespace().count();
        let reading_time_minutes = ((word_count as f64 / 200.0).ceil() as u32).max(1);

        // Extract metadata
        let mut metadata = HashMap::new();
        if config.extract_metadata {
            self.extract_metadata(document, &mut metadata);
        }

        // Extract links
        let links = if config.extract_links {
            self.extract_links(document, url).await
        } else {
            Vec::new()
        };

        // Extract images
        let images = if config.extract_images {
            self.extract_images(document, url).await
        } else {
            Vec::new()
        };

        // Detect language
        let language = self.detect_language(document);

        Ok(EnhancedWebContent {
            url: url.to_string(),
            title,
            description,
            content,
            cleaned_content,
            word_count,
            reading_time_minutes,
            metadata,
            links,
            images,
            performance,
            status_code,
            content_type,
            language,
            last_modified,
            cache_control,
        })
    }

    /// Extract main content from various content selectors
    fn extract_main_content(&self, document: &Html) -> String {
        let content_selectors = [
            "main",
            "article",
            ".content",
            "#content",
            ".post-content",
            ".entry-content",
            "section",
            ".main-content",
            "body",
        ];

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if text.trim().len() > 100 {
                        return text;
                    }
                }
            }
        }

        // Fallback to body text
        document
            .select(&Selector::parse("body").unwrap())
            .next()
            .map(|element| element.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    }

    /// Clean content by removing extra whitespace and unwanted characters
    fn clean_content(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?;:-()[]{}\"'".contains(*c))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Extract metadata from meta tags and structured data
    fn extract_metadata(&self, document: &Html, metadata: &mut HashMap<String, String>) {
        // Extract meta tags
        for element in document.select(&Selector::parse("meta").unwrap()) {
            if let (Some(name), Some(content)) = (
                element.value().attr("name").or_else(|| element.value().attr("property")),
                element.value().attr("content")
            ) {
                metadata.insert(name.to_string(), content.to_string());
            }
        }

        // Extract Open Graph data
        for element in document.select(&Selector::parse("meta[property^='og:']").unwrap()) {
            if let (Some(property), Some(content)) = (
                element.value().attr("property"),
                element.value().attr("content")
            ) {
                metadata.insert(property.to_string(), content.to_string());
            }
        }

        // Extract Twitter Card data
        for element in document.select(&Selector::parse("meta[name^='twitter:']").unwrap()) {
            if let (Some(name), Some(content)) = (
                element.value().attr("name"),
                element.value().attr("content")
            ) {
                metadata.insert(name.to_string(), content.to_string());
            }
        }
    }

    /// Extract all links from the document
    async fn extract_links(&self, document: &Html, base_url: &str) -> Vec<ExtractedLink> {
        let mut links = Vec::new();
        let base = Url::parse(base_url).ok();

        for element in document.select(&Selector::parse("a[href]").unwrap()) {
            if let Some(href) = element.value().attr("href") {
                let text = element.text().collect::<String>().trim().to_string();
                
                let absolute_url = if let Some(base) = &base {
                    base.join(href).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string())
                } else {
                    href.to_string()
                };

                let link_type = self.classify_link(&absolute_url, &text);
                let is_external = self.is_external_link(&absolute_url, base_url);

                links.push(ExtractedLink {
                    url: absolute_url,
                    text,
                    link_type,
                    is_external,
                });
            }
        }

        links
    }

    /// Extract all images from the document
    async fn extract_images(&self, document: &Html, base_url: &str) -> Vec<ExtractedImage> {
        let mut images = Vec::new();
        let base = Url::parse(base_url).ok();

        for element in document.select(&Selector::parse("img[src]").unwrap()) {
            if let Some(src) = element.value().attr("src") {
                let absolute_url = if let Some(base) = &base {
                    base.join(src).map(|u| u.to_string()).unwrap_or_else(|_| src.to_string())
                } else {
                    src.to_string()
                };

                let alt_text = element.value().attr("alt").map(|s| s.to_string());
                
                let width = element.value().attr("width")
                    .and_then(|w| w.parse::<u32>().ok());
                    
                let height = element.value().attr("height")
                    .and_then(|h| h.parse::<u32>().ok());

                images.push(ExtractedImage {
                    url: absolute_url,
                    alt_text,
                    width,
                    height,
                    size_bytes: None, // Would require additional request to get size
                });
            }
        }

        images
    }

    /// Classify link type based on URL and text
    fn classify_link(&self, url: &str, text: &str) -> LinkType {
        let url_lower = url.to_lowercase();
        let text_lower = text.to_lowercase();

        if url_lower.starts_with("mailto:") {
            LinkType::Email
        } else if url_lower.starts_with("tel:") {
            LinkType::Phone
        } else if url_lower.contains("download") || text_lower.contains("download") {
            LinkType::Download
        } else if url_lower.starts_with("http") && !url_lower.contains(&self.extract_domain_from_url(url).unwrap_or_default()) {
            LinkType::External
        } else if text_lower.contains("nav") || text_lower.contains("menu") {
            LinkType::Navigation
        } else {
            LinkType::Content
        }
    }

    /// Check if link is external
    fn is_external_link(&self, url: &str, base_url: &str) -> bool {
        if let (Ok(link_url), Ok(base_url)) = (Url::parse(url), Url::parse(base_url)) {
            link_url.host() != base_url.host()
        } else {
            false
        }
    }

    /// Extract domain from URL
    fn extract_domain_from_url(&self, url: &str) -> Option<String> {
        Url::parse(url).ok()?.host_str().map(|s| s.to_string())
    }

    /// Detect document language
    fn detect_language(&self, document: &Html) -> Option<String> {
        // Check html lang attribute
        if let Some(element) = document.select(&Selector::parse("html").unwrap()).next() {
            if let Some(lang) = element.value().attr("lang") {
                return Some(lang.to_string());
            }
        }

        // Check meta tags
        for element in document.select(&Selector::parse("meta[http-equiv='content-language']").unwrap()) {
            if let Some(content) = element.value().attr("content") {
                return Some(content.to_string());
            }
        }

        None
    }

    /// Format enhanced content for display
    fn format_enhanced_content(&self, content: &EnhancedWebContent) -> String {
        let mut output = Vec::new();

        output.push(format!("📄 Enhanced Web Content Analysis"));
        output.push("=".repeat(50));
        
        output.push(format!("🔗 URL: {}", content.url));
        output.push(format!("📊 Status: {} {}", 
            if content.status_code < 400 { "✅" } else { "❌" },
            content.status_code
        ));

        if let Some(title) = &content.title {
            output.push(format!("📰 Title: {}", title));
        }

        if let Some(description) = &content.description {
            output.push(format!("📝 Description: {}", description));
        }

        output.push(format!("📊 Content Statistics:"));
        output.push(format!("   • Word Count: {}", content.word_count));
        output.push(format!("   • Reading Time: {} minutes", content.reading_time_minutes));
        output.push(format!("   • Response Size: {} bytes", content.performance.response_size));
        output.push(format!("   • Load Time: {:.2}s", content.performance.total_time.as_secs_f64()));

        if let Some(content_type) = &content.content_type {
            output.push(format!("   • Content Type: {}", content_type));
        }

        if let Some(language) = &content.language {
            output.push(format!("   • Language: {}", language));
        }

        if !content.links.is_empty() {
            output.push(format!("🔗 Links Found: {}", content.links.len()));
            let external_links = content.links.iter().filter(|l| l.is_external).count();
            output.push(format!("   • External Links: {}", external_links));
        }

        if !content.images.is_empty() {
            output.push(format!("🖼️ Images Found: {}", content.images.len()));
        }

        if !content.metadata.is_empty() {
            output.push(format!("📋 Metadata: {} entries", content.metadata.len()));
        }

        // Show content preview
        output.push("📖 Content Preview:".to_string());
        let preview = if content.cleaned_content.len() > 500 {
            format!("{}...", content.cleaned_content.chars().take(500).collect::<String>())
        } else {
            content.cleaned_content.clone()
        };
        output.push(preview);

        output.join("\n")
    }

    /// Batch web scraping for multiple URLs
    pub async fn batch_web_scrape(
        &self,
        urls: Vec<String>,
        config: Option<WebScrapingConfig>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_default();
        println!("{} Batch web scraping {} URLs", "🌐".cyan(), urls.len());

        let all_results = Vec::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(config.max_concurrent_requests));

        let mut tasks = Vec::new();
        for url in urls {
            let permit = semaphore.clone().acquire_owned().await?;
            let config = config.clone();
            let url_clone = url.clone();
            
            let task = tokio::spawn(async move {
                let _permit = permit;
                tokio::time::sleep(Duration::from_millis(config.request_delay_ms)).await;
                // Since we can't clone ToolExecutor, we'll create a simplified version
                let client = reqwest::Client::new();
                match client.get(&url_clone).send().await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        if status < 400 {
                            Ok(format!("✅ Successfully scraped: {}", url_clone))
                        } else {
                            Err(format!("❌ HTTP error {}: {}", status, url_clone))
                        }
                    }
                    Err(e) => Err(format!("❌ Request failed: {} - {}", url_clone, e))
                }
            });
            
            tasks.push(task);
        }

        for task in tasks {
            match task.await? {
                Ok(result_msg) => {
                    println!("{}", result_msg);
                }
                Err(error_msg) => {
                    println!("{}", error_msg);
                }
            }
        }

        let summary = self.create_batch_summary(&all_results);

        Ok(ToolResult {
            success: true,
            output: summary,
            error: None,
            metadata: Some(serde_json::to_value(&all_results)?),
            web_search_result: None,
        })
    }

    /// Create summary for batch scraping results
    fn create_batch_summary(&self, results: &[EnhancedWebContent]) -> String {
        let mut output = Vec::new();

        output.push(format!("📊 Batch Web Scraping Summary"));
        output.push("=".repeat(50));
        
        output.push(format!("Total URLs Processed: {}", results.len()));
        
        let successful = results.iter().filter(|r| r.status_code < 400).count();
        output.push(format!("Successful: {}", successful));
        output.push(format!("Failed: {}", results.len() - successful));

        let total_words: usize = results.iter().map(|r| r.word_count).sum();
        output.push(format!("Total Words Extracted: {}", total_words));

        let total_links: usize = results.iter().map(|r| r.links.len()).sum();
        output.push(format!("Total Links Found: {}", total_links));

        let total_images: usize = results.iter().map(|r| r.images.len()).sum();
        output.push(format!("Total Images Found: {}", total_images));

        let avg_load_time: f64 = results.iter()
            .map(|r| r.performance.total_time.as_secs_f64())
            .sum::<f64>() / results.len() as f64;
        output.push(format!("Average Load Time: {:.2}s", avg_load_time));

        output.push("\n📄 Individual Results:".to_string());
        for (i, result) in results.iter().enumerate() {
            output.push(format!("{}. {} [{}] - {} words", 
                i + 1, 
                result.title.as_deref().unwrap_or("No title"),
                result.status_code,
                result.word_count
            ));
        }

        output.join("\n")
    }
}