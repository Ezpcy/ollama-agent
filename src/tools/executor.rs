use colored::Colorize;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use super::core::{EditOperation, ToolExecutor, ToolResult};
use super::search::{enhanced_file_search, ErrorStrategy, SearchQuery, ToolChain};
use super::web_search::{WebSearchEngine, format_search_results, get_fallback_resources};
use super::core::WebSearchConfig;
use super::enhanced_websearch::{EnhancedWebSearchEngine, EnhancedWebSearchConfig, format_enhanced_search_results};

impl ToolExecutor {
    // Enhanced web search implementation using the new intelligent system
    pub async fn web_search(&self, query: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = EnhancedWebSearchConfig::default();
        let search_engine = EnhancedWebSearchEngine::new(config);
        
        match search_engine.intelligent_search(query).await {
            Ok(results) => {
                let search_success = !results.is_empty();
                let final_output = format_enhanced_search_results(&results, query);
                
                // Count results with content
                let content_count = results.iter().filter(|r| r.content.is_some()).count();
                let avg_relevance = if !results.is_empty() {
                    results.iter().map(|r| r.relevance_score).sum::<f64>() / results.len() as f64
                } else {
                    0.0
                };

                Ok(ToolResult {
                    success: search_success,
                    output: final_output,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "query": query,
                        "query_intent": format!("{:?}", results.first().map(|r| &r.query_intent).unwrap_or(&super::enhanced_websearch::QueryIntent::General)),
                        "total_results": results.len(),
                        "results_with_content": content_count,
                        "search_engines_used": results.iter().map(|r| &r.source).collect::<std::collections::HashSet<_>>().into_iter().collect::<Vec<_>>(),
                        "average_relevance_score": avg_relevance,
                        "average_authority_score": if !results.is_empty() { results.iter().map(|r| r.authority_score).sum::<f64>() / results.len() as f64 } else { 0.0 },
                        "average_quality_score": if !results.is_empty() { results.iter().map(|r| r.quality_score).sum::<f64>() / results.len() as f64 } else { 0.0 }
                    })),
                    web_search_result: None,
                })
            }
            Err(e) => {
                let fallback_resources = get_fallback_resources(query);
                Ok(ToolResult {
                    success: false,
                    output: format!(
                        "Enhanced search failed for '{}'.\n\nHere are some relevant resources:\n{}\n\n{} Error details: {}",
                        query,
                        fallback_resources.iter().map(|item| format!("• {} - {}", item.title, item.url)).collect::<Vec<_>>().join("\n"),
                        "⚠️".yellow(),
                        e
                    ),
                    error: Some(e.to_string()),
                    metadata: Some(serde_json::json!({
                        "query": query,
                        "error": e.to_string(),
                        "fallback_resources_provided": fallback_resources.len(),
                        "search_type": "enhanced_intelligent"
                    })),
                    web_search_result: None,
                })
            }
        }
    }

    // Web scraping implementation using the new robust system
    pub async fn web_scrape(&self, url: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = WebSearchConfig::default();
        let search_engine = WebSearchEngine::new(config);
        
        match search_engine.extract_page_content(url).await {
            Ok(content) => {
                let content_length = content.len();
                let word_count = content.split_whitespace().count();
                
                Ok(ToolResult {
                    success: true,
                    output: format!(
                        "📄 Content extracted from: {}\n\nContent ({} characters, {} words):\n\n{}",
                        url, content_length, word_count, content
                    ),
                    error: None,
                    metadata: Some(serde_json::json!({
                        "url": url,
                        "content_length": content_length,
                        "word_count": word_count,
                        "extraction_successful": true
                    })),
                    web_search_result: None,
                })
            },
            Err(e) => Ok(ToolResult {
                success: false,
                output: format!("{} Failed to scrape content from {}\n\nError: {}", "✗".red(), url, e),
                error: Some(e.to_string()),
                metadata: Some(serde_json::json!({
                    "url": url,
                    "error": e.to_string(),
                    "extraction_successful": false
                })),
                web_search_result: None,
            })
        }
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
            web_search_result: None,
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
                                web_search_result: None,
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
                    web_search_result: None,
                });
            }
        };

        match fs::read_to_string(&validated_path) {
            Ok(content) => Ok(ToolResult {
                success: true,
                output: content,
                error: None,
                metadata: None,
                web_search_result: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                metadata: None,
                web_search_result: None,
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
                    web_search_result: None,
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
                web_search_result: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                metadata: None,
                web_search_result: None,
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
                    web_search_result: None,
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
                        web_search_result: None,
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
                        web_search_result: None,
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
                        web_search_result: None,
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
                    web_search_result: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Could not write to file {}: {}", path, e)),
                metadata: None,
                web_search_result: None,
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
            web_search_result: None,
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
        use std::path::Path;

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
                web_search_result: None,
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
            web_search_result: None,
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
            web_search_result: None,
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
            web_search_result: None,
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
            web_search_result: None,
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

    // Enhanced web search with specialized engines
    pub async fn enhanced_web_search(
        &self,
        query: &str,
        include_specialized: bool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = WebSearchConfig::default();
        let search_engine = WebSearchEngine::new(config);
        
        match search_engine.enhanced_search(query, include_specialized).await {
            Ok(search_result) => {
                let search_success = !search_result.results.is_empty();
                let final_output = format_search_results(&search_result);
                
                // Enhanced metadata
                let content_count = search_result.results.iter().filter(|r| r.content.is_some()).count();
                let avg_relevance = if !search_result.results.is_empty() {
                    search_result.results.iter().map(|r| r.relevance_score).sum::<f64>() / search_result.results.len() as f64
                } else {
                    0.0
                };

                Ok(ToolResult {
                    success: search_success,
                    output: final_output,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "query": search_result.query_used,
                        "total_results": search_result.results.len(),
                        "results_with_content": content_count,
                        "citations": search_result.citations.len(),
                        "searches_performed": search_result.search_metadata.total_searches_performed,
                        "processing_time_ms": search_result.search_metadata.processing_time_ms,
                        "average_relevance_score": avg_relevance,
                        "specialized_search_enabled": include_specialized
                    })),
                    web_search_result: Some(search_result),
                })
            }
            Err(e) => {
                let fallback_resources = get_fallback_resources(query);
                Ok(ToolResult {
                    success: false,
                    output: format!(
                        "Enhanced search failed for '{}'.\n\nHere are some relevant resources:\n{}\n\n{} Error details: {}",
                        query,
                        fallback_resources.iter().map(|item| format!("• {} - {}", item.title, item.url)).collect::<Vec<_>>().join("\n"),
                        "⚠️".yellow(),
                        e
                    ),
                    error: Some(e.to_string()),
                    metadata: Some(serde_json::json!({
                        "query": query,
                        "error": e.to_string(),
                        "fallback_resources_provided": fallback_resources.len()
                    })),
                    web_search_result: None,
                })
            }
        }
    }

    // Web performance testing
    pub async fn web_performance_test(
        &self,
        url: &str,
        test_count: usize,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Running performance test for: {}", "⚡".cyan(), url.yellow());

        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        for i in 0..test_count {
            let request_start = std::time::Instant::now();
            
            match self.web_client.get(url).send().await {
                Ok(response) => {
                    let response_time = request_start.elapsed();
                    let status = response.status().as_u16();
                    let content_length = response.content_length().unwrap_or(0);
                    
                    results.push((i + 1, true, status, response_time.as_millis() as u64, content_length));
                }
                Err(e) => {
                    let response_time = request_start.elapsed();
                    results.push((i + 1, false, 0, response_time.as_millis() as u64, 0));
                    println!("{} Request {} failed: {}", "❌".red(), i + 1, e);
                }
            }

            // Small delay between requests
            if i < test_count - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        let total_time = start_time.elapsed();
        let successful_requests = results.iter().filter(|(_, success, _, _, _)| *success).count();
        
        let avg_response_time = if successful_requests > 0 {
            results.iter()
                .filter(|(_, success, _, _, _)| *success)
                .map(|(_, _, _, time, _)| *time)
                .sum::<u64>() as f64 / successful_requests as f64
        } else {
            0.0
        };

        let min_response_time = results.iter()
            .filter(|(_, success, _, _, _)| *success)
            .map(|(_, _, _, time, _)| *time)
            .min()
            .unwrap_or(0);

        let max_response_time = results.iter()
            .filter(|(_, success, _, _, _)| *success)
            .map(|(_, _, _, time, _)| *time)
            .max()
            .unwrap_or(0);

        let output = format!(
            "⚡ Web Performance Test Results for: {}\n\
            ═══════════════════════════════════════════════════════\n\
            📊 Test Summary:\n\
            • Total Requests: {}\n\
            • Successful: {} ({:.1}%)\n\
            • Failed: {} ({:.1}%)\n\
            • Total Time: {:.2}s\n\
            • Requests/sec: {:.2}\n\n\
            ⏱️ Response Time Statistics:\n\
            • Average: {:.1}ms\n\
            • Minimum: {}ms\n\
            • Maximum: {}ms\n\n\
            📈 Performance Rating: {}",
            url,
            test_count,
            successful_requests,
            (successful_requests as f64 / test_count as f64) * 100.0,
            test_count - successful_requests,
            ((test_count - successful_requests) as f64 / test_count as f64) * 100.0,
            total_time.as_secs_f64(),
            test_count as f64 / total_time.as_secs_f64(),
            avg_response_time,
            min_response_time,
            max_response_time,
            if avg_response_time < 200.0 { "🟢 Excellent" }
            else if avg_response_time < 500.0 { "🟡 Good" }
            else if avg_response_time < 1000.0 { "🟠 Fair" }
            else { "🔴 Poor" }
        );

        Ok(ToolResult {
            success: successful_requests > 0,
            output,
            error: None,
            metadata: Some(serde_json::json!({
                "url": url,
                "test_count": test_count,
                "successful_requests": successful_requests,
                "failed_requests": test_count - successful_requests,
                "success_rate": (successful_requests as f64 / test_count as f64) * 100.0,
                "avg_response_time_ms": avg_response_time,
                "min_response_time_ms": min_response_time,
                "max_response_time_ms": max_response_time,
                "total_time_seconds": total_time.as_secs_f64(),
                "requests_per_second": test_count as f64 / total_time.as_secs_f64()
            })),
            web_search_result: None,
        })
    }
}
