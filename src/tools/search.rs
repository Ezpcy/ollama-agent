use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::sync::RwLock;
use walkdir::WalkDir;

use super::core::ToolResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub file_type: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub relevance_score: f64,
    pub matches: Vec<Match>,
    pub metadata: FileMetadata,
    pub fuzzy_score: Option<f64>,
    pub fuzzy_matches: Vec<usize>, // Character positions that matched
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_results: Option<usize>,
    pub search_content: bool,
    pub search_filenames: bool,
    pub fuzzy_matching: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            is_regex: false,
            case_sensitive: false,
            include_patterns: vec!["*".to_string()],
            exclude_patterns: vec![
                "*.git*".to_string(),
                "target/*".to_string(),
                "node_modules/*".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
            ],
            max_results: Some(100),
            search_content: true,
            search_filenames: true,
            fuzzy_matching: true,
        }
    }
}

pub struct SearchIndex {
    file_index: RwLock<HashMap<PathBuf, FileMetadata>>,
    ignore_patterns: Vec<glob::Pattern>,
    root_path: PathBuf,
}

impl SearchIndex {
    pub fn new(root_path: PathBuf) -> Self {
        let default_ignores = vec![
            ".git/**",
            "target/**",
            "node_modules/**",
            "*.tmp",
            "*.log",
            ".DS_Store",
            "*.swp",
            "*.swo",
        ];

        let ignore_patterns = default_ignores
            .iter()
            .filter_map(|pattern| glob::Pattern::new(pattern).ok())
            .collect();

        Self {
            file_index: RwLock::new(HashMap::new()),
            ignore_patterns,
            root_path,
        }
    }

    pub async fn build_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Building search index...", "🔍".cyan());
        
        let mut index = self.file_index.write().await;
        index.clear();

        for entry in WalkDir::new(&self.root_path).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                
                // Check if file should be ignored
                if self.should_ignore(path) {
                    continue;
                }

