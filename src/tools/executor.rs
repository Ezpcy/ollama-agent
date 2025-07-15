use colored::Colorize;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use super::core::{EditOperation, ToolExecutor, ToolResult};

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
        })
    }

    // File operations
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

        let regex = Regex::new(pattern)?;
        let mut found_files = Vec::new();

        for entry in WalkDir::new(search_dir).follow_links(true) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(filename) = entry.file_name().to_str() {
                    if regex.is_match(filename) {
                        found_files.push(entry.path().display().to_string());
                    }
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: if found_files.is_empty() {
                "No files found matching the pattern".to_string()
            } else {
                found_files.join("\n")
            },
            error: None,
        })
    }

    pub fn file_read(&self, path: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Reading file: {}", "📖".cyan(), path.yellow());

        match fs::read_to_string(path) {
            Ok(content) => Ok(ToolResult {
                success: true,
                output: content,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
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
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
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
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Could not write to file {}: {}", path, e)),
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
        })
    }

    pub async fn execute_command(
        &self,
        command: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Executing command: {}", "⚡".cyan(), command.yellow());

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()?
        } else {
            Command::new("sh").args(["-c", command]).output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(ToolResult {
            success: output.status.success(),
            output: stdout.to_string(),
            error: if stderr.is_empty() {
                None
            } else {
                Some(stderr.to_string())
            },
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
}
