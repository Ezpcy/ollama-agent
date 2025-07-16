use super::core::{
    ApiAuth, AvailableTool, CargoOperation, DatabaseType, DockerResourceType, EditOperation,
    ExportFormat, GitBranchOperation, HttpMethod, ModelParameter, NpmOperation, PipOperation,
    RestOperation, TextOperation,
};
use colored::Colorize;
use serde::{Deserialize, Serialize};
// use std::collections::HashMap;

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

    fn parse_immediate_commands(&self, _input: &str) -> Option<AvailableTool> {
        // Remove all hardcoded patterns - let LLM handle everything
        None
    }

    fn build_enhanced_analysis_prompt(&self, user_input: &str) -> String {
        format!(
            r#"You are an intelligent command parser that analyzes user requests and maps them to the appropriate tools.

USER REQUEST: "{}"

AVAILABLE TOOLS AND THEIR USAGE PATTERNS:

## File Operations
- FileRead: Read file content
  Examples: "read Cargo.toml", "show main.rs", "what's in the config file", "display package.json"
  Parameters: path (string, exact filename)

- FileWrite: Write content to file
  Examples: "write hello to test.txt", "create readme with content", "save data to file.json"
  Parameters: path (string), content (string)

- FileEdit: Edit existing file
  Examples: "edit main.rs", "modify config", "update the dockerfile"
  Parameters: path (string), operation (object)

- FileSearch: Find files by pattern
  Examples: "find *.rs files", "search for config files", "locate all json files"
  Parameters: pattern (string), directory (optional string)

- ContentSearch: Search text within files
  Examples: "find TODO in code", "search for main function", "look for error messages"
  Parameters: pattern (string), directory (optional string)

- ListDirectory: List directory contents
  Examples: "list files", "show directory", "what's in src/", "ls"
  Parameters: path (string)

- FileWatch: Monitor file changes
  Examples: "watch config.json", "monitor Cargo.toml for 60 seconds", "watch the main.rs file for changes", "observe package.json for 2 minutes"
  Parameters: path (string, exact filename), duration_seconds (optional number, convert minutes to seconds)
  IMPORTANT: For "watch the X" format, extract X as the path, not "the"

- CreateProject: Generate a new project
  Examples: "create python project myapp", "make javascript project"
  Parameters: name (string), project_type (string), path (optional string)

## Git Operations
- GitStatus: Check repository status
  Examples: "git status", "check git", "repo status", "show changes"
  Parameters: repository_path (optional string - OMIT unless user specifies a specific directory)

- GitAdd: Stage files
  Examples: "git add main.rs", "stage changes", "add all files", "stage everything"
  Parameters: files (array of strings), repository_path (optional string - OMIT unless user specifies a specific directory)

- GitCommit: Create commit
  Examples: "commit changes", "git commit with message fix bug", "commit 'added feature'"
  Parameters: message (string), repository_path (optional string - OMIT unless user specifies a specific directory)

- GitPush: Push to remote
  Examples: "push changes", "git push", "push to origin", "push to main branch"
  Parameters: remote (optional string), branch (optional string), repository_path (optional string - OMIT unless user specifies a specific directory)

- GitPull: Pull from remote
  Examples: "pull changes", "git pull", "pull from origin", "update from remote"
  Parameters: remote (optional string), branch (optional string), repository_path (optional string - OMIT unless user specifies a specific directory)

- GitLog: Show commit history
  Examples: "git log", "show commits", "last 5 commits", "commit history"
  Parameters: count (optional number), oneline (boolean), repository_path (optional string - OMIT unless user specifies a specific directory)

## System Operations
- SystemInfo: Get system information
  Examples: "system info", "system details", "show system", "hardware info"
  Parameters: none

- MemoryUsage: Check memory usage
  Examples: "memory usage", "check memory", "show ram", "memory info"
  Parameters: none

- DiskUsage: Check disk space
  Examples: "disk usage", "check disk space", "storage info", "disk space in /home"
  Parameters: path (optional string)

- ProcessList: List running processes
  Examples: "list processes", "show processes", "running apps", "ps aux"
  Parameters: filter (optional string)

- ExecuteCommand: Run system commands
  Examples: "run ls -la", "execute python script.py", "command mkdir test"
  Parameters: command (string)

## Model Configuration
- SetModelParameter: Change model settings
  Examples: "set temperature to 0.8", "change max tokens to 2048", "set top-p to 0.9"
  Parameters: parameter (enum), value (varies by parameter)

- GetModelParameter: View model settings
  Examples: "show model config", "get temperature", "display settings", "model parameters"
  Parameters: parameter (optional enum)

- SwitchModel: Change active model
  Examples: "switch to llama2", "use codellama", "change model to gemma", "switch model"
  Parameters: model_name (string)

## Package Management
- CargoOperation: Rust operations
  Examples: "cargo build", "cargo test", "add serde", "build project"
  Parameters: operation (enum), package (optional string), features (optional array)

- NpmOperation: Node.js operations
  Examples: "npm install", "npm run dev", "install express", "run tests"
  Parameters: operation (enum), package (optional string), dev (boolean)

- PipOperation: Python operations
  Examples: "pip install requests", "pip list", "pip uninstall package"
  Parameters: operation (enum), package (optional string), requirements_file (optional string)

## Docker Operations
- DockerList: List Docker resources
  Examples: "list containers", "show docker images"
  Parameters: resource_type (enum)

- DockerRun: Run a container
  Examples: "run nginx", "start postgres with ports"
  Parameters: image (string), command (optional string), ports (optional array), volumes (optional array), environment (optional object)

- DockerStop: Stop a container
  Examples: "stop container web", "docker stop myapp"
  Parameters: container (string)

- DockerLogs: Show container logs
  Examples: "docker logs web", "show logs for api -f"
  Parameters: container (string), follow (boolean), tail (optional number)

## Database Operations
- SqlQuery: Query databases
  Examples: "run SQL select", "query postgres db"
  Parameters: connection_string (string), query (string), database_type (enum)

- SqliteQuery: Query SQLite file
  Examples: "query data.db", "sqlite select"
  Parameters: database_path (string), query (string)

## Web & API
- WebSearch: Search internet
  Examples: "search rust tutorials", "google python guides", "find documentation"
  Parameters: query (string)

- WebScrape: Extract web content
  Examples: "scrape https://example.com", "get content from url", "extract webpage"
  Parameters: url (string)

- HttpRequest: Make HTTP requests
  Examples: "GET api.example.com", "POST to webhook", "HTTP request to server"
  Parameters: method (enum), url (string), headers (optional object), body (optional string)

- RestApiCall: High level REST API operation
  Examples: "create user via /api", "delete /api/item/1"
  Parameters: endpoint (string), operation (enum), data (optional object), auth (optional object)

- GraphQLQuery: Execute GraphQL operations
  Examples: "query GraphQL endpoint", "graphql mutation"
  Parameters: endpoint (string), query (string), variables (optional object), auth (optional object)

## Text Processing
- JsonFormat: Format JSON
  Examples: "format json", "pretty print json", "beautify json data"
  Parameters: input (string)

- JsonQuery: Query JSON data
  Examples: "get .users[0] from json", "json query"
  Parameters: input (string), query (string)

- CsvParse: Parse CSV text
  Examples: "parse csv", "csv data with ; delimiter"
  Parameters: input (string), delimiter (optional char)

- TextTransform: Text manipulation
  Examples: "uppercase this text", "replace foo with bar"
  Parameters: input (string), operation (enum)

- RegexMatch: Pattern matching
  Examples: "find emails in text", "match phone numbers", "extract urls"
  Parameters: pattern (string), text (string), flags (optional string)

## Session Management
- ClearHistory: Clear conversation
  Examples: "clear history", "clear conversation", "reset chat", "new session"
  Parameters: none

- SetConfig: Update configuration value
  Examples: "set api_key", "configure timeout"
  Parameters: key (string), value (any)

- GetConfig: View configuration
  Examples: "get config", "show setting api_key"
  Parameters: key (optional string)

- ExportConversation: Save chat history
  Examples: "export chat to file", "save conversation as json"
  Parameters: format (enum), path (string)

- ImportConversation: Load chat history
  Examples: "import conversation from file"
  Parameters: path (string)

## Advanced Operations
- ScheduleTask: Schedule recurring command
  Examples: "schedule task \"backup.sh\" daily", "run cleanup every hour"
  Parameters: command (string), schedule (string), name (optional string)

- ListScheduledTasks: Show scheduled tasks
  Examples: "list scheduled tasks"
  Parameters: none

- CancelScheduledTask: Cancel scheduled command
  Examples: "cancel task backup", "remove scheduled cleanup"
  Parameters: name (string)

PARSING RULES:
1. Understand user intent, not just keywords
2. Handle natural language variations and synonyms
3. Extract parameters intelligently from context
4. Convert time units (1 minute = 60 seconds)
5. Preserve exact case for filenames (Cargo.toml, not cargo.toml)
6. For "watch the X" format, the path is X, not "the"
7. Use sensible defaults for optional parameters
8. Handle multiple tools if the request is complex

RESPONSE FORMAT (JSON only):
{{
  "reasoning": "What the user wants to accomplish",
  "tools": [
    {{
      "tool_type": "ToolName",
      "parameters": {{
        "param_name": "value"
      }},
      "reasoning": "Why this tool is needed"
    }}
  ]
}}

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
                    let path = tool_req
                        .parameters
                        .get("path")
                        .or_else(|| tool_req.parameters.get("filename"))
                        .or_else(|| tool_req.parameters.get("file"))
                        .or_else(|| tool_req.parameters.get("filepath"))
                        .and_then(|v| v.as_str());

                    if let Some(path) = path {
                        tools.push(AvailableTool::FileRead {
                            path: path.to_string(),
                        });
                    }
                }
                "FileWrite" => {
                    let path = tool_req
                        .parameters
                        .get("path")
                        .or_else(|| tool_req.parameters.get("filename"))
                        .or_else(|| tool_req.parameters.get("file"))
                        .or_else(|| tool_req.parameters.get("filepath"))
                        .and_then(|v| v.as_str());

                    if let Some(path) = path {
                        let content = tool_req
                            .parameters
                            .get("content")
                            .or_else(|| tool_req.parameters.get("text"))
                            .or_else(|| tool_req.parameters.get("data"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        tools.push(AvailableTool::FileWrite {
                            path: path.to_string(),
                            content,
                        });
                    }
                }
                "FileEdit" => {
                    let path = tool_req
                        .parameters
                        .get("path")
                        .or_else(|| tool_req.parameters.get("filename"))
                        .or_else(|| tool_req.parameters.get("file"))
                        .or_else(|| tool_req.parameters.get("filepath"))
                        .and_then(|v| v.as_str());

                    if let (Some(path), Some(op_val)) = (path, tool_req.parameters.get("operation"))
                    {
                        if let Some(operation) = Self::parse_edit_operation(op_val) {
                            tools.push(AvailableTool::FileEdit {
                                path: path.to_string(),
                                operation,
                            });
                        }
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
                    let model_name = tool_req
                        .parameters
                        .get("model_name")
                        .or_else(|| tool_req.parameters.get("model"))
                        .or_else(|| tool_req.parameters.get("name"))
                        .and_then(|v| v.as_str());

                    if let Some(model_name) = model_name {
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

                // File watching
                "FileWatch" => {
                    // Try multiple parameter names for flexibility
                    let path = tool_req
                        .parameters
                        .get("path")
                        .or_else(|| tool_req.parameters.get("filename"))
                        .or_else(|| tool_req.parameters.get("file"))
                        .or_else(|| tool_req.parameters.get("filepath"))
                        .and_then(|v| v.as_str());

                    if let Some(path) = path {
                        let duration_seconds = tool_req
                            .parameters
                            .get("duration_seconds")
                            .or_else(|| tool_req.parameters.get("duration"))
                            .or_else(|| tool_req.parameters.get("time"))
                            .or_else(|| tool_req.parameters.get("seconds"))
                            .and_then(|v| {
                                // Handle different formats: "60s", "60", 60, "1 minute", "2 minutes"
                                if let Some(s) = v.as_str() {
                                    let s = s.trim().to_lowercase();
                                    if s.ends_with(" minutes") || s.ends_with(" minute") {
                                        s.split_whitespace()
                                            .next()
                                            .and_then(|num| num.parse::<u64>().ok())
                                            .map(|n| n * 60)
                                    } else if s.ends_with(" seconds") || s.ends_with(" second") {
                                        s.split_whitespace()
                                            .next()
                                            .and_then(|num| num.parse::<u64>().ok())
                                    } else if s.ends_with('s') {
                                        s.trim_end_matches('s').parse::<u64>().ok()
                                    } else if s.ends_with("min") {
                                        s.trim_end_matches("min")
                                            .parse::<u64>()
                                            .ok()
                                            .map(|n| n * 60)
                                    } else {
                                        s.parse::<u64>().ok()
                                    }
                                } else {
                                    v.as_u64()
                                }
                            });
                        tools.push(AvailableTool::FileWatch {
                            path: path.to_string(),
                            duration_seconds,
                        });
                    }
                }

                "FileSearch" => {
                    let pattern = tool_req.parameters.get("pattern").and_then(|v| v.as_str());
                    let directory = tool_req
                        .parameters
                        .get("directory")
                        .and_then(|v| v.as_str());
                    if let Some(pattern) = pattern {
                        tools.push(AvailableTool::FileSearch {
                            pattern: pattern.to_string(),
                            directory: directory.map(|d| d.to_string()),
                        });
                    }
                }
                "ContentSearch" => {
                    let pattern = tool_req.parameters.get("pattern").and_then(|v| v.as_str());
                    let directory = tool_req
                        .parameters
                        .get("directory")
                        .and_then(|v| v.as_str());
                    if let Some(pattern) = pattern {
                        tools.push(AvailableTool::ContentSearch {
                            pattern: pattern.to_string(),
                            directory: directory.map(|d| d.to_string()),
                        });
                    }
                }
                "ListDirectory" => {
                    let path = tool_req
                        .parameters
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");
                    tools.push(AvailableTool::ListDirectory {
                        path: path.to_string(),
                    });
                }
                "CreateProject" => {
                    if let Some(name) = tool_req.parameters.get("name").and_then(|v| v.as_str()) {
                        let project_type = tool_req
                            .parameters
                            .get("project_type")
                            .or_else(|| tool_req.parameters.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("generic");
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
                "ExecuteCommand" => {
                    if let Some(command) =
                        tool_req.parameters.get("command").and_then(|v| v.as_str())
                    {
                        tools.push(AvailableTool::ExecuteCommand {
                            command: command.to_string(),
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

                "GitPush" => {
                    let remote = tool_req
                        .parameters
                        .get("remote")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let branch = tool_req
                        .parameters
                        .get("branch")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GitPush {
                        remote,
                        branch,
                        repository_path,
                    });
                }
                "GitPull" => {
                    let remote = tool_req
                        .parameters
                        .get("remote")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let branch = tool_req
                        .parameters
                        .get("branch")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GitPull {
                        remote,
                        branch,
                        repository_path,
                    });
                }
                "GitBranch" => {
                    if let Some(op) = tool_req
                        .parameters
                        .get("operation")
                        .and_then(|v| v.as_str())
                    {
                        let operation = match op.to_lowercase().as_str() {
                            "list" => GitBranchOperation::List,
                            "create" => {
                                if let Some(name) =
                                    tool_req.parameters.get("name").and_then(|v| v.as_str())
                                {
                                    GitBranchOperation::Create {
                                        name: name.to_string(),
                                    }
                                } else {
                                    GitBranchOperation::List
                                }
                            }
                            "switch" => {
                                if let Some(name) =
                                    tool_req.parameters.get("name").and_then(|v| v.as_str())
                                {
                                    GitBranchOperation::Switch {
                                        name: name.to_string(),
                                    }
                                } else {
                                    GitBranchOperation::List
                                }
                            }
                            "delete" => {
                                if let Some(name) =
                                    tool_req.parameters.get("name").and_then(|v| v.as_str())
                                {
                                    GitBranchOperation::Delete {
                                        name: name.to_string(),
                                    }
                                } else {
                                    GitBranchOperation::List
                                }
                            }
                            "merge" => {
                                if let Some(from) =
                                    tool_req.parameters.get("from").and_then(|v| v.as_str())
                                {
                                    GitBranchOperation::Merge {
                                        from: from.to_string(),
                                    }
                                } else {
                                    GitBranchOperation::List
                                }
                            }
                            _ => GitBranchOperation::List,
                        };
                        let repository_path = tool_req
                            .parameters
                            .get("repository_path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::GitBranch {
                            operation,
                            repository_path,
                        });
                    }
                }
                "GitLog" => {
                    let count = tool_req
                        .parameters
                        .get("count")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u32);
                    let oneline = tool_req
                        .parameters
                        .get("oneline")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GitLog {
                        count,
                        oneline,
                        repository_path,
                    });
                }
                "GitDiff" => {
                    let file = tool_req
                        .parameters
                        .get("file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let cached = tool_req
                        .parameters
                        .get("cached")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let repository_path = tool_req
                        .parameters
                        .get("repository_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GitDiff {
                        file,
                        cached,
                        repository_path,
                    });
                }

                "RestApiCall" => {
                    if let Some(endpoint) =
                        tool_req.parameters.get("endpoint").and_then(|v| v.as_str())
                    {
                        let operation = tool_req
                            .parameters
                            .get("operation")
                            .and_then(|v| v.as_str())
                            .unwrap_or("get");
                        let data = tool_req.parameters.get("data").cloned();
                        let auth = tool_req
                            .parameters
                            .get("auth")
                            .cloned()
                            .and_then(|v| Self::parse_api_auth(&v));
                        let op_enum = match operation.to_lowercase().as_str() {
                            "create" => RestOperation::Create {
                                data: data.clone().unwrap_or(serde_json::Value::Null),
                            },
                            "update" => {
                                if let Some(id) =
                                    tool_req.parameters.get("id").and_then(|v| v.as_str())
                                {
                                    RestOperation::Update {
                                        id: id.to_string(),
                                        data: data.clone().unwrap_or(serde_json::Value::Null),
                                    }
                                } else {
                                    RestOperation::Get
                                }
                            }
                            "delete" => {
                                if let Some(id) =
                                    tool_req.parameters.get("id").and_then(|v| v.as_str())
                                {
                                    RestOperation::Delete { id: id.to_string() }
                                } else {
                                    RestOperation::Get
                                }
                            }
                            _ => RestOperation::Get,
                        };
                        tools.push(AvailableTool::RestApiCall {
                            endpoint: endpoint.to_string(),
                            operation: op_enum,
                            data,
                            auth,
                        });
                    }
                }
                "GraphQLQuery" => {
                    if let Some(endpoint) =
                        tool_req.parameters.get("endpoint").and_then(|v| v.as_str())
                    {
                        if let Some(query) =
                            tool_req.parameters.get("query").and_then(|v| v.as_str())
                        {
                            let variables = tool_req.parameters.get("variables").cloned();
                            let auth = tool_req
                                .parameters
                                .get("auth")
                                .cloned()
                                .and_then(|v| Self::parse_api_auth(&v));
                            tools.push(AvailableTool::GraphQLQuery {
                                endpoint: endpoint.to_string(),
                                query: query.to_string(),
                                variables,
                                auth,
                            });
                        }
                    }
                }
                "SqlQuery" => {
                    if let (Some(conn), Some(query)) = (
                        tool_req
                            .parameters
                            .get("connection_string")
                            .and_then(|v| v.as_str()),
                        tool_req.parameters.get("query").and_then(|v| v.as_str()),
                    ) {
                        let db_type = tool_req
                            .parameters
                            .get("database_type")
                            .and_then(|v| v.as_str())
                            .and_then(Self::parse_database_type)
                            .unwrap_or(DatabaseType::SQLite);
                        tools.push(AvailableTool::SqlQuery {
                            connection_string: conn.to_string(),
                            query: query.to_string(),
                            database_type: db_type,
                        });
                    }
                }
                "SqliteQuery" => {
                    if let (Some(path), Some(query)) = (
                        tool_req
                            .parameters
                            .get("database_path")
                            .and_then(|v| v.as_str()),
                        tool_req.parameters.get("query").and_then(|v| v.as_str()),
                    ) {
                        tools.push(AvailableTool::SqliteQuery {
                            database_path: path.to_string(),
                            query: query.to_string(),
                        });
                    }
                }
                "CargoOperation" => {
                    if let Some(op) = tool_req
                        .parameters
                        .get("operation")
                        .and_then(|v| v.as_str())
                    {
                        let operation = Self::parse_cargo_operation(op);
                        let package = tool_req
                            .parameters
                            .get("package")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let features = tool_req
                            .parameters
                            .get("features")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            });
                        tools.push(AvailableTool::CargoOperation {
                            operation,
                            package,
                            features,
                        });
                    }
                }
                "NpmOperation" => {
                    if let Some(op) = tool_req
                        .parameters
                        .get("operation")
                        .and_then(|v| v.as_str())
                    {
                        let operation = if let Some(map) = tool_req.parameters.as_object() {
                            Self::parse_npm_operation(op, map)
                        } else {
                            Self::parse_npm_operation(op, &serde_json::Map::new())
                        };
                        let package = tool_req
                            .parameters
                            .get("package")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let dev = tool_req
                            .parameters
                            .get("dev")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        tools.push(AvailableTool::NpmOperation {
                            operation,
                            package,
                            dev,
                        });
                    }
                }
                "PipOperation" => {
                    if let Some(op) = tool_req
                        .parameters
                        .get("operation")
                        .and_then(|v| v.as_str())
                    {
                        let operation = Self::parse_pip_operation(op);
                        let package = tool_req
                            .parameters
                            .get("package")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let requirements_file = tool_req
                            .parameters
                            .get("requirements_file")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::PipOperation {
                            operation,
                            package,
                            requirements_file,
                        });
                    }
                }
                "NetworkInfo" => {
                    tools.push(AvailableTool::NetworkInfo);
                }
                "DockerList" => {
                    if let Some(res) = tool_req
                        .parameters
                        .get("resource_type")
                        .and_then(|v| v.as_str())
                    {
                        let resource_type = Self::parse_docker_resource(res);
                        tools.push(AvailableTool::DockerList { resource_type });
                    }
                }
                "DockerRun" => {
                    if let Some(image) = tool_req.parameters.get("image").and_then(|v| v.as_str()) {
                        let command = tool_req
                            .parameters
                            .get("command")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let ports = tool_req
                            .parameters
                            .get("ports")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            });
                        let volumes = tool_req
                            .parameters
                            .get("volumes")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            });
                        let environment = tool_req
                            .parameters
                            .get("environment")
                            .and_then(|v| v.as_object())
                            .map(|obj| {
                                obj.iter()
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|s| (k.clone(), s.to_string()))
                                    })
                                    .collect()
                            });
                        tools.push(AvailableTool::DockerRun {
                            image: image.to_string(),
                            command,
                            ports,
                            volumes,
                            environment,
                        });
                    }
                }
                "DockerStop" => {
                    if let Some(container) = tool_req
                        .parameters
                        .get("container")
                        .and_then(|v| v.as_str())
                    {
                        tools.push(AvailableTool::DockerStop {
                            container: container.to_string(),
                        });
                    }
                }
                "DockerLogs" => {
                    if let Some(container) = tool_req
                        .parameters
                        .get("container")
                        .and_then(|v| v.as_str())
                    {
                        let follow = tool_req
                            .parameters
                            .get("follow")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let tail = tool_req
                            .parameters
                            .get("tail")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as u32);
                        tools.push(AvailableTool::DockerLogs {
                            container: container.to_string(),
                            follow,
                            tail,
                        });
                    }
                }
                "JsonFormat" => {
                    if let Some(input) = tool_req.parameters.get("input").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::JsonFormat {
                            input: input.to_string(),
                        });
                    }
                }
                "JsonQuery" => {
                    if let (Some(input), Some(query)) = (
                        tool_req.parameters.get("input").and_then(|v| v.as_str()),
                        tool_req.parameters.get("query").and_then(|v| v.as_str()),
                    ) {
                        tools.push(AvailableTool::JsonQuery {
                            input: input.to_string(),
                            query: query.to_string(),
                        });
                    }
                }
                "CsvParse" => {
                    if let Some(input) = tool_req.parameters.get("input").and_then(|v| v.as_str()) {
                        let delimiter = tool_req
                            .parameters
                            .get("delimiter")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.chars().next());
                        tools.push(AvailableTool::CsvParse {
                            input: input.to_string(),
                            delimiter,
                        });
                    }
                }
                "RegexMatch" => {
                    if let (Some(pattern), Some(text)) = (
                        tool_req.parameters.get("pattern").and_then(|v| v.as_str()),
                        tool_req.parameters.get("text").and_then(|v| v.as_str()),
                    ) {
                        let flags = tool_req
                            .parameters
                            .get("flags")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::RegexMatch {
                            pattern: pattern.to_string(),
                            text: text.to_string(),
                            flags,
                        });
                    }
                }
                "TextTransform" => {
                    if let (Some(input), Some(op)) = (
                        tool_req.parameters.get("input").and_then(|v| v.as_str()),
                        tool_req
                            .parameters
                            .get("operation")
                            .and_then(|v| v.as_str()),
                    ) {
                        let operation = if let Some(map) = tool_req.parameters.as_object() {
                            Self::parse_text_operation(op, map)
                        } else {
                            Self::parse_text_operation(op, &serde_json::Map::new())
                        };
                        tools.push(AvailableTool::TextTransform {
                            input: input.to_string(),
                            operation,
                        });
                    }
                }
                "SetConfig" => {
                    if let (Some(key), Some(value)) = (
                        tool_req.parameters.get("key").and_then(|v| v.as_str()),
                        tool_req.parameters.get("value"),
                    ) {
                        tools.push(AvailableTool::SetConfig {
                            key: key.to_string(),
                            value: value.clone(),
                        });
                    }
                }
                "GetConfig" => {
                    let key = tool_req
                        .parameters
                        .get("key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tools.push(AvailableTool::GetConfig { key });
                }
                "ExportConversation" => {
                    if let (Some(format_str), Some(path)) = (
                        tool_req.parameters.get("format").and_then(|v| v.as_str()),
                        tool_req.parameters.get("path").and_then(|v| v.as_str()),
                    ) {
                        let format = Self::parse_export_format(format_str);
                        tools.push(AvailableTool::ExportConversation {
                            format,
                            path: path.to_string(),
                        });
                    }
                }
                "ImportConversation" => {
                    if let Some(path) = tool_req.parameters.get("path").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::ImportConversation {
                            path: path.to_string(),
                        });
                    }
                }
                "ClearHistory" => {
                    tools.push(AvailableTool::ClearHistory);
                }
                "ScheduleTask" => {
                    if let (Some(command), Some(schedule)) = (
                        tool_req.parameters.get("command").and_then(|v| v.as_str()),
                        tool_req.parameters.get("schedule").and_then(|v| v.as_str()),
                    ) {
                        let name = tool_req
                            .parameters
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        tools.push(AvailableTool::ScheduleTask {
                            command: command.to_string(),
                            schedule: schedule.to_string(),
                            name,
                        });
                    }
                }
                "ListScheduledTasks" => {
                    tools.push(AvailableTool::ListScheduledTasks);
                }
                "CancelScheduledTask" => {
                    if let Some(name) = tool_req.parameters.get("name").and_then(|v| v.as_str()) {
                        tools.push(AvailableTool::CancelScheduledTask {
                            name: name.to_string(),
                        });
                    }
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

    fn parse_edit_operation(value: &serde_json::Value) -> Option<EditOperation> {
        let obj = value.as_object()?;
        let op = obj.get("type")?.as_str()?.to_lowercase();
        match op.as_str() {
            "replace" => Some(EditOperation::Replace {
                old: obj.get("old")?.as_str()?.to_string(),
                new: obj.get("new")?.as_str()?.to_string(),
            }),
            "insert" => Some(EditOperation::Insert {
                line: obj.get("line")?.as_u64()? as usize,
                content: obj.get("content")?.as_str()?.to_string(),
            }),
            "append" => Some(EditOperation::Append {
                content: obj.get("content")?.as_str()?.to_string(),
            }),
            "delete" => Some(EditOperation::Delete {
                line_start: obj.get("line_start")?.as_u64()? as usize,
                line_end: obj
                    .get("line_end")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize),
            }),
            _ => None,
        }
    }

    fn parse_api_auth(value: &serde_json::Value) -> Option<ApiAuth> {
        let obj = value.as_object()?;
        let kind = obj.get("type")?.as_str()?.to_lowercase();
        match kind.as_str() {
            "bearer" => obj
                .get("token")
                .and_then(|v| v.as_str())
                .map(|t| ApiAuth::Bearer {
                    token: t.to_string(),
                }),
            "basic" => Some(ApiAuth::Basic {
                username: obj.get("username")?.as_str()?.to_string(),
                password: obj.get("password")?.as_str()?.to_string(),
            }),
            "apikey" => Some(ApiAuth::ApiKey {
                key: obj.get("key")?.as_str()?.to_string(),
                header: obj.get("header")?.as_str()?.to_string(),
            }),
            _ => None,
        }
    }

    fn parse_database_type(s: &str) -> Option<DatabaseType> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Some(DatabaseType::PostgreSQL),
            "mysql" => Some(DatabaseType::MySQL),
            "sqlite" => Some(DatabaseType::SQLite),
            "mongodb" => Some(DatabaseType::MongoDB),
            _ => None,
        }
    }

    fn parse_cargo_operation(s: &str) -> CargoOperation {
        match s.to_lowercase().as_str() {
            "build" => CargoOperation::Build,
            "run" => CargoOperation::Run,
            "test" => CargoOperation::Test,
            "check" => CargoOperation::Check,
            "install" => CargoOperation::Install,
            "add" => CargoOperation::Add,
            "remove" => CargoOperation::Remove,
            "update" => CargoOperation::Update,
            "clean" => CargoOperation::Clean,
            _ => CargoOperation::Build,
        }
    }

    fn parse_npm_operation(
        op: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> NpmOperation {
        match op.to_lowercase().as_str() {
            "install" => NpmOperation::Install,
            "uninstall" => NpmOperation::Uninstall,
            "update" => NpmOperation::Update,
            "audit" => NpmOperation::Audit,
            "run" => {
                let script = params
                    .get("script")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                NpmOperation::Run { script }
            }
            "list" => NpmOperation::List,
            _ => NpmOperation::List,
        }
    }

    fn parse_pip_operation(op: &str) -> PipOperation {
        match op.to_lowercase().as_str() {
            "install" => PipOperation::Install,
            "uninstall" => PipOperation::Uninstall,
            "list" => PipOperation::List,
            "freeze" => PipOperation::Freeze,
            "show" => PipOperation::Show,
            _ => PipOperation::List,
        }
    }

    fn parse_docker_resource(s: &str) -> DockerResourceType {
        match s.to_lowercase().as_str() {
            "containers" => DockerResourceType::Containers,
            "images" => DockerResourceType::Images,
            "volumes" => DockerResourceType::Volumes,
            "networks" => DockerResourceType::Networks,
            _ => DockerResourceType::Containers,
        }
    }

    fn parse_text_operation(
        op: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> TextOperation {
        match op.to_lowercase().as_str() {
            "upper" | "uppercase" => TextOperation::ToUpperCase,
            "lower" | "lowercase" => TextOperation::ToLowerCase,
            "trim" => TextOperation::Trim,
            "count" => TextOperation::Count {
                pattern: params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "replace" => TextOperation::Replace {
                old: params
                    .get("old")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                new: params
                    .get("new")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "split" => TextOperation::Split {
                delimiter: params
                    .get("delimiter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "join" => TextOperation::Join {
                delimiter: params
                    .get("delimiter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            _ => TextOperation::Trim,
        }
    }

    fn parse_export_format(s: &str) -> ExportFormat {
        match s.to_lowercase().as_str() {
            "json" => ExportFormat::Json,
            "markdown" | "md" => ExportFormat::Markdown,
            "text" | "txt" => ExportFormat::Text,
            "html" => ExportFormat::Html,
            _ => ExportFormat::Json,
        }
    }

    fn enhanced_fallback_parse(&self, _input: &str) -> Vec<AvailableTool> {
        // Remove fallback parsing - let LLM handle everything
        // If LLM fails, return empty vec to trigger general conversation
        vec![]
    }

    fn simple_fallback_parse(&self, _input: &str) -> Vec<AvailableTool> {
        // Remove all fallback parsing - let LLM handle everything
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
