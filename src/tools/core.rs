use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Tool definition system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub command: Option<String>,
    pub requires_permission: bool,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// Available tools enum - significantly extended
#[derive(Debug, Clone)]
pub enum AvailableTool {
    // File Operations
    WebSearch {
        query: String,
    },
    WebScrape {
        url: String,
    },
    FileSearch {
        pattern: String,
        directory: Option<String>,
    },
    FileRead {
        path: String,
    },
    FileWrite {
        path: String,
        content: String,
    },
    FileEdit {
        path: String,
        operation: EditOperation,
    },
    ContentSearch {
        pattern: String,
        directory: Option<String>,
    },
    CreateProject {
        name: String,
        project_type: String,
        path: Option<String>,
    },
    ExecuteCommand {
        command: String,
    },
    GenerateCommand {
        user_request: String,
        context: Option<String>,
    },
    ListDirectory {
        path: String,
    },
    FileWatch {
        path: String,
        duration_seconds: Option<u64>,
    },

    // Git Operations
    GitStatus {
        repository_path: Option<String>,
    },
    GitAdd {
        files: Vec<String>,
        repository_path: Option<String>,
    },
    GitCommit {
        message: String,
        repository_path: Option<String>,
    },
    GitPush {
        remote: Option<String>,
        branch: Option<String>,
        repository_path: Option<String>,
    },
    GitPull {
        remote: Option<String>,
        branch: Option<String>,
        repository_path: Option<String>,
    },
    GitBranch {
        operation: GitBranchOperation,
        repository_path: Option<String>,
    },
    GitLog {
        count: Option<u32>,
        oneline: bool,
        repository_path: Option<String>,
    },
    GitDiff {
        file: Option<String>,
        cached: bool,
        repository_path: Option<String>,
    },

    // API Operations
    HttpRequest {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        timeout_seconds: Option<u64>,
    },
    RestApiCall {
        endpoint: String,
        operation: RestOperation,
        data: Option<serde_json::Value>,
        auth: Option<ApiAuth>,
    },
    GraphQLQuery {
        endpoint: String,
        query: String,
        variables: Option<serde_json::Value>,
        auth: Option<ApiAuth>,
    },

    // Database Operations
    SqlQuery {
        connection_string: String,
        query: String,
        database_type: DatabaseType,
    },
    SqliteQuery {
        database_path: String,
        query: String,
    },

    // Package Management
    CargoOperation {
        operation: CargoOperation,
        package: Option<String>,
        features: Option<Vec<String>>,
    },
    NpmOperation {
        operation: NpmOperation,
        package: Option<String>,
        dev: bool,
    },
    PipOperation {
        operation: PipOperation,
        package: Option<String>,
        requirements_file: Option<String>,
    },

    // System Operations
    ProcessList {
        filter: Option<String>,
    },
    SystemInfo,
    DiskUsage {
        path: Option<String>,
    },
    MemoryUsage,
    NetworkInfo,

    // Docker Operations
    DockerList {
        resource_type: DockerResourceType,
    },
    DockerRun {
        image: String,
        command: Option<String>,
        ports: Option<Vec<String>>,
        volumes: Option<Vec<String>>,
        environment: Option<HashMap<String, String>>,
    },
    DockerStop {
        container: String,
    },
    DockerLogs {
        container: String,
        follow: bool,
        tail: Option<u32>,
    },

    // Text Processing
    JsonFormat {
        input: String,
    },
    JsonQuery {
        input: String,
        query: String, // JSONPath or jq syntax
    },
    CsvParse {
        input: String,
        delimiter: Option<char>,
    },
    RegexMatch {
        pattern: String,
        text: String,
        flags: Option<String>,
    },
    TextTransform {
        input: String,
        operation: TextOperation,
    },

    // Model Configuration
    SetModelParameter {
        parameter: ModelParameter,
        value: serde_json::Value,
    },
    GetModelParameter {
        parameter: Option<ModelParameter>,
    },
    SwitchModel {
        model_name: String,
    },

    // Configuration & Session
    SetConfig {
        key: String,
        value: serde_json::Value,
    },
    GetConfig {
        key: Option<String>,
    },
    ExportConversation {
        format: ExportFormat,
        path: String,
    },
    ImportConversation {
        path: String,
    },
    ClearHistory,

