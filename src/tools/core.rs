use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc as StdArc;

// Tool definition system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub command: Option<String>,
    pub requires_permission: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
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
    SystemPackageManager {
        operation: PackageManagerOperation,
        package: Option<String>,
    },
    ServiceManager {
        operation: ServiceOperation,
        service_name: String,
    },
    EnvironmentInfo,
    NetworkScan {
        target: String,
        scan_type: NetworkScanType,
    },

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
    ParallelExecution {
        tools: Vec<AvailableTool>,
    },
    SmartSuggestion {
        context: String,
        current_goal: String,
    },
    PerformanceMonitor {
        operation: MonitorOperation,
    },
    CodeAnalysis {
        path: String,
        analysis_type: CodeAnalysisType,
    },
    SecurityScan {
        target: String,
        scan_depth: SecurityScanDepth,
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
    // PostgreSQL,
    // MySQL,
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

#[derive(Debug, Clone)]
pub enum PackageManagerOperation {
    Install,
    Remove,
    Update,
    Search,
    List,
    Info,
    CheckInstalled,
}

#[derive(Debug, Clone)]
pub enum ServiceOperation {
    Start,
    Stop,
    Restart,
    Status,
    Enable,
    Disable,
    List,
}

#[derive(Debug, Clone)]
pub enum NetworkScanType {
    Port,
    Ping,
    Discovery,
    Traceroute,
}

#[derive(Debug, Clone)]
pub enum MonitorOperation {
    Start,
    Stop,
    Status,
    Report,
}

#[derive(Debug, Clone)]
pub enum CodeAnalysisType {
    Complexity,
    Dependencies,
    Security,
    Performance,
    Documentation,
    TestCoverage,
}

#[derive(Debug, Clone)]
pub enum SecurityScanDepth {
    Quick,
    Standard,
    Deep,
    Compliance,
}

// Tool executor
pub struct ToolExecutor {
    pub web_client: reqwest::Client,
    pub config: ToolConfig,
    pub execution_depth: StdArc<AtomicU32>,
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
            execution_depth: StdArc::new(AtomicU32::new(0)),
        }
    }

    pub fn with_config(config: ToolConfig) -> Self {
        Self {
            web_client: reqwest::Client::new(),
            config,
            execution_depth: StdArc::new(AtomicU32::new(0)),
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
            AvailableTool::GenerateCommand {
                user_request,
                context,
            } => {
                self.generate_command(&user_request, context.as_deref())
                    .await
            }
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

            // Enhanced system operations
            AvailableTool::SystemPackageManager { operation, package } => {
                self.system_package_manager(operation, package.as_deref())
                    .await
            }
            AvailableTool::ServiceManager {
                operation,
                service_name,
            } => self.service_manager(operation, &service_name).await,
            AvailableTool::EnvironmentInfo => self.environment_info().await,
            AvailableTool::NetworkScan { target, scan_type } => {
                self.network_scan(&target, scan_type).await
            }

            // Enhanced advanced operations
            AvailableTool::ParallelExecution { tools } => {
                self.parallel_execution_safe(&tools).await
            }
            AvailableTool::SmartSuggestion {
                context,
                current_goal,
            } => self.smart_suggestion(&context, &current_goal).await,
            AvailableTool::PerformanceMonitor { operation } => {
                self.performance_monitor(operation).await
            }
            AvailableTool::CodeAnalysis {
                path,
                analysis_type,
            } => self.code_analysis(&path, analysis_type).await,
            AvailableTool::SecurityScan { target, scan_depth } => {
                self.security_scan(&target, scan_depth).await
            }
        }
    }

    /// Execute multiple tools in parallel with recursion protection
    pub async fn parallel_execution_safe(
        &self,
        tools: &[AvailableTool],
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        const MAX_DEPTH: u32 = 3;
        const MAX_PARALLEL_TOOLS: usize = 5;
        
        // Check recursion depth
        let current_depth = self.execution_depth.load(Ordering::Relaxed);
        if current_depth >= MAX_DEPTH {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Maximum execution depth {} reached to prevent infinite recursion",
                    MAX_DEPTH
                )),
                metadata: Some(serde_json::json!({
                    "max_depth_reached": true,
                    "current_depth": current_depth,
                    "tools_count": tools.len()
                })),
            });
        }
        
        // Limit number of parallel tools
        let limited_tools = if tools.len() > MAX_PARALLEL_TOOLS {
            println!(
                "{} Limiting parallel execution to {} tools (requested: {})",
                "⚠️".yellow(),
                MAX_PARALLEL_TOOLS,
                tools.len()
            );
            &tools[..MAX_PARALLEL_TOOLS]
        } else {
            tools
        };
        
        if limited_tools.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No tools to execute".to_string(),
                error: None,
                metadata: Some(serde_json::json!({
                    "tools_executed": 0
                })),
            });
        }
        
        println!(
            "{} Executing {} tools in parallel",
            "⚡".cyan(),
            limited_tools.len()
        );
        
        // Increment depth counter
        self.execution_depth.fetch_add(1, Ordering::Relaxed);
        
        // Create futures for each tool
        let futures: Vec<_> = limited_tools
            .iter()
            .enumerate()
            .map(|(index, tool)| {
                let tool_clone = tool.clone();
                let executor_clone = Self {
                    web_client: self.web_client.clone(),
                    config: self.config.clone(),
                    execution_depth: self.execution_depth.clone(),
                };
                
                async move {
                    let start_time = std::time::Instant::now();
                    let result = executor_clone.execute_tool(tool_clone.clone()).await;
                    let duration = start_time.elapsed();
                    
                    match result {
                        Ok(tool_result) => {
                            println!(
                                "{} Tool {} completed in {:.2}s",
                                "✓".green(),
                                index + 1,
                                duration.as_secs_f64()
                            );
                            (index, Ok(tool_result))
                        }
                        Err(e) => {
                            println!(
                                "{} Tool {} failed in {:.2}s: {}",
                                "✗".red(),
                                index + 1,
                                duration.as_secs_f64(),
                                e
                            );
                            (index, Err(e))
                        }
                    }
                }
            })
            .collect();
        
        // Execute all tools concurrently
        let results = futures::future::join_all(futures).await;
        
        // Decrement depth counter
        self.execution_depth.fetch_sub(1, Ordering::Relaxed);
        
        // Process results
        let mut successful_results = Vec::new();
        let mut failed_results = Vec::new();
        let mut output_parts = Vec::new();
        
        for (index, result) in results {
            match result {
                Ok(tool_result) => {
                    successful_results.push((index, tool_result.clone()));
                    if tool_result.success {
                        output_parts.push(format!(
                            "Tool {}: SUCCESS\n{}",
                            index + 1,
                            tool_result.output
                        ));
                    } else {
                        output_parts.push(format!(
                            "Tool {}: FAILED\n{}",
                            index + 1,
                            tool_result.error.unwrap_or("Unknown error".to_string())
                        ));
                    }
                }
                Err(e) => {
                    failed_results.push((index, e.to_string()));
                    output_parts.push(format!(
                        "Tool {}: ERROR\n{}",
                        index + 1,
                        e
                    ));
                }
            }
        }
        
        let overall_success = failed_results.is_empty() && 
            successful_results.iter().all(|(_, result)| result.success);
        
        let summary = if overall_success {
            format!(
                "{} All {} tools executed successfully",
                "✓".green(),
                limited_tools.len()
            )
        } else {
            format!(
                "{} {}/{} tools completed successfully",
                if successful_results.len() > failed_results.len() { "⚠️".yellow() } else { "✗".red() },
                successful_results.iter().filter(|(_, r)| r.success).count(),
                limited_tools.len()
            )
        };
        
        Ok(ToolResult {
            success: overall_success,
            output: format!(
                "{}\n\n{}\n\n--- Detailed Results ---\n{}",
                summary,
                format!(
                    "Execution Summary:\n- Total tools: {}\n- Successful: {}\n- Failed: {}",
                    limited_tools.len(),
                    successful_results.iter().filter(|(_, r)| r.success).count(),
                    failed_results.len() + successful_results.iter().filter(|(_, r)| !r.success).count()
                ),
                output_parts.join("\n\n---\n\n")
            ),
            error: if failed_results.is_empty() {
                None
            } else {
                Some(format!(
                    "Failed tools: {}",
                    failed_results
                        .iter()
                        .map(|(i, e)| format!("Tool {}: {}", i + 1, e))
                        .collect::<Vec<_>>()
                        .join("; ")
                ))
            },
            metadata: Some(serde_json::json!({
                "parallel_execution": true,
                "total_tools": limited_tools.len(),
                "successful_tools": successful_results.iter().filter(|(_, r)| r.success).count(),
                "failed_tools": failed_results.len() + successful_results.iter().filter(|(_, r)| !r.success).count(),
                "execution_depth": current_depth,
                "max_depth_limit": MAX_DEPTH,
                "max_parallel_limit": MAX_PARALLEL_TOOLS,
                "tools_limited": tools.len() > MAX_PARALLEL_TOOLS
            })),
        })
    }
}
