use super::core::{AvailableTool, EditOperation};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ToolAnalysis {
    reasoning: String,
    tools: Vec<ToolRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolRequest {
    tool_type: String,
    parameters: serde_json::Value,
    reasoning: String,
}

pub struct NaturalLanguageParser {
    // Remove regex patterns - we'll use LLM instead
}

impl NaturalLanguageParser {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn parse_request_with_llm(
        &self,
        input: &str,
        llm_client: &crate::client::SelectedModel,
    ) -> Vec<AvailableTool> {
        let analysis_prompt = self.build_analysis_prompt(input);

        // Get LLM response
        match crate::client::stream_response(llm_client, &analysis_prompt).await {
            Ok(response) => {
                // Parse the LLM's JSON response
                if let Ok(analysis) = self.parse_llm_response(&response) {
                    return self.convert_to_tools(analysis);
                }
            }
            Err(e) => {
                println!("{} Error getting LLM analysis: {}", "⚠".yellow(), e);
            }
        }

        // Fallback to simple keyword detection if LLM fails
        self.simple_fallback_parse(input)
    }

    fn build_analysis_prompt(&self, user_input: &str) -> String {
        format!(
            r#"Analyze this user request and determine which tools are needed. 

User request: "{}"

Available tools:
1. WebSearch - search the internet for information
2. WebScrape - scrape content from a specific URL
3. FileRead - read a file from the filesystem
4. FileWrite - write content to a file
5. FileEdit - edit an existing file
6. FileSearch - search for files by name pattern
7. ContentSearch - search for text content within files
8. ListDirectory - list files in a directory
9. CreateProject - create a new project (Rust, Python, JS, etc.)
10. ExecuteCommand - run a system command

Respond ONLY with valid JSON in this exact format:
{{
  "reasoning": "Brief explanation of what the user wants",
  "tools": [
    {{
      "tool_type": "ToolName",
      "parameters": {{
        "param1": "value1",
        "param2": "value2"
      }},
      "reasoning": "Why this tool is needed"
    }}
  ]
}}

Examples:
- "read the cargo.toml file" → FileRead with path "Cargo.toml"
- "search for rust programming" → WebSearch with query "rust programming"
- "list files in src directory" → ListDirectory with path "src"
- "create a rust project called myapp" → CreateProject with name "myapp", project_type "rust"

Analyze the request and respond with JSON only:"#,
            user_input
        )
    }

    fn parse_llm_response(&self, response: &str) -> Result<ToolAnalysis, serde_json::Error> {
        // Extract JSON from the response (in case there's extra text)
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let json_str = &response[start..=end];
                return serde_json::from_str(json_str);
            }
        }

        // Try parsing the whole response as JSON
        serde_json::from_str(response)
    }

    fn convert_to_tools(&self, analysis: ToolAnalysis) -> Vec<AvailableTool> {
        let mut tools = Vec::new();

        println!(
            "{} LLM Analysis: {}",
            "🧠".cyan(),
            analysis.reasoning.blue()
        );

        for tool_req in analysis.tools {
            println!(
                "  {} {} - {}",
                "→".blue(),
                tool_req.tool_type.yellow(),
                tool_req.reasoning.dimmed()
            );

            match tool_req.tool_type.as_str() {
                "FileRead" => {
                    if let Some(path) = tool_req.parameters.get("path").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::FileRead {
                            path: path.to_string(),
                        });
                    }
                }
                "FileWrite" => {
                    if let Some(path) = tool_req.parameters.get("path").and_then(|v| v.as_str()) {
                        let content = tool_req
                            .parameters
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        tools.push(AvailableTool::FileWrite {
                            path: path.to_string(),
                            content,
                        });
                    }
                }
                "WebSearch" => {
                    if let Some(query) = tool_req.parameters.get("query").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::WebSearch {
                            query: query.to_string(),
                        });
                    }
                }
                "WebScrape" => {
                    if let Some(url) = tool_req.parameters.get("url").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::WebScrape {
                            url: url.to_string(),
                        });
                    }
                }
                "ListDirectory" => {
                    if let Some(path) = tool_req.parameters.get("path").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::ListDirectory {
                            path: path.to_string(),
                        });
                    }
                }
                "FileSearch" => {
                    if let Some(pattern) =
                        tool_req.parameters.get("pattern").and_then(|v| v.as_str())
                    {
                        let directory = tool_req
                            .parameters
                            .get("directory")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::FileSearch {
                            pattern: pattern.to_string(),
                            directory,
                        });
                    }
                }
                "ContentSearch" => {
                    if let Some(pattern) =
                        tool_req.parameters.get("pattern").and_then(|v| v.as_str())
                    {
                        let directory = tool_req
                            .parameters
                            .get("directory")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::ContentSearch {
                            pattern: pattern.to_string(),
                            directory,
                        });
                    }
                }
                "CreateProject" => {
                    if let Some(name) = tool_req.parameters.get("name").and_then(|v| v.as_str()) {
                        if let Some(project_type) = tool_req
                            .parameters
                            .get("project_type")
                            .and_then(|v| v.as_str())
                        {
                            let path = tool_req
                                .parameters
                                .get("path")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            tools.push(AvailableTool::CreateProject {
                                name: name.to_string(),
                                project_type: project_type.to_string(),
                                path,
                            });
                        }
                    }
                }
                "ExecuteCommand" => {
                    if let Some(command) =
                        tool_req.parameters.get("command").and_then(|v| v.as_str())
                    {
                        tools.push(AvailableTool::ExecuteCommand {
                            command: command.to_string(),
                        });
                    }
                }
                "FileEdit" => {
                    if let Some(path) = tool_req.parameters.get("path").and_then(|v| v.as_str()) {
                        let operation = EditOperation::Append {
                            content: "# File edit operation".to_string(),
                        };
                        tools.push(AvailableTool::FileEdit {
                            path: path.to_string(),
                            operation,
                        });
                    }
                }
                _ => {
                    println!(
                        "  {} Unknown tool type: {}",
                        "⚠".yellow(),
                        tool_req.tool_type
                    );
                }
            }
        }

        tools
    }

    fn simple_fallback_parse(&self, input: &str) -> Vec<AvailableTool> {
        let lower = input.to_lowercase();

        // Simple keyword-based fallback
        if lower.contains("read") && (lower.contains("cargo") || lower.contains("toml")) {
            return vec![AvailableTool::FileRead {
                path: "Cargo.toml".to_string(),
            }];
        }

        if lower.contains("list") && (lower.contains("directory") || lower.contains("files")) {
            return vec![AvailableTool::ListDirectory {
                path: ".".to_string(),
            }];
        }

        if lower.contains("search") && !lower.contains("file") {
            // Extract search query (simple approach)
            let query = input
                .replace("search for", "")
                .replace("search", "")
                .trim()
                .to_string();
            return vec![AvailableTool::WebSearch { query }];
        }

        vec![]
    }

    // Keep this for backward compatibility
    pub fn parse_request(&self, input: &str) -> Vec<AvailableTool> {
        self.simple_fallback_parse(input)
    }

    pub fn suggest_clarification(&self, input: &str) -> Option<String> {
        let lower_input = input.to_lowercase();

        if lower_input.contains("file") {
            Some("I can help with file operations. Try: 'read Cargo.toml', 'search for *.rs files', or 'create file.txt with content Hello'".to_string())
        } else if lower_input.contains("search") {
            Some("I can search the web or files. Try: 'search for rust programming' or 'search for \"function main\" in src/'".to_string())
        } else if lower_input.contains("project") {
            Some("I can create projects. Try: 'create a rust project called my-app' or 'make a python project named calculator'".to_string())
        } else {
            None
        }
    }
}
