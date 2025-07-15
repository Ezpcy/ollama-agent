use super::core::{
    ApiAuth, AvailableTool, CargoOperation, DatabaseType, DockerResourceType, EditOperation,
    ExportFormat, GitBranchOperation, HttpMethod, ModelParameter, NpmOperation, PipOperation,
    RestOperation, TextOperation,
};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    // Enhanced with model awareness
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
        // First check for immediate commands that don't need LLM parsing
        if let Some(tool) = self.parse_immediate_commands(input) {
            return vec![tool];
        }

        let analysis_prompt = self.build_enhanced_analysis_prompt(input);

        // Get LLM response
        match crate::client::stream_response(llm_client, &analysis_prompt).await {
            Ok(response) => {
                if let Ok(analysis) = self.parse_llm_response(&response) {
                    return self.convert_to_tools(analysis);
                }
            }
            Err(e) => {
                println!("{} Error getting LLM analysis: {}", "⚠".yellow(), e);
            }
        }

        // Enhanced fallback with more sophisticated parsing
        self.enhanced_fallback_parse(input)
    }

    fn parse_immediate_commands(&self, input: &str) -> Option<AvailableTool> {
        let lower_owned = input.to_lowercase();
        let lower = lower_owned.trim();

        // Model parameter commands
        if lower.starts_with("set temperature") {
            if let Some(temp_str) = lower.strip_prefix("set temperature") {
                if let Ok(temp) = temp_str.trim().parse::<f64>() {
                    return Some(AvailableTool::SetModelParameter {
                        parameter: ModelParameter::Temperature,
                        value: serde_json::Value::Number(
                            serde_json::Number::from_f64(temp).unwrap(),
                        ),
                    });
                }
            }
        }

        if lower.starts_with("set max tokens") {
            if let Some(tokens_str) = lower.strip_prefix("set max tokens") {
                if let Ok(tokens) = tokens_str.trim().parse::<u64>() {
                    return Some(AvailableTool::SetModelParameter {
                        parameter: ModelParameter::MaxTokens,
                        value: serde_json::Value::Number(serde_json::Number::from(tokens)),
                    });
                }
            }
        }

        if lower == "show model config" || lower == "get model parameters" {
            return Some(AvailableTool::GetModelParameter { parameter: None });
        }

        if lower.starts_with("switch to model") {
            if let Some(model_name) = lower.strip_prefix("switch to model") {
                return Some(AvailableTool::SwitchModel {
                    model_name: model_name.trim().to_string(),
                });
            }
        }

        // Git shortcuts
        if lower == "git status" {
            return Some(AvailableTool::GitStatus {
                repository_path: None,
            });
        }

        if lower == "git log" {
            return Some(AvailableTool::GitLog {
                count: Some(10),
                oneline: true,
                repository_path: None,
            });
        }

        // System shortcuts
        if lower == "system info" {
            return Some(AvailableTool::SystemInfo);
        }

        if lower == "memory usage" {
            return Some(AvailableTool::MemoryUsage);
        }

        if lower == "disk usage" {
            return Some(AvailableTool::DiskUsage { path: None });
        }

        // Clear conversation
        if lower == "clear history" || lower == "clear conversation" {
            return Some(AvailableTool::ClearHistory);
        }

        None
    }

    fn build_enhanced_analysis_prompt(&self, user_input: &str) -> String {
        format!(
            r#"Analyze this user request and determine which tools are needed. 

User request: "{}"

Available tools (with examples):

## File Operations
- FileRead: read a file ("read Cargo.toml", "show me the main.rs file")
- FileWrite: write content to a file ("write hello world to test.txt", "create a readme file")
- FileEdit: edit an existing file ("add a line to main.rs", "replace function name in utils.rs")
- FileSearch: search for files by name pattern ("find all .rs files", "search for config files")
- ContentSearch: search for text within files ("find TODO comments", "search for function main")
- ListDirectory: list files in a directory ("list files in src", "show current directory")
- FileWatch: watch a file for changes ("watch config.json for changes")

## Git Operations
- GitStatus: check git status
- GitAdd: add files to git ("git add main.rs", "stage all changes")
- GitCommit: create a commit ("commit with message 'fix bug'")
- GitPush: push to remote ("push to origin", "push changes")
- GitPull: pull from remote ("pull latest changes")
- GitBranch: branch operations ("create branch feature-x", "switch to main", "list branches")
- GitLog: show commit history ("show git log", "last 5 commits")
- GitDiff: show changes ("show diff", "diff main.rs")

## Web & API Operations
- WebSearch: search the internet ("search for rust tutorials")
- WebScrape: scrape content from URL ("scrape https://example.com")
- HttpRequest: make HTTP requests ("GET request to api.example.com")
- RestApiCall: REST API operations ("get users from api", "create user via api")
- GraphQLQuery: GraphQL queries ("query users with GraphQL")

## Package Management
- CargoOperation: Rust package operations ("cargo build", "cargo test", "add serde dependency")
- NpmOperation: Node.js package operations ("npm install", "run dev script")
- PipOperation: Python package operations ("pip install requests", "list packages")

## System Operations
- ProcessList: list running processes ("show running processes")
- SystemInfo: get system information
- DiskUsage: check disk usage ("check disk space")
- MemoryUsage: check memory usage
- NetworkInfo: get network information
- ExecuteCommand: run system commands ("run ls -la", "execute python script")

## Docker Operations
- DockerList: list docker resources ("list containers", "show docker images")
- DockerRun: run a container ("run nginx container", "start postgres with port 5432")
- DockerStop: stop a container ("stop container myapp")
- DockerLogs: view container logs ("show logs for webapp")

## Text Processing
- JsonFormat: format JSON ("format this json", "pretty print json")
- JsonQuery: query JSON data ("get user.name from json", "extract emails")
- CsvParse: parse CSV data ("parse this csv", "convert csv to table")
- RegexMatch: match with regex ("find emails in text", "match phone numbers")
- TextTransform: transform text ("convert to uppercase", "trim whitespace")

## Model Configuration
- SetModelParameter: change model settings ("set temperature to 0.8", "increase max tokens")
- GetModelParameter: view model settings ("show temperature", "get model config")
- SwitchModel: change the current model ("switch to llama2", "use codellama")

## Project Management
- CreateProject: create new projects ("create rust project myapp", "make python project")
- ScheduleTask: schedule recurring tasks ("schedule daily backup")
- ListScheduledTasks: view scheduled tasks
- CancelScheduledTask: cancel a scheduled task

## Session Management
- SetConfig: set configuration ("set auto-approve to true")
- GetConfig: get configuration ("show config")
- ExportConversation: export chat history ("export conversation to markdown")
- ImportConversation: import chat history
- ClearHistory: clear conversation history

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

Consider:
1. The user's intent and what they're trying to accomplish
2. Multiple tools may be needed for complex requests
3. Use sensible defaults for optional parameters
4. Suggest the most specific tool for the task

Analyze the request and respond with JSON only:"#,
            user_input
        )
    }

    fn parse_llm_response(&self, response: &str) -> Result<ToolAnalysis, serde_json::Error> {
        // Extract JSON from the response
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let json_str = &response[start..=end];
                return serde_json::from_str(json_str);
            }
        }

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
                // Existing tools
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

                // Git operations
                "GitStatus" => {
                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GitStatus { repository_path });
                }
                "GitAdd" => {
                    let files = tool_req
                        .parameters
                        .get("files")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_else(|| vec![".".to_string()]);

                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    tools.push(AvailableTool::GitAdd {
                        files,
                        repository_path,
                    });
                }
                "GitCommit" => {
                    if let Some(message) =
                        tool_req.parameters.get("message").and_then(|v| v.as_str())
                    {
                        let repository_path = tool_req
                            .parameters
                            .get("repository_path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        tools.push(AvailableTool::GitCommit {
                            message: message.to_string(),
                            repository_path,
                        });
                    }
                }

                // HTTP/API operations
                "HttpRequest" => {
                    if let Some(url) = tool_req.parameters.get("url").and_then(|v| v.as_str()) {
                        let method = tool_req
                            .parameters
                            .get("method")
                            .and_then(|v| v.as_str())
                            .and_then(|m| match m.to_uppercase().as_str() {
                                "GET" => Some(HttpMethod::GET),
                                "POST" => Some(HttpMethod::POST),
                                "PUT" => Some(HttpMethod::PUT),
                                "DELETE" => Some(HttpMethod::DELETE),
                                "PATCH" => Some(HttpMethod::PATCH),
                                _ => None,
                            })
                            .unwrap_or(HttpMethod::GET);

                        let headers = tool_req
                            .parameters
                            .get("headers")
                            .and_then(|v| v.as_object())
                            .map(|obj| {
                                obj.iter()
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|s| (k.clone(), s.to_string()))
                                    })
                                    .collect()
                            });

                        let body = tool_req
                            .parameters
                            .get("body")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        tools.push(AvailableTool::HttpRequest {
                            method,
                            url: url.to_string(),
                            headers,
                            body,
                            timeout_seconds: None,
                        });
                    }
                }

                // Model configuration
                "SetModelParameter" => {
                    if let Some(param_str) = tool_req
                        .parameters
                        .get("parameter")
                        .and_then(|v| v.as_str())
                    {
                        let parameter = match param_str.to_lowercase().as_str() {
                            "temperature" => Some(ModelParameter::Temperature),
                            "max_tokens" | "maxtokens" => Some(ModelParameter::MaxTokens),
                            "top_p" | "topp" => Some(ModelParameter::TopP),
                            "top_k" | "topk" => Some(ModelParameter::TopK),
                            "repeat_penalty" | "repeatpenalty" => {
                                Some(ModelParameter::RepeatPenalty)
                            }
                            "system_prompt" | "systemprompt" => Some(ModelParameter::SystemPrompt),
                            "context_length" | "contextlength" => {
                                Some(ModelParameter::ContextLength)
                            }
                            _ => None,
                        };

                        if let (Some(parameter), Some(value)) =
                            (parameter, tool_req.parameters.get("value"))
                        {
                            tools.push(AvailableTool::SetModelParameter {
                                parameter,
                                value: value.clone(),
                            });
                        }
                    }
                }
                "GetModelParameter" => {
                    let parameter = tool_req
                        .parameters
                        .get("parameter")
                        .and_then(|v| v.as_str())
                        .and_then(|param_str| match param_str.to_lowercase().as_str() {
                            "temperature" => Some(ModelParameter::Temperature),
                            "max_tokens" => Some(ModelParameter::MaxTokens),
                            "top_p" => Some(ModelParameter::TopP),
                            "top_k" => Some(ModelParameter::TopK),
                            "repeat_penalty" => Some(ModelParameter::RepeatPenalty),
                            "system_prompt" => Some(ModelParameter::SystemPrompt),
                            "context_length" => Some(ModelParameter::ContextLength),
                            _ => None,
                        });

                    tools.push(AvailableTool::GetModelParameter { parameter });
                }
                "SwitchModel" => {
                    if let Some(model_name) = tool_req
                        .parameters
                        .get("model_name")
                        .and_then(|v| v.as_str())
                    {
                        tools.push(AvailableTool::SwitchModel {
                            model_name: model_name.to_string(),
                        });
                    }
                }

                // System operations
                "SystemInfo" => {
                    tools.push(AvailableTool::SystemInfo);
                }
                "MemoryUsage" => {
                    tools.push(AvailableTool::MemoryUsage);
                }
                "DiskUsage" => {
                    let path = tool_req
                        .parameters
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::DiskUsage { path });
                }
                "ProcessList" => {
                    let filter = tool_req
                        .parameters
                        .get("filter")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::ProcessList { filter });
                }

                // Add more tool conversions here...
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

    fn enhanced_fallback_parse(&self, input: &str) -> Vec<AvailableTool> {
        let lower = input.to_lowercase();
        let mut tools = Vec::new();

        // Enhanced keyword matching with better context understanding

        // Git operations
        if lower.contains("git") {
            if lower.contains("status") {
                tools.push(AvailableTool::GitStatus {
                    repository_path: None,
                });
            } else if lower.contains("add") {
                tools.push(AvailableTool::GitAdd {
                    files: vec![".".to_string()],
                    repository_path: None,
                });
            } else if lower.contains("commit") {
                let message = if let Some(start) = lower.find("commit") {
                    input[start..]
                        .trim_start_matches("commit")
                        .trim()
                        .to_string()
                } else {
                    "Automated commit".to_string()
                };
                tools.push(AvailableTool::GitCommit {
                    message,
                    repository_path: None,
                });
            } else if lower.contains("push") {
                tools.push(AvailableTool::GitPush {
                    remote: None,
                    branch: None,
                    repository_path: None,
                });
            } else if lower.contains("pull") {
                tools.push(AvailableTool::GitPull {
                    remote: None,
                    branch: None,
                    repository_path: None,
                });
            }
        }

        // Model configuration fallback
        if lower.contains("temperature") && (lower.contains("set") || lower.contains("change")) {
            // Try to extract temperature value
            if let Some(temp_match) = regex::Regex::new(r"(\d+\.?\d*)")
                .ok()
                .and_then(|re| re.find(&lower))
            {
                if let Ok(temp) = temp_match.as_str().parse::<f64>() {
                    tools.push(AvailableTool::SetModelParameter {
                        parameter: ModelParameter::Temperature,
                        value: serde_json::Value::Number(
                            serde_json::Number::from_f64(temp).unwrap(),
                        ),
                    });
                }
            }
        }

        // System info requests
        if lower.contains("system") && (lower.contains("info") || lower.contains("information")) {
            tools.push(AvailableTool::SystemInfo);
        }

        if lower.contains("memory") || lower.contains("ram") {
            tools.push(AvailableTool::MemoryUsage);
        }

        if lower.contains("disk") && (lower.contains("space") || lower.contains("usage")) {
            tools.push(AvailableTool::DiskUsage { path: None });
        }

        // Process management
        if lower.contains("process") && lower.contains("list") {
            tools.push(AvailableTool::ProcessList { filter: None });
        }

        // If no tools found, try original fallback
        if tools.is_empty() {
            tools = self.simple_fallback_parse(input);
        }

        tools
    }

    fn simple_fallback_parse(&self, input: &str) -> Vec<AvailableTool> {
        let lower = input.to_lowercase();

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
        self.enhanced_fallback_parse(input)
    }

    pub fn suggest_clarification(&self, input: &str) -> Option<String> {
        let lower_input = input.to_lowercase();

        if lower_input.contains("model")
            || lower_input.contains("temperature")
            || lower_input.contains("parameter")
        {
            Some("I can help with model configuration. Try: 'set temperature to 0.8', 'show model config', or 'switch to llama2'".to_string())
        } else if lower_input.contains("git") {
            Some("I can help with git operations. Try: 'git status', 'git add main.rs', 'commit with message', or 'push changes'".to_string())
        } else if lower_input.contains("api") || lower_input.contains("http") {
            Some("I can help with API calls. Try: 'GET request to api.example.com', 'scrape https://example.com', or 'query GraphQL endpoint'".to_string())
        } else if lower_input.contains("docker") {
            Some("I can help with Docker. Try: 'list containers', 'run nginx container', or 'show docker logs for myapp'".to_string())
        } else if lower_input.contains("system") {
            Some("I can help with system operations. Try: 'system info', 'memory usage', 'disk space', or 'list processes'".to_string())
        } else if lower_input.contains("file") {
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
