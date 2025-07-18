use colored::Colorize;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use super::core::{EditOperation, ToolExecutor, ToolResult};
use super::search::{enhanced_file_search, ErrorStrategy, SearchQuery, ToolChain};

impl ToolExecutor {
    // Enhanced web search implementation with multiple sources and content scraping
    pub async fn web_search(&self, query: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Searching web for: {}", "🔍".cyan(), query.yellow());

        let mut all_results = Vec::new();
        let mut search_success = false;
        let mut search_errors = Vec::new();

        // Try multiple search engines for better results
        let search_engines = vec![
            ("Wikipedia", self.search_wikipedia(query).await),
            ("Bing", self.search_bing(query).await),
            ("DuckDuckGo", self.search_duckduckgo(query).await),
        ];

        for (engine_name, search_result) in search_engines {
            match search_result {
                Ok(results) => {
                    if !results.is_empty() {
                        all_results.extend(results);
                        search_success = true;
                        println!(
                            "{} Found {} results from {}",
                            "✓".green(),
                            all_results.len(),
                            engine_name
                        );
                    }
                }
                Err(e) => {
                    search_errors.push(format!("{}: {}", engine_name, e));
                    println!("{} {} search failed: {}", "✗".red(), engine_name, e);
                }
            }
        }

        // Try our alternative search method if we don't have enough results
        if all_results.len() < 2 {
            if let Ok(alt_results) = self.alternative_search(query).await {
                if !alt_results.is_empty() {
                    all_results.extend(alt_results);
                    search_success = true;
                    println!("{} Found results from alternative search", "✓".green());
                }
            }
        }

        // Remove duplicates and rank results
        let deduplicated_results = self.deduplicate_and_rank_results(all_results);

        // Extract URLs from the results and scrape content
        let mut content_results = Vec::new();
        let mut scraping_errors = Vec::new();

        for result in deduplicated_results.iter().take(3) {
            // Limit to top 3 results to avoid overwhelming
            if let Some(url) = self.extract_url_from_result(result) {
                println!("{} Scraping content from: {}", "📄".cyan(), url);

                match self.scrape_url_content(&url).await {
                    Ok(content) => {
                        if !content.is_empty() {
                            let title = self.extract_title_from_result(result);
                            content_results.push(format!(
                                "🔗 **{}**\nURL: {}\nContent: {}",
                                title, url, content
                            ));
                        }
                    }
                    Err(e) => {
                        scraping_errors.push(format!("Failed to scrape {}: {}", url, e));
                        println!("{} Failed to scrape {}: {}", "✗".red(), url, e);
                    }
                }
            }
        }

        // If we have content, use it; otherwise fall back to search results
        let final_output = if !content_results.is_empty() {
            format!(
                "Search Results with Content for '{}':\n\n{}",
                query,
                content_results.join("\n\n---\n\n")
            )
        } else {
            format!(
                "Search Results for '{}':\n\n{}{}",
                query,
                deduplicated_results.join("\n\n"),
                if !scraping_errors.is_empty() {
                    format!(
                        "\n\nNote: Content scraping failed for some URLs: {}",
                        scraping_errors.join("; ")
                    )
                } else {
                    String::new()
                }
            )
        };

        Ok(ToolResult {
            success: search_success,
            output: if deduplicated_results.is_empty() {
                // Provide helpful information even when search engines fail
                let fallback_msg = if self.is_programming_query(query) {
                    self.get_programming_resources(query).join("\n")
                } else if self.is_educational_query(query) {
                    self.get_educational_resources(query).join("\n")
                } else {
                    format!(
                        "Search engines are currently unavailable, but here are some general resources:\n{}",
                        self.get_educational_resources(query).join("\n")
                    )
                };
                
                format!(
                    "Search completed for '{}' but search engines returned limited results.\n\nHere are some relevant resources:\n{}\n\nTechnical details: {}",
                    query,
                    fallback_msg,
                    search_errors.join("; ")
                )
            } else {
                final_output
            },
            error: if search_success {
                None
            } else {
                Some(format!(
                    "All search engines failed: {}",
                    search_errors.join("; ")
                ))
            },
            metadata: Some(serde_json::json!({
                "query": query,
                "total_results": deduplicated_results.len(),
                "content_results": content_results.len(),
                "search_engines_used": ["DuckDuckGo", "Bing", "Wikipedia"],
                "search_errors": search_errors,
                "scraping_errors": scraping_errors
            })),
        })
    }

    // DuckDuckGo search with improved parsing
    async fn search_duckduckgo(
        &self,
        query: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Use the correct DuckDuckGo HTML search endpoint
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .web_client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);
        let mut results = Vec::new();

        // DuckDuckGo-specific selectors based on their HTML structure
        let selector = Selector::parse(".result__title .result__a").unwrap();
        