                if let Ok(metadata) = entry.metadata() {
                    let file_metadata = FileMetadata {
                        path: path.to_path_buf(),
                        size: metadata.len(),
                        modified: metadata.modified()?,
                        file_type: self.get_file_type(path),
                        content_hash: None, // TODO: Implement content hashing for change detection
                    };

                    index.insert(path.to_path_buf(), file_metadata);
                }
            }
        }

        println!("{} Indexed {} files", "✅".green(), index.len());
        Ok(())
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let index = self.file_index.read().await;
        let mut results = Vec::new();

        // Use fuzzy matching if enabled, otherwise use regex
        if query.fuzzy_matching {
            self.fuzzy_search(&index, query, &mut results);
        } else {
            let regex = if query.is_regex {
                Regex::new(&query.pattern)?
            } else {
                let escaped_pattern = if query.case_sensitive {
                    regex::escape(&query.pattern)
                } else {
                    format!("(?i){}", regex::escape(&query.pattern))
                };
                Regex::new(&escaped_pattern)?
            };

            self.regex_search(&index, query, &regex, &mut results)?;
        }

        // Sort by relevance score (descending)
        results.sort_by(|a, b| {
            let score_a = a.fuzzy_score.unwrap_or(a.relevance_score);
            let score_b = b.fuzzy_score.unwrap_or(b.relevance_score);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        if let Some(max_results) = query.max_results {
            results.truncate(max_results);
        }

        Ok(results)
    }

    pub async fn find_file_by_name(&self, name: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let index = self.file_index.read().await;
        let mut results = Vec::new();

        for path in index.keys() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename == name || filename.contains(name) {
                    results.push(path.clone());
                }
            }
        }

        Ok(results)
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.ignore_patterns.iter().any(|pattern| pattern.matches(&path_str))
    }

    fn matches_include_patterns(&self, path: &Path, patterns: &[String]) -> bool {
        if patterns.is_empty() || patterns.contains(&"*".to_string()) {
            return true;
        }

        patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches_path(path))
                .unwrap_or(false)
        })
    }

    fn matches_exclude_patterns(&self, path: &Path, patterns: &[String]) -> bool {
        patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches_path(path))
                .unwrap_or(false)
        })
    }

    fn get_file_type(&self, path: &Path) -> String {
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn is_text_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            matches!(ext, "rs" | "py" | "js" | "ts" | "html" | "css" | "json" | "toml" | "yaml" | "yml" | "md" | "txt" | "log")
        } else {
            false
        }
    }

    fn fuzzy_search(&self, index: &HashMap<PathBuf, FileMetadata>, query: &SearchQuery, results: &mut Vec<SearchResult>) {
        let pattern = if query.case_sensitive {
            query.pattern.clone()
        } else {
            query.pattern.to_lowercase()
        };

        for (path, metadata) in index.iter() {
            // Check include/exclude patterns
            if !self.matches_include_patterns(path, &query.include_patterns) {
                continue;
            }
            if self.matches_exclude_patterns(path, &query.exclude_patterns) {
                continue;
            }

            let mut matches = Vec::new();
            let mut relevance_score = 0.0;
            let mut fuzzy_score = None;
            let mut fuzzy_matches = Vec::new();

            // Fuzzy search in filename
            if query.search_filenames {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let search_text = if query.case_sensitive {
                        filename.to_string()
                    } else {
                        filename.to_lowercase()
                    };

                    if let Some((score, match_positions)) = self.fuzzy_match(&pattern, &search_text) {
                        relevance_score += score * 10.0; // Filename matches are highly relevant
                        fuzzy_score = Some(score);
                        fuzzy_matches = match_positions;
                        matches.push(Match {
                            line_number: 0,
                            line_content: filename.to_string(),
                            match_start: 0,
                            match_end: filename.len(),
                        });
                    }
                }
            }

            // Fuzzy search in full path for better subdirectory matching
            if query.search_filenames {
                let full_path = path.to_string_lossy();
                let search_text = if query.case_sensitive {
                    full_path.to_string()
                } else {
                    full_path.to_lowercase()
                };

                if let Some((score, match_positions)) = self.fuzzy_match(&pattern, &search_text) {
                    // Only update if this is a better match than filename alone
                    if fuzzy_score.map_or(true, |existing| score > existing) {
                        relevance_score = score * 8.0; // Path matches are also highly relevant
                        fuzzy_score = Some(score);
                        fuzzy_matches = match_positions;
                        
                        // Add or update the match
                        if matches.is_empty() {
                            matches.push(Match {
                                line_number: 0,
                                line_content: full_path.to_string(),
                                match_start: 0,
                                match_end: full_path.len(),
                            });
                        }
                    }
                }
            }

            // Fuzzy search in file content
            if query.search_content && self.is_text_file(path) {
                if let Ok(content) = fs::read_to_string(path) {
                    for (line_number, line) in content.lines().enumerate() {
                        let search_text = if query.case_sensitive {
                            line.to_string()
                        } else {
                            line.to_lowercase()
                        };

                        if let Some((score, _)) = self.fuzzy_match(&pattern, &search_text) {
                            relevance_score += score;
                            matches.push(Match {
                                line_number: line_number + 1,
                                line_content: line.to_string(),
                                match_start: 0,
                                match_end: line.len(),
                            });
                        }
                    }
                }
            }

            if !matches.is_empty() {
                results.push(SearchResult {
                    path: path.clone(),
                    relevance_score,
                    matches,
                    metadata: metadata.clone(),
                    fuzzy_score,
                    fuzzy_matches,
                });
            }
        }
    }

    fn regex_search(&self, index: &HashMap<PathBuf, FileMetadata>, query: &SearchQuery, regex: &Regex, results: &mut Vec<SearchResult>) -> Result<(), Box<dyn std::error::Error>> {
        for (path, metadata) in index.iter() {
            // Check include/exclude patterns
            if !self.matches_include_patterns(path, &query.include_patterns) {
                continue;
            }
            if self.matches_exclude_patterns(path, &query.exclude_patterns) {
                continue;
            }

            let mut matches = Vec::new();
            let mut relevance_score = 0.0;

            // Search in filename
            if query.search_filenames {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if regex.is_match(filename) {
                        relevance_score += 10.0; // Filename matches are highly relevant
                        matches.push(Match {
                            line_number: 0,
                            line_content: filename.to_string(),
                            match_start: 0,
                            match_end: filename.len(),
                        });
                    }
                }
            }

            // Search in file content
            if query.search_content && self.is_text_file(path) {
                if let Ok(content) = fs::read_to_string(path) {
                    for (line_number, line) in content.lines().enumerate() {
                        if let Some(mat) = regex.find(line) {
                            relevance_score += 1.0;
                            matches.push(Match {
                                line_number: line_number + 1,
                                line_content: line.to_string(),
                                match_start: mat.start(),
                                match_end: mat.end(),
                            });
                        }
                    }
                }
            }

            if !matches.is_empty() {
                results.push(SearchResult {
                    path: path.clone(),
                    relevance_score,
                    matches,
                    metadata: metadata.clone(),
                    fuzzy_score: None,
                    fuzzy_matches: Vec::new(),
                });
            }
        }
        Ok(())
    }

    // FZF-like fuzzy matching algorithm
    fn fuzzy_match(&self, pattern: &str, text: &str) -> Option<(f64, Vec<usize>)> {
        if pattern.is_empty() {
            return Some((1.0, Vec::new()));
        }

        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        
        let mut pattern_idx = 0;
        let mut match_positions = Vec::new();
        let mut consecutive_matches = 0;
        let mut score = 0.0;
        
        for (text_idx, &text_char) in text_chars.iter().enumerate() {
            if pattern_idx < pattern_chars.len() && text_char == pattern_chars[pattern_idx] {
                match_positions.push(text_idx);
                pattern_idx += 1;
                consecutive_matches += 1;
                
                // Bonus for consecutive matches
                score += 1.0 + (consecutive_matches as f64 * 0.1);
                
                // Bonus for matches at word boundaries
                if text_idx == 0 || text_chars[text_idx - 1] == '/' || text_chars[text_idx - 1] == '_' || text_chars[text_idx - 1] == '-' {
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
            
            // Bonus for matches at the beginning of filename
            let start_bonus = if !match_positions.is_empty() && match_positions[0] == 0 {
                0.5
            } else {
                0.0
            };
            
            let final_score = base_score * length_bonus + start_bonus;
            Some((final_score, match_positions))
        } else {
            None
        }
    }
}

pub struct ToolChain {
    pub steps: Vec<ChainStep>,
    pub error_strategy: ErrorStrategy,
}

#[derive(Debug, Clone)]
pub struct ChainStep {
    pub tool_name: String,
    pub parameters: HashMap<String, String>,
    pub depends_on: Option<usize>, // Index of the step this depends on
    pub use_previous_result: bool,
}

#[derive(Debug, Clone)]
pub enum ErrorStrategy {
    FailFast,
    ContinueOnError,
    RetryWithBackoff { max_retries: u32, backoff_ms: u64 },
}

impl ToolChain {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            error_strategy: ErrorStrategy::FailFast,
        }
    }

    pub fn add_step(&mut self, step: ChainStep) {
        self.steps.push(step);
    }

    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    // Example: Search for file A, then read its content to find path for file B, then read file B
    pub fn create_file_chain_search(file_a_pattern: &str, path_extraction_pattern: &str) -> Self {
        let mut chain = Self::new();
        
        // Step 1: Search for file A
        chain.add_step(ChainStep {
            tool_name: "file_search".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("pattern".to_string(), file_a_pattern.to_string());
                params
            },
            depends_on: None,
            use_previous_result: false,
        });

        // Step 2: Read file A to extract path for file B
        chain.add_step(ChainStep {
            tool_name: "file_read".to_string(),
            parameters: HashMap::new(), // Will be populated from previous result
            depends_on: Some(0),
            use_previous_result: true,
        });

        // Step 3: Extract path and read file B (would need custom logic)
        chain.add_step(ChainStep {
            tool_name: "extract_and_read".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("pattern".to_string(), path_extraction_pattern.to_string());
                params
            },
            depends_on: Some(1),
            use_previous_result: true,
        });

        chain
    }
}