    // Advanced Operations
    ScheduleTask {
        command: String,
        schedule: String, // cron-like syntax
        name: Option<String>,
    },
    ListScheduledTasks,
    CancelScheduledTask {
        name: String,
    },
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    Replace {
        old: String,
        new: String,
    },
    Insert {
        line: usize,
        content: String,
    },
    Append {
        content: String,
    },
    Delete {
        line_start: usize,
        line_end: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub enum GitBranchOperation {
    List,
    Create { name: String },
    Switch { name: String },
    Delete { name: String },
    Merge { from: String },
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
}

#[derive(Debug, Clone)]
pub enum RestOperation {
    Get,
    Create { data: serde_json::Value },
    Update { id: String, data: serde_json::Value },
    Delete { id: String },
}

#[derive(Debug, Clone)]
pub enum ApiAuth {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, header: String },
}

#[derive(Debug, Clone)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
}

#[derive(Debug, Clone)]
pub enum CargoOperation {
    Build,
    Run,
    Test,
    Check,
    Install,
    Add,
    Remove,
    Update,
    Clean,
}

#[derive(Debug, Clone)]
pub enum NpmOperation {
    Install,
    Uninstall,
    Update,
    Audit,
    Run { script: String },
    List,
}

#[derive(Debug, Clone)]
pub enum PipOperation {
    Install,
    Uninstall,
    List,
    Freeze,
    Show,
}

#[derive(Debug, Clone)]
pub enum DockerResourceType {
    Containers,
    Images,
    Volumes,
    Networks,
}

#[derive(Debug, Clone)]
pub enum TextOperation {
    ToUpperCase,
    ToLowerCase,
    Trim,
    Count { pattern: String },
    Replace { old: String, new: String },
    Split { delimiter: String },
    Join { delimiter: String },
}

#[derive(Debug, Clone)]
pub enum ModelParameter {
    Temperature,
    MaxTokens,
    TopP,
    TopK,
    RepeatPenalty,
    SystemPrompt,
    ContextLength,
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Markdown,
    Text,
    Html,
}

// Tool executor
pub struct ToolExecutor {
    pub web_client: reqwest::Client,
    pub config: ToolConfig,
}

