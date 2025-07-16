use colored::Colorize;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use super::core::{EditOperation, ToolExecutor, ToolResult};
use super::search::{enhanced_file_search, SearchQuery, ToolChain, ErrorStrategy};

impl ToolExecutor {
    // Web search implementation
    pub async fn web_search(&self, query: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Searching web for: {}", "🔍".cyan(), query.yellow());

        let search_url = format!(
            "https://duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .web_client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Assistant/1.0)")
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut results = Vec::new();

        if let Ok(selector) = Selector::parse(".result__title a") {
            for element in document.select(&selector).take(5) {
                if let Some(title) = element.text().collect::<Vec<_>>().first() {
                    if let Some(href) = element.value().attr("href") {
                        results.push(format!("{}: {}", title.trim(), href));
                    }
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: if results.is_empty() {
                format!(
                    "Search completed for '{}' but no clear results found",
                    query
                )
            } else {
                results.join("\n")
            },
            error: None,
            metadata: None,
        })
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
                    if let Some(score) = self.fuzzy_match_sync(&pattern_lower, &filename.to_lowercase()) {
                        found_files.push((path.to_path_buf(), score));
                    }
                }
                
                // Also check full path for better directory matching
                let full_path = path.to_string_lossy();
                if let Some(score) = self.fuzzy_match_sync(&pattern_lower, &full_path.to_lowercase()) {
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
                    Err(e) => {
                        match &chain.error_strategy {
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
                            ErrorStrategy::RetryWithBackoff { max_retries, backoff_ms } => {
                                if retries < *max_retries {
                                    retries += 1;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(*backoff_ms)).await;
                                    continue;
                                } else {
                                    return Err(e);
                                }
                            }
                        }
                    }
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
                let search_content = params.get("search_content")
                    .map(|s| s.parse().unwrap_or(true))
                    .unwrap_or(true);
                let is_regex = params.get("is_regex")
                    .map(|s| s.parse().unwrap_or(false))
                    .unwrap_or(false);
                let max_results = params.get("max_results")
                    .and_then(|s| s.parse().ok());
                    
                self.enhanced_file_search(
                    pattern,
                    directory.map(|s| s.as_str()),
                    search_content,
                    is_regex,
                    max_results,
                ).await
            }
            _ => {
                Err(format!("Unknown tool in chain: {}", step.tool_name).into())
            }
        }
    }

    pub fn file_read(&self, path: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Reading file: {}", "📖".cyan(), path.yellow());

        match fs::read_to_string(path) {
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

        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::write(path, content) {
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

    pub async fn execute_command(
        &self,
        command: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Executing command: {}", "⚡".cyan(), command.yellow());

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
            let msg = format!("Command completed with exit code: {}", status.code().unwrap_or(-1));
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
                format!("Command completed with exit code: {}", output.status.code().unwrap_or(-1))
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
        println!("{} Generating command for: {}", "🤖".cyan(), user_request.yellow());

        // Build the command generation prompt
        let mut prompt = String::new();
        
        // Add system-level context about the operating system
        let os = std::env::consts::OS;
        let shell = if cfg!(target_os = "windows") { "cmd" } else { "bash" };
        
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
        
        ignore_patterns.iter().any(|pattern| path_str.contains(pattern))
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