        for element in document.select(&selector).take(10) {
            if let Some(href) = element.value().attr("href") {
                // Get title text
                let title_text = element.text().collect::<String>();
                let clean_title = title_text.trim();
                
                // Skip empty titles or very short ones
                if clean_title.is_empty() || clean_title.len() < 3 {
                    continue;
                }
                
                // Extract the actual URL from DuckDuckGo's redirect
                let actual_url = if href.starts_with("//duckduckgo.com/l/?uddg=") {
                    // Extract the encoded URL from the redirect
                    if let Some(start) = href.find("uddg=") {
                        let encoded_url = &href[start + 5..];
                        if let Some(end) = encoded_url.find('&') {
                            urlencoding::decode(&encoded_url[..end])
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| href.to_string())
                        } else {
                            urlencoding::decode(encoded_url)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| href.to_string())
                        }
                    } else {
                        href.to_string()
                    }
                } else if href.starts_with("http") {
                    href.to_string()
                } else {
                    continue; // Skip relative URLs
                };
                
                // Skip ads and internal links
                if clean_title.to_lowercase().contains("ad") && href.contains("ad_domain") {
                    continue;
                }
                
                // Format result
                results.push(format!("📄 {}: {}", clean_title, actual_url));
            }
            
            if results.len() >= 5 {
                break;
            }
        }

        Ok(results)
    }

    // Bing search implementation
    async fn search_bing(&self, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let search_url = format!(
            "https://www.bing.com/search?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .web_client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);
        let mut results = Vec::new();

        // Bing-specific selectors - focus on the main algorithm results
        let selector = Selector::parse(".b_algo h2 a").unwrap();
        
        for element in document.select(&selector).take(10) {
            if let Some(href) = element.value().attr("href") {
                // Get title text
                let title_text = element.text().collect::<String>();
                let clean_title = title_text.trim();
                
                // Skip empty titles or very short ones
                if clean_title.is_empty() || clean_title.len() < 3 {
                    continue;
                }
                
                // Skip non-URL hrefs or internal links
                if !href.starts_with("http") || href.contains("bing.com") || href.contains("microsoft.com") {
                    continue;
                }
                
                // Format result
                results.push(format!("🔍 {}: {}", clean_title, href));
            }
            
            if results.len() >= 5 {
                break;
            }
        }

        Ok(results)
    }

    // Wikipedia search for factual queries
    async fn search_wikipedia(
        &self,
        query: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let api_url = format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            urlencoding::encode(query)
        );

        let response = self
            .web_client
            .get(&api_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Assistant/1.0)")
            .send()
            .await?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;

            if let Some(extract) = json.get("extract").and_then(|v| v.as_str()) {
                if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                    if let Some(page_url) = json
                        .get("content_urls")
                        .and_then(|v| v.get("desktop"))
                        .and_then(|v| v.get("page"))
                        .and_then(|v| v.as_str())
                    {
                        let result = format!(
                            "📚 Wikipedia - {}: {}\n   Summary: {}",
                            title, page_url, extract
                        );
                        return Ok(vec![result]);
                    }
                }
            }
        }

        Ok(vec![])
    }

    // Alternative search method using various APIs and known sources
    async fn alternative_search(&self, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        // For programming-related queries, try to provide helpful links
        if self.is_programming_query(query) {
            results.extend(self.get_programming_resources(query));
        }
        
        // For general queries, provide some educational resources
        if self.is_educational_query(query) {
            results.extend(self.get_educational_resources(query));
        }
        
        // For technical queries, provide documentation links
        if self.is_technical_query(query) {
            results.extend(self.get_technical_resources(query));
        }
        
        Ok(results)
    }

    fn is_programming_query(&self, query: &str) -> bool {
        let programming_keywords = [
            "programming", "code", "rust", "python", "javascript", "java", "c++", "golang", 
            "development", "software", "algorithm", "function", "variable", "syntax", "compile",
            "debug", "library", "framework", "api", "database", "sql", "html", "css", "react",
            "vue", "angular", "node", "npm", "cargo", "git", "github", "tutorial", "learn"
        ];
        
        let query_lower = query.to_lowercase();
        programming_keywords.iter().any(|keyword| query_lower.contains(keyword))
    }

    fn is_educational_query(&self, query: &str) -> bool {
        let educational_keywords = [
            "what is", "how to", "learn", "tutorial", "guide", "explain", "definition",
            "meaning", "example", "basics", "introduction", "beginner", "course"
        ];
        
        let query_lower = query.to_lowercase();
        educational_keywords.iter().any(|keyword| query_lower.contains(keyword))
    }

    fn is_technical_query(&self, query: &str) -> bool {
        let technical_keywords = [
            "documentation", "docs", "specification", "standard", "rfc", "manual",
            "reference", "config", "setup", "install", "command", "cli", "terminal"
        ];
        
        let query_lower = query.to_lowercase();
        technical_keywords.iter().any(|keyword| query_lower.contains(keyword))
    }

    fn get_programming_resources(&self, query: &str) -> Vec<String> {
        let mut resources = Vec::new();
        let query_lower = query.to_lowercase();
        
        if query_lower.contains("rust") {
            resources.push("📘 The Rust Programming Language: https://doc.rust-lang.org/book/".to_string());
            resources.push("📚 Rust by Example: https://doc.rust-lang.org/rust-by-example/".to_string());
            resources.push("🦀 Rust Standard Library: https://doc.rust-lang.org/std/".to_string());
        }
        
        if query_lower.contains("python") {
            resources.push("🐍 Python Official Documentation: https://docs.python.org/3/".to_string());
            resources.push("📖 Python Tutorial: https://docs.python.org/3/tutorial/".to_string());
        }
        
        if query_lower.contains("javascript") {
            resources.push("📝 MDN JavaScript Guide: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide".to_string());
            resources.push("⚡ JavaScript Reference: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference".to_string());
        }
        
        if query_lower.contains("git") {
            resources.push("📚 Pro Git Book: https://git-scm.com/book".to_string());
            resources.push("🔧 Git Documentation: https://git-scm.com/docs".to_string());
        }
        
        resources
    }

    fn get_educational_resources(&self, _query: &str) -> Vec<String> {
        vec![
            "🎓 freeCodeCamp: https://www.freecodecamp.org/".to_string(),
            "📚 Khan Academy: https://www.khanacademy.org/".to_string(),
            "💻 Codecademy: https://www.codecademy.com/".to_string(),
        ]
    }

    fn get_technical_resources(&self, query: &str) -> Vec<String> {
        let mut resources = Vec::new();
        let query_lower = query.to_lowercase();
        
        if query_lower.contains("linux") || query_lower.contains("unix") {
            resources.push("📖 Linux Documentation: https://www.kernel.org/doc/html/latest/".to_string());
            resources.push("🐧 Linux Man Pages: https://man7.org/linux/man-pages/".to_string());
        }
        
        if query_lower.contains("docker") {
            resources.push("🐳 Docker Documentation: https://docs.docker.com/".to_string());
        }
        
        if query_lower.contains("kubernetes") {
            resources.push("☸️ Kubernetes Documentation: https://kubernetes.io/docs/".to_string());
        }
        
        resources
    }

    // Check if query is factual (names, dates, biographical info)
    fn is_factual_query(&self, query: &str) -> bool {
        let factual_keywords = [
            "born",
            "birthday",
            "birth",
            "when",
            "who",
            "what",
            "where",
            "how old",
            "age",
            "biography",
            "life",
            "career",
            "education",
            "family",
            "died",
            "death",
        ];

        let query_lower = query.to_lowercase();
        factual_keywords
            .iter()
            .any(|keyword| query_lower.contains(keyword))
    }

    // Remove duplicates and rank results by relevance
    fn deduplicate_and_rank_results(&self, results: Vec<String>) -> Vec<String> {
        let mut unique_results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();
        let mut seen_titles = std::collections::HashSet::new();

        for result in results {
            // Extract URL for URL-based deduplication
            let url = self.extract_url_from_result(&result).unwrap_or_default();
            
            // Extract title for title-based deduplication
            let title = if let Some(colon_pos) = result.find(": ") {
                result[..colon_pos].to_string()
            } else {
                result.clone()
            };

            // Clean title by removing emoji and extra whitespace
            let clean_title = title
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .trim()
                .to_lowercase();

            // Check for duplicates by both URL and title
            let is_duplicate = if !url.is_empty() && seen_urls.contains(&url) {
                true
            } else if !clean_title.is_empty() && seen_titles.contains(&clean_title) {
                true
            } else {
                false
            };

            if !is_duplicate {
                if !url.is_empty() {
                    seen_urls.insert(url);
                }
                if !clean_title.is_empty() {
                    seen_titles.insert(clean_title);
                }
                unique_results.push(result);
            }
        }

        // Prioritize authoritative sources
        unique_results.sort_by(|a, b| {
            let a_score = self.get_source_authority_score(a);
            let b_score = self.get_source_authority_score(b);
            b_score.cmp(&a_score)
        });

        unique_results.into_iter().take(8).collect()
    }

    // Score sources by authority (Wikipedia, gov sites, edu sites, etc.)
    fn get_source_authority_score(&self, result: &str) -> i32 {
        let mut score = 0;

        if result.contains("wikipedia.org") {
            score += 10;
        }
        if result.contains(".gov") {
            score += 8;
        }
        if result.contains(".edu") {
            score += 7;
        }
        if result.contains("britannica.com") {
            score += 6;
        }
        if result.contains("imdb.com") {
            score += 5;
        }
        if result.contains("📚") {
            score += 5; // Wikipedia results
        }

        score
    }

    // Extract URL from search result string
    fn extract_url_from_result(&self, result: &str) -> Option<String> {
        // Our new format is "📄 Title: URL" or "🔍 Title: URL"
        if let Some(colon_pos) = result.find(": ") {
            let url_part = &result[colon_pos + 2..];
            
            // Find the end of the URL by looking for whitespace or newline
            let url_end = url_part
                .find(|c: char| c.is_whitespace() || c == '\n' || c == '\r')
                .unwrap_or(url_part.len());
            
            let url = url_part[..url_end].trim().to_string();
            
            // Validate URL format
            if url.starts_with("http://") || url.starts_with("https://") {
                Some(url)
            } else {
                None
            }
        } else {
            // Fallback: look for URL patterns anywhere in the string
            if let Some(url_start) = result.find("http") {
                let url_slice = &result[url_start..];
                let url_end = url_slice
                    .find(|c: char| c.is_whitespace() || c == '\n' || c == '\r')
                    .unwrap_or(url_slice.len());
                
                let url = result[url_start..url_start + url_end].to_string();
                
                if url.starts_with("http://") || url.starts_with("https://") {
                    Some(url)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    // Extract title from search result string
    fn extract_title_from_result(&self, result: &str) -> String {
        // Extract title before the first colon, removing emoji prefix
        if let Some(colon_pos) = result.find(": ") {
            let title_part = &result[..colon_pos];
            // Remove emoji prefix (📄 or 🔍)
            if let Some(space_pos) = title_part.find(' ') {
                title_part[space_pos + 1..].trim().to_string()
            } else {
                title_part.trim().to_string()
            }
        } else {
            "Unknown Title".to_string()
        }
    }

    // Scrape content from a URL using existing web_scrape functionality
    async fn scrape_url_content(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("{} Extracting content from: {}", "🔍".cyan(), url);

        // Add basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Invalid URL format".into());
        }

        let response = self
            .web_client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .timeout(std::time::Duration::from_secs(15)) // Increased timeout
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Prioritized content selectors - try most specific first
        let content_selectors = [
            "article",
            ".content",
            "#content", 
            ".post-content",
            ".entry-content",
            "main",
            ".main-content",
            ".article-body",
            ".story-body",
            ".article-text",
            ".post-body",
            "[role='main']",
            ".markdown-body", // GitHub, documentation sites
            ".document", // Documentation sites
        ];

        let mut content = Vec::new();
        let mut found_content = false;

        // Try content selectors in order of specificity
        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let elements: Vec<_> = document.select(&selector).collect();
                
                if !elements.is_empty() {
                    for element in elements.iter().take(3) { // Limit to first 3 matching containers
                        let text: String = element.text().collect::<Vec<_>>().join(" ");
                        let cleaned_text = text
                            .trim()
                            .lines()
                            .map(|line| line.trim())
                            .filter(|line| !line.is_empty() && line.len() > 10)
                            .collect::<Vec<_>>()
                            .join(" ");

                        if !cleaned_text.is_empty() && cleaned_text.len() > 50 {
                            content.push(cleaned_text);
                            found_content = true;
                        }
                    }
                    
                    if found_content && !content.is_empty() {
                        break; // Found good content, stop trying other selectors
                    }
                }
            }
        }

        // Fallback: if no structured content found, try paragraph text
        if !found_content {
            if let Ok(p_selector) = Selector::parse("p") {
                for element in document.select(&p_selector).take(10) {
                    let text: String = element.text().collect::<Vec<_>>().join(" ");
                    let cleaned_text = text.trim();
                    
                    if !cleaned_text.is_empty() && cleaned_text.len() > 30 {
                        content.push(cleaned_text.to_string());
                    }
                }
            }
        }

        // Clean up the extracted content
        let final_content = content.join(" ");
        let cleaned_content = final_content
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?;:()[]{}\"'-".contains(*c))
            .collect::<String>();

        // Limit content length to avoid overwhelming the model
        if cleaned_content.len() > 2000 {
            Ok(format!(
                "{}...",
                cleaned_content.chars().take(2000).collect::<String>()
            ))
        } else {
            Ok(cleaned_content)
        }
    }

    // Web scraping implementation
    pub async fn web_scrape(&self, url: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Scraping URL: {}", "🌐".cyan(), url.yellow());

        let response = self
            .web_client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Assistant/1.0)")
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let content_selectors = [
            "article",
            ".content",
            "#content",
            ".post-content",
            ".entry-content",
            "main",
            "p",
            "h1",
            "h2",
            "h3",
        ];
        let mut content = Vec::new();

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(10) {
                    let text: String = element.text().collect::<Vec<_>>().join(" ");
                    let cleaned_text = text.trim();
                    if cleaned_text.len() > 50 {
                        content.push(cleaned_text.to_string());
                    }
                }
                if content.len() > 3 {
                    break;
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: if content.is_empty() {
                format!(
                    "Successfully accessed {} but could not extract readable content",
                    url
                )
            } else {
                content.join("\n\n")
            },
            error: None,
            metadata: None,
        })
    }

    // File operations - Fuzzy search enabled
    pub fn file_search(
        &self,
        pattern: &str,
        directory: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let search_dir = directory.unwrap_or(".");
        println!(
            "{} Searching for files matching '{}' in {}",
            "📁".cyan(),
            pattern.yellow(),
            search_dir.blue()
        );

        // Use synchronous fuzzy matching implementation
        let mut found_files = Vec::new();
        let search_path = std::path::Path::new(search_dir);

        let pattern_lower = pattern.to_lowercase();

        for entry in WalkDir::new(search_path).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();

                // Skip ignored files/directories
                if self.should_ignore_path(path) {
                    continue;
                }

                // Check filename for fuzzy match
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(score) =
                        self.fuzzy_match_sync(&pattern_lower, &filename.to_lowercase())
                    {
                        found_files.push((path.to_path_buf(), score));
                    }
                }

                // Also check full path for better directory matching
                let full_path = path.to_string_lossy();
                if let Some(score) =
                    self.fuzzy_match_sync(&pattern_lower, &full_path.to_lowercase())
                {
                    // Update score if this is better than filename match
                    if let Some(existing) = found_files.iter_mut().find(|(p, _)| p == path) {
                        if score > existing.1 {
                            existing.1 = score;
                        }
                    } else {
                        found_files.push((path.to_path_buf(), score));
                    }
                }
            }
        }

        // Sort by score (descending)
        found_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Format output
        let mut output = Vec::new();
        for (path, score) in found_files.iter().take(50) {
            output.push(format!("{} (score: {:.2})", path.display(), score));
        }

        Ok(ToolResult {
            success: true,
            output: if output.is_empty() {
                "No files found matching the pattern".to_string()
            } else {
                output.join("\n")
            },
            error: None,
            metadata: None,
        })
    }

    // Enhanced file search with ranking and content search
    pub async fn enhanced_file_search(
        &self,
        pattern: &str,
        directory: Option<&str>,
        search_content: bool,
        is_regex: bool,
        max_results: Option<usize>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let search_dir = Path::new(directory.unwrap_or("."));

        let query = SearchQuery {
            pattern: pattern.to_string(),
            is_regex,
            case_sensitive: false,
            search_content,
            search_filenames: true,
            max_results,
            ..Default::default()
        };

        enhanced_file_search(search_dir, &query).await
    }

    // Tool chain execution for complex file operations
    pub async fn execute_tool_chain(
        &self,
        chain: &ToolChain,
    ) -> Result<Vec<ToolResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for (_index, step) in chain.steps.iter().enumerate() {
            let mut retries = 0;
            let _max_retries = match &chain.error_strategy {
                ErrorStrategy::RetryWithBackoff { max_retries, .. } => *max_retries,
                _ => 0,
            };

            loop {
                let result = self.execute_chain_step(step, &results).await;

                match result {
                    Ok(tool_result) => {
                        results.push(tool_result);
                        break;
                    }
                    Err(e) => match &chain.error_strategy {
                        ErrorStrategy::FailFast => return Err(e),
                        ErrorStrategy::ContinueOnError => {
                            results.push(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                                metadata: None,
                            });
                            break;
                        }
                        ErrorStrategy::RetryWithBackoff {
                            max_retries,
                            backoff_ms,
                        } => {
                            if retries < *max_retries {
                                retries += 1;
                                tokio::time::sleep(tokio::time::Duration::from_millis(*backoff_ms))
                                    .await;
                                continue;
                            } else {
                                return Err(e);
                            }
                        }
                    },
                }
            }
        }

        Ok(results)
    }

    async fn execute_chain_step(
        &self,
        step: &super::search::ChainStep,
        previous_results: &[ToolResult],
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let mut params = step.parameters.clone();

        // If this step depends on a previous result, incorporate it
        if let Some(dep_index) = step.depends_on {
            if let Some(prev_result) = previous_results.get(dep_index) {
                if step.use_previous_result {
                    // Extract the first line as the file path if it's a file search result
                    if step.tool_name == "file_read" {
                        let first_line = prev_result.output.lines().next().unwrap_or("");
                        params.insert("path".to_string(), first_line.to_string());
                    }
                }
            }
        }

        // Execute the appropriate tool based on the step name
        match step.tool_name.as_str() {
            "file_search" => {
                let default_pattern = String::new();
                let pattern = params.get("pattern").unwrap_or(&default_pattern);
                let directory = params.get("directory");
                self.file_search(pattern, directory.map(|s| s.as_str()))
            }
            "file_read" => {
                let default_path = String::new();
                let path = params.get("path").unwrap_or(&default_path);
                self.file_read(path)
            }
            "enhanced_file_search" => {
                let default_pattern = String::new();
                let pattern = params.get("pattern").unwrap_or(&default_pattern);
                let directory = params.get("directory");
                let search_content = params
                    .get("search_content")
                    .map(|s| s.parse().unwrap_or(true))
                    .unwrap_or(true);
                let is_regex = params
                    .get("is_regex")
                    .map(|s| s.parse().unwrap_or(false))
                    .unwrap_or(false);
                let max_results = params.get("max_results").and_then(|s| s.parse().ok());

                self.enhanced_file_search(
                    pattern,
                    directory.map(|s| s.as_str()),
                    search_content,
                    is_regex,
                    max_results,
                )
                .await
            }
            _ => Err(format!("Unknown tool in chain: {}", step.tool_name).into()),
        }
    }

    pub fn file_read(&self, path: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Reading file: {}", "📖".cyan(), path.yellow());

        // Validate path to prevent directory traversal
        let validated_path = match self.validate_path(path) {
            Ok(path) => path,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error),
                    metadata: None,
                });
            }
        };

        match fs::read_to_string(&validated_path) {
            Ok(content) => Ok(ToolResult {
                success: true,
                output: content,
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    pub fn file_write(
        &self,
        path: &str,
        content: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Writing to file: {}", "✏️".cyan(), path.yellow());

        // Validate path to prevent directory traversal
        let validated_path = match self.validate_path(path) {
            Ok(path) => path,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error),
                    metadata: None,
                });
            }
        };

        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::write(&validated_path, content) {
            Ok(_) => Ok(ToolResult {
                success: true,
                output: format!("Successfully wrote {} bytes to {}", content.len(), path),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    pub fn file_edit(
        &self,
        path: &str,
        operation: EditOperation,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Editing file: {}", "✏️".cyan(), path.yellow());

        let current_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Could not read file {}: {}", path, e)),
                    metadata: None,
                });
            }
        };

        let new_content = match operation {
            EditOperation::Replace { ref old, ref new } => {
                if current_content.contains(old) {
                    current_content.replace(old, new)
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Text '{}' not found in file", old)),
                        metadata: None,
                    });
                }
            }
            EditOperation::Insert { line, ref content } => {
                let mut lines: Vec<&str> = current_content.lines().collect();
                if line <= lines.len() {
                    lines.insert(line, content);
                    lines.join("\n")
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "Line {} is out of bounds (file has {} lines)",
                            line,
                            lines.len()
                        )),
                        metadata: None,
                    });
                }
            }
            EditOperation::Append { ref content } => {
                format!("{}\n{}", current_content, content)
            }
            EditOperation::Delete {
                line_start,
                line_end,
            } => {
                let mut lines: Vec<&str> = current_content.lines().collect();
                let end = line_end.unwrap_or(line_start);

                if line_start > 0
                    && line_start <= lines.len()
                    && end <= lines.len()
                    && line_start <= end
                {
                    let start_idx = line_start - 1;
                    let end_idx = end - 1;

                    for _ in start_idx..=end_idx {
                        if start_idx < lines.len() {
                            lines.remove(start_idx);
                        }
                    }
                    lines.join("\n")
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "Invalid line range: {} to {} (file has {} lines)",
                            line_start,
                            end,
                            lines.len()
                        )),
                        metadata: None,
                    });
                }
            }
        };

        match std::fs::write(path, &new_content) {
            Ok(_) => {
                let operation_desc = match operation {
                    EditOperation::Replace { .. } => "Text replaced".to_string(),
                    EditOperation::Insert { line, .. } => {
                        format!("Content inserted at line {}", line)
                    }
                    EditOperation::Append { .. } => "Content appended".to_string(),
                    EditOperation::Delete {
                        line_start,
                        line_end,
                    } => {
                        if let Some(end) = line_end {
                            format!("Lines {} to {} deleted", line_start, end)
                        } else {
                            format!("Line {} deleted", line_start)
                        }
                    }
                };

                Ok(ToolResult {
                    success: true,
                    output: format!("{} in {}", operation_desc, path),
                    error: None,
                    metadata: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Could not write to file {}: {}", path, e)),
                metadata: None,
            }),
        }
    }

    pub fn content_search(
        &self,
        pattern: &str,
        directory: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let search_dir = directory.unwrap_or(".");
        println!(
            "{} Searching for content '{}' in {}",
            "🔍".cyan(),
            pattern.yellow(),
            search_dir.blue()
        );

        let regex = Regex::new(pattern)?;
        let mut results = Vec::new();

        for entry in WalkDir::new(search_dir).follow_links(true) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            results.push(format!(
                                "{}:{}: {}",
                                entry.path().display(),
                                line_num + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: if results.is_empty() {
                "No content found matching the pattern".to_string()
            } else {
                results.join("\n")
            },
            error: None,
            metadata: None,
        })
    }

    fn validate_command(&self, command: &str) -> Result<(), String> {
        // Check for dangerous patterns that could lead to command injection
        let dangerous_patterns = [
            "|",
            "&&",
            "||",
            ";",
            "`",
            "$(",
            "&",
            ">",
            "<",
            ">>",
            "<<",
            "rm -rf /",
            "rm -rf /*",
            ":(){ :|:& };:",
            "curl",
            "wget",
            "nc",
            "netcat",
        ];

        // Check for SQL injection patterns
        let sql_patterns = [
            "DROP TABLE",
            "DELETE FROM",
            "UPDATE",
            "INSERT INTO",
            "CREATE TABLE",
            "ALTER TABLE",
        ];

        // Check for path traversal patterns
        let path_patterns = [
            "../", "..\\", "/etc/", "/var/", "/usr/", "/home/", "C:\\", "~/",
        ];

        let command_lower = command.to_lowercase();

        // Check dangerous command patterns
        for pattern in &dangerous_patterns {
            if command_lower.contains(pattern) {
                return Err(format!(
                    "Command contains potentially dangerous pattern: {}",
                    pattern
                ));
            }
        }

        // Check SQL injection patterns
        for pattern in &sql_patterns {
            if command_lower.contains(&pattern.to_lowercase()) {
                return Err(format!(
                    "Command contains SQL injection pattern: {}",
                    pattern
                ));
            }
        }

        // Check path traversal patterns
        for pattern in &path_patterns {
            if command_lower.contains(pattern) {
                return Err(format!(
                    "Command contains path traversal pattern: {}",
                    pattern
                ));
            }
        }

        // Check for excessively long commands (potential buffer overflow)
        if command.len() > 1000 {
            return Err("Command is too long (potential buffer overflow)".to_string());
        }

        // Check for non-printable characters
        if command
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err("Command contains non-printable characters".to_string());
        }

        Ok(())
    }

    fn validate_path(&self, path: &str) -> Result<std::path::PathBuf, String> {
        use std::path::{Path, PathBuf};

        // Check for obviously malicious patterns
        if path.contains("..") || path.contains("~") {
            return Err("Path contains directory traversal patterns".to_string());
        }

        // Check for access to sensitive directories
        let sensitive_dirs = [
            "/etc",
            "/var",
            "/usr",
            "/boot",
            "/sys",
            "/proc",
            "/dev",
            "/root",
            "/home",
            "C:\\",
            "C:\\Windows",
            "C:\\Program Files",
        ];

        for sensitive_dir in &sensitive_dirs {
            if path.starts_with(sensitive_dir) {
                return Err(format!(
                    "Access to sensitive directory {} is not allowed",
                    sensitive_dir
                ));
            }
        }

        // Canonicalize the path to resolve any remaining traversal attempts
        let path_obj = Path::new(path);
        let canonical_path = match path_obj.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If canonicalization fails, check if parent exists and create safe path
                let parent = path_obj.parent();
                if let Some(parent) = parent {
                    if let Ok(parent_canonical) = parent.canonicalize() {
                        parent_canonical.join(path_obj.file_name().unwrap_or_default())
                    } else {
                        return Err("Invalid path or parent directory".to_string());
                    }
                } else {
                    return Err("Invalid path".to_string());
                }
            }
        };

        // Get current working directory for relative path validation
        let current_dir =
            std::env::current_dir().map_err(|_| "Cannot determine current directory")?;

        // Ensure the canonical path is within the current directory or its subdirectories
        if !canonical_path.starts_with(&current_dir) {
            return Err("Path is outside of allowed directory scope".to_string());
        }

        Ok(canonical_path)
    }

    pub async fn execute_command(
        &self,
        command: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Executing command: {}", "⚡".cyan(), command.yellow());

        // Security validation: Check for dangerous patterns
        if let Err(validation_error) = self.validate_command(command) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(validation_error),
                metadata: None,
            });
        }

        // Check if we're in a TTY environment
        let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdin());

        let mut child = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", command]);

            if is_tty {
                // For TTY environments, inherit stdio to allow interaction
                cmd.stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
            } else {
                // For non-TTY environments, use piped stdio
                cmd.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
            };

            cmd.spawn()?
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", command]);

            if is_tty {
                // For TTY environments, inherit stdio to allow interaction
                cmd.stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
            } else {
                // For non-TTY environments, use piped stdio
                cmd.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
            };

            cmd.spawn()?
        };

        let (output_msg, success) = if is_tty {
            // For TTY environments, just wait for completion
            let status = child.wait()?;
            let msg = format!(
                "Command completed with exit code: {}",
                status.code().unwrap_or(-1)
            );
            (msg, status.success())
        } else {
            // For non-TTY, capture output
            let output = child.wait_with_output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let msg = if !stdout.is_empty() {
                stdout.to_string()
            } else if !stderr.is_empty() {
                stderr.to_string()
            } else {
                format!(
                    "Command completed with exit code: {}",
                    output.status.code().unwrap_or(-1)
                )
            };

            (msg, output.status.success())
        };

        Ok(ToolResult {
            success,
            output: output_msg,
            error: None,
            metadata: None,
        })
    }

    pub async fn generate_command(
        &self,
        user_request: &str,
        context: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Generating command for: {}",
            "🤖".cyan(),
            user_request.yellow()
        );

        // Build the command generation prompt
        let mut prompt = String::new();

        // Add system-level context about the operating system
        let os = std::env::consts::OS;
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        };

        prompt.push_str(&format!(
            "You are a command generation assistant. Generate a single, executable command for the following request.\n\n\
            Operating System: {}\n\
            Shell: {}\n\
            Current Directory: {}\n\n",
            os,
            shell,
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        ));

        // Add context if provided
        if let Some(ctx) = context {
            prompt.push_str(&format!("Context: {}\n\n", ctx));
        }

        prompt.push_str(&format!(
            "User Request: {}\n\n\
            IMPORTANT RULES:\n\
            - Generate ONLY the command, no explanations\n\
            - Use appropriate flags and options\n\
            - Consider safety and common best practices\n\
            - For project initialization, use standard tools (npx, cargo, etc.)\n\
            - For file operations, use standard unix tools (find, grep, ls, etc.)\n\
            - If multiple commands are needed, separate with && or ;\n\
            - Output format: just the command string\n\n\
            Examples:\n\
            Request: \"initialize a next.js project called myapp\"\n\
            Response: npx create-next-app@latest myapp\n\n\
            Request: \"find all Python files in the current directory\"\n\
            Response: find . -name \"*.py\" -type f\n\n\
            Request: \"search for the word 'function' in JavaScript files\"\n\
            Response: grep -r \"function\" --include=\"*.js\" .\n\n\
            Command:",
            user_request
        ));

        Ok(ToolResult {
            success: true,
            output: prompt,
            error: None,
            metadata: Some(serde_json::json!({
                "type": "command_generation_prompt",
                "user_request": user_request,
                "context": context,
                "os": os,
                "shell": shell
            })),
        })
    }

    pub fn list_directory(&self, path: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing directory: {}", "📂".cyan(), path.yellow());

        let entries = fs::read_dir(path)?;
        let mut items = Vec::new();

        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
            let size = if metadata.is_file() {
                format!(" ({} bytes)", metadata.len())
            } else {
                String::new()
            };

            items.push(format!(
                "{} {}{}",
                file_type,
                entry.file_name().to_string_lossy(),
                size
            ));
        }

        Ok(ToolResult {
            success: true,
            output: items.join("\n"),
            error: None,
            metadata: None,
        })
    }

    pub fn create_project(
        &self,
        name: &str,
        project_type: &str,
        path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let base_path = path.unwrap_or(".");
        let project_path = format!("{}/{}", base_path, name);

        println!(
            "{} Creating {} project: {}",
            "🚀".cyan(),
            project_type.yellow(),
            name.blue()
        );

        fs::create_dir_all(&project_path)?;

        let result_msg = match project_type.to_lowercase().as_str() {
            "rust" => self.create_rust_project(&project_path, name)?,
            "python" => self.create_python_project(&project_path, name)?,
            "javascript" | "js" => self.create_js_project(&project_path, name)?,
            _ => format!("Created basic project directory: {}", project_path),
        };

        Ok(ToolResult {
            success: true,
            output: format!(
                "Successfully created {} project: {} ({})",
                project_type, name, result_msg
            ),
            error: None,
            metadata: None,
        })
    }

    fn create_rust_project(
        &self,
        path: &str,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            name
        );

        fs::write(format!("{}/Cargo.toml", path), cargo_toml)?;
        fs::create_dir_all(format!("{}/src", path))?;
        fs::write(
            format!("{}/src/main.rs", path),
            "fn main() {\n    println!(\"Hello, world!\");\n}",
        )?;

        Ok("Created Rust project with Cargo.toml and src/main.rs".to_string())
    }

    fn create_python_project(
        &self,
        path: &str,
        _name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        fs::write(
            format!("{}/main.py", path),
            "#!/usr/bin/env python3\n\ndef main():\n    print(\"Hello, world!\")\n\nif __name__ == \"__main__\":\n    main()\n",
        )?;
        fs::write(
            format!("{}/requirements.txt", path),
            "# Add your dependencies here\n",
        )?;

        Ok("Created Python project with main.py and requirements.txt".to_string())
    }

    fn create_js_project(
        &self,
        path: &str,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let package_json = format!(
            r#"{{
  "name": "{}",
  "version": "1.0.0",
  "description": "",
  "main": "index.js",
  "scripts": {{
    "start": "node index.js"
  }},
  "dependencies": {{}}
}}
"#,
            name
        );

        fs::write(format!("{}/package.json", path), package_json)?;
        fs::write(
            format!("{}/index.js", path),
            "console.log('Hello, world!');\n",
        )?;

        Ok("Created JavaScript project with package.json and index.js".to_string())
    }

    // Helper method to check if a path should be ignored
    fn should_ignore_path(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();

        // Common patterns to ignore
        let ignore_patterns = [
            ".git/",
            "target/",
            "node_modules/",
            ".DS_Store",
            ".tmp",
            ".log",
            ".cache",
            ".lock",
            "__pycache__/",
            ".pytest_cache/",
        ];

        ignore_patterns
            .iter()
            .any(|pattern| path_str.contains(pattern))
    }

    // Synchronous fuzzy matching for filename search
    fn fuzzy_match_sync(&self, pattern: &str, text: &str) -> Option<f64> {
        if pattern.is_empty() {
            return Some(1.0);
        }

        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();

        let mut pattern_idx = 0;
        let mut consecutive_matches = 0;
        let mut score = 0.0;

        for (text_idx, &text_char) in text_chars.iter().enumerate() {
            if pattern_idx < pattern_chars.len() && text_char == pattern_chars[pattern_idx] {
                pattern_idx += 1;
                consecutive_matches += 1;

                // Bonus for consecutive matches
                score += 1.0 + (consecutive_matches as f64 * 0.1);

                // Bonus for matches at word boundaries
                if text_idx == 0
                    || text_chars[text_idx - 1] == '/'
                    || text_chars[text_idx - 1] == '_'
                    || text_chars[text_idx - 1] == '-'
                {
                    score += 0.5;
                }
            } else {
                consecutive_matches = 0;
            }
        }

        // Check if all pattern characters were matched
        if pattern_idx == pattern_chars.len() {
            // Calculate final score based on match quality
            let base_score = score / pattern_chars.len() as f64;

            // Bonus for shorter text (better matches)
            let length_bonus = 1.0 / (1.0 + text_chars.len() as f64 * 0.01);

            // Bonus for matches at the beginning
            let start_bonus = if pattern_idx > 0 && text_chars.get(0) == pattern_chars.get(0) {
                0.5
            } else {
                0.0
            };

            let final_score = base_score * length_bonus + start_bonus;
            Some(final_score)
        } else {
            None
        }
    }
}