#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub auto_approve_safe: bool,
    pub max_file_size: usize,
    pub default_timeout: u64,
    pub git_default_remote: String,
    pub database_connections: HashMap<String, String>,
    pub api_keys: HashMap<String, String>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            auto_approve_safe: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            default_timeout: 30,
            git_default_remote: "origin".to_string(),
            database_connections: HashMap::new(),
            api_keys: HashMap::new(),
        }
    }
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            web_client: reqwest::Client::new(),
            config: ToolConfig::default(),
        }
    }

    pub fn with_config(config: ToolConfig) -> Self {
        Self {
            web_client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn execute_tool(
        &self,
        tool: AvailableTool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        match tool {
            // Existing tools
            AvailableTool::WebSearch { query } => self.web_search(&query).await,
            AvailableTool::WebScrape { url } => self.web_scrape(&url).await,
            AvailableTool::FileSearch { pattern, directory } => {
                self.file_search(&pattern, directory.as_deref())
            }
            AvailableTool::FileRead { path } => self.file_read(&path),
            AvailableTool::FileWrite { path, content } => self.file_write(&path, &content),
            AvailableTool::FileEdit { path, operation } => self.file_edit(&path, operation),
            AvailableTool::ContentSearch { pattern, directory } => {
                self.content_search(&pattern, directory.as_deref())
            }
            AvailableTool::CreateProject {
                name,
                project_type,
                path,
            } => self.create_project(&name, &project_type, path.as_deref()),
            AvailableTool::ExecuteCommand { command } => self.execute_command(&command).await,
            AvailableTool::GenerateCommand { user_request, context } => {
                self.generate_command(&user_request, context.as_deref()).await
            },
            AvailableTool::ListDirectory { path } => self.list_directory(&path),

            // New tools
            AvailableTool::FileWatch {
                path,
                duration_seconds,
            } => self.file_watch(&path, duration_seconds).await,

            // Git operations
            AvailableTool::GitStatus { repository_path } => {
                self.git_status(repository_path.as_deref()).await
            }
            AvailableTool::GitAdd {
                files,
                repository_path,
            } => self.git_add(&files, repository_path.as_deref()).await,
            AvailableTool::GitCommit {
                message,
                repository_path,
            } => self.git_commit(&message, repository_path.as_deref()).await,
            AvailableTool::GitPush {
                remote,
                branch,
                repository_path,
            } => {
                self.git_push(
                    remote.as_deref(),
                    branch.as_deref(),
                    repository_path.as_deref(),
                )
                .await
            }
            AvailableTool::GitPull {
                remote,
                branch,
                repository_path,
            } => {
                self.git_pull(
                    remote.as_deref(),
                    branch.as_deref(),
                    repository_path.as_deref(),
                )
                .await
            }
            AvailableTool::GitBranch {
                operation,
                repository_path,
            } => self.git_branch(operation, repository_path.as_deref()).await,
            AvailableTool::GitLog {
                count,
                oneline,
                repository_path,
            } => {
                self.git_log(count, oneline, repository_path.as_deref())
                    .await
            }
            AvailableTool::GitDiff {
                file,
                cached,
                repository_path,
            } => {
                self.git_diff(file.as_deref(), cached, repository_path.as_deref())
                    .await
            }

            // API operations
            AvailableTool::HttpRequest {
                method,
                url,
                headers,
                body,
                timeout_seconds,
            } => {
                self.http_request(method, &url, headers, body, timeout_seconds)
                    .await
            }
            AvailableTool::RestApiCall {
                endpoint,
                operation,
                data,
                auth,
            } => self.rest_api_call(&endpoint, operation, data, auth).await,
            AvailableTool::GraphQLQuery {
                endpoint,
                query,
                variables,
                auth,
            } => self.graphql_query(&endpoint, &query, variables, auth).await,

            // Database operations
            AvailableTool::SqlQuery {
                connection_string,
                query,
                database_type,
            } => {
                self.sql_query(&connection_string, &query, database_type)
                    .await
            }
            AvailableTool::SqliteQuery {
                database_path,
                query,
            } => self.sqlite_query(&database_path, &query).await,

            // Package management
            AvailableTool::CargoOperation {
                operation,
                package,
                features,
            } => {
                self.cargo_operation(operation, package.as_deref(), features)
                    .await
            }
            AvailableTool::NpmOperation {
                operation,
                package,
                dev,
            } => self.npm_operation(operation, package.as_deref(), dev).await,
            AvailableTool::PipOperation {
                operation,
                package,
                requirements_file,
            } => {
                self.pip_operation(operation, package.as_deref(), requirements_file.as_deref())
                    .await
            }

            // System operations
            AvailableTool::ProcessList { filter } => self.process_list(filter.as_deref()).await,
            AvailableTool::SystemInfo => self.system_info().await,
            AvailableTool::DiskUsage { path } => self.disk_usage(path.as_deref()).await,
            AvailableTool::MemoryUsage => self.memory_usage().await,
            AvailableTool::NetworkInfo => self.network_info().await,

            // Docker operations
            AvailableTool::DockerList { resource_type } => self.docker_list(resource_type).await,
            AvailableTool::DockerRun {
                image,
                command,
                ports,
                volumes,
                environment,
            } => {
                self.docker_run(&image, command, ports, volumes, environment)
                    .await
            }
            AvailableTool::DockerStop { container } => self.docker_stop(&container).await,
            AvailableTool::DockerLogs {
                container,
                follow,
                tail,
            } => self.docker_logs(&container, follow, tail).await,

            // Text processing
            AvailableTool::JsonFormat { input } => self.json_format(&input),
            AvailableTool::JsonQuery { input, query } => self.json_query(&input, &query),
            AvailableTool::CsvParse { input, delimiter } => self.csv_parse(&input, delimiter),
            AvailableTool::RegexMatch {
                pattern,
                text,
                flags,
            } => self.regex_match(&pattern, &text, flags.as_deref()),
            AvailableTool::TextTransform { input, operation } => {
                self.text_transform(&input, operation)
            }

            // Model configuration
            AvailableTool::SetModelParameter { parameter, value } => {
                self.set_model_parameter(parameter, value).await
            }
            AvailableTool::GetModelParameter { parameter } => {
                self.get_model_parameter(parameter).await
            }
            AvailableTool::SwitchModel { model_name } => self.switch_model(&model_name).await,

            // Configuration & session
            AvailableTool::SetConfig { key, value } => self.set_config(&key, value).await,
            AvailableTool::GetConfig { key } => self.get_config(key.as_deref()).await,
            AvailableTool::ExportConversation { format, path } => {
                self.export_conversation(format, &path).await
            }
            AvailableTool::ImportConversation { path } => self.import_conversation(&path).await,
            AvailableTool::ClearHistory => self.clear_history().await,

            // Advanced operations
            AvailableTool::ScheduleTask {
                command,
                schedule,
                name,
            } => {
                self.schedule_task(&command, &schedule, name.as_deref())
                    .await
            }
            AvailableTool::ListScheduledTasks => self.list_scheduled_tasks().await,
            AvailableTool::CancelScheduledTask { name } => self.cancel_scheduled_task(&name).await,
        }
    }
}