// Enhanced search functionality for the executor
pub async fn enhanced_file_search(
    root_path: &Path,
    query: &SearchQuery,
) -> Result<ToolResult, Box<dyn std::error::Error>> {
    let index = SearchIndex::new(root_path.to_path_buf());
    index.build_index().await?;
    
    let results = index.search(query).await?;
    
    if results.is_empty() {
        return Ok(ToolResult {
            success: true,
            output: format!("No files found matching pattern: {}", query.pattern),
            error: None,
            metadata: None,
        });
    }

    let mut output = Vec::new();
    output.push(format!("Found {} results:", results.len()));
    
    for result in results.iter().take(query.max_results.unwrap_or(50)) {
        output.push(format!(
            "\n{} {} (score: {:.1})",
            "📄".cyan(),
            result.path.display().to_string().yellow(),
            result.relevance_score
        ));
        
        // Show first few matches
        for mat in result.matches.iter().take(3) {
            if mat.line_number > 0 {
                output.push(format!(
                    "  {}:{} {}",
                    mat.line_number.to_string().blue(),
                    " ".repeat(6 - mat.line_number.to_string().len()),
                    mat.line_content.trim()
                ));
            }
        }
    }

    Ok(ToolResult {
        success: true,
        output: output.join("\n"),
        error: None,
        metadata: Some(serde_json::to_value(&results)?),
    })
}