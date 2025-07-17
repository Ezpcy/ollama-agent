use colored::Colorize;
use std::time::Instant;

use crate::client::{generate_response_silent, stream_response, SelectedModel};
use crate::input::VimInputHandler;
use crate::tools::{
    AsyncToolExecutor, AvailableTool, ConversationEntry, NaturalLanguageParser, PermissionManager,
    ResourceLimits, ToolExecutor,
};

#[derive(Debug, Clone)]
pub enum ResponseMode {
    CommandGeneration,
    ToolExecution(Vec<AvailableTool>),
    GeneralConversation,
}
use crate::workspace::WorkspaceContext;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AssistantSession {
    model: SelectedModel,
    tool_executor: ToolExecutor,
    async_executor: AsyncToolExecutor,
    permission_manager: PermissionManager,
    parser: NaturalLanguageParser,
    conversation_history: Vec<ConversationEntry>,
    session_stats: SessionStats,
    vim_handler: VimInputHandler,
    workspace_context: Option<WorkspaceContext>,
    workspace_files: HashMap<PathBuf, String>,
}

#[derive(Debug, Default)]
struct SessionStats {
    commands_processed: u32,
    tools_executed: u32,
    total_response_time: f64,
    successful_operations: u32,
    failed_operations: u32,
}

impl AssistantSession {
    pub fn new(model: SelectedModel, tool_executor: ToolExecutor) -> Self {
        // Initialize the global config with the selected model
        Self::init_global_config(&model);

        let async_executor = AsyncToolExecutor::new(ResourceLimits::default());

        Self {
            model,
            tool_executor,
            async_executor,
            permission_manager: PermissionManager::new(),
            parser: NaturalLanguageParser::new(),
            conversation_history: Vec::new(),
            session_stats: SessionStats::default(),
            vim_handler: VimInputHandler::new(),
            workspace_context: None,
            workspace_files: HashMap::new(),
        }
    }

    pub fn with_vim_mode(
        model: SelectedModel,
        tool_executor: ToolExecutor,
        vim_enabled: bool,
    ) -> Self {
        let mut session = Self::new(model, tool_executor);
        if vim_enabled {
            session.vim_handler.enable_vim_mode();
        }
        session
    }

    fn init_global_config(model: &SelectedModel) {
        // Update the global model config to reflect the current model
        crate::tools::model_config::set_current_model(&model.name);
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.show_welcome().await;

        loop {
            let user_input = match self.get_user_input() {
                Ok(input) => input,
                Err(_) => {
                    println!("\n{}", "Session ended by user".yellow());
                    break;
                }
            };

            if self.is_exit_command(&user_input) {
                self.show_farewell();
                break;
            }

            if self.is_stats_command(&user_input) {
                self.show_session_stats();
                continue;
            }

            if self.is_help_command(&user_input) {
                self.show_help();
                continue;
            }

            if self.is_switch_model_command(&user_input) {
                if let Err(e) = self.handle_model_switch(&user_input).await {
                    println!("{} Error switching model: {}", "❌".red(), e);
                }
                continue;
            }

            if self.is_performance_command(&user_input) {
                // TODO: Implement performance summary display
                println!("Performance summary not yet implemented");
                continue;
            }

            if self.is_resource_usage_command(&user_input) {
                let usage = self.async_executor.get_resource_usage().await;
                usage.display();
                continue;
            }

            if self.is_clear_logs_command(&user_input) {
                crate::tools::logging::clear_logs().await;
                continue;
            }

            if self.is_toggle_tool_mode_command(&user_input) {
                if let Err(e) = self.handle_toggle_tool_mode().await {
                    println!("{} Error toggling tool mode: {}", "❌".red(), e);
                }
                continue;
            }

            if let Err(e) = self.process_request(&user_input).await {
                println!("{} {}", "Error processing request:".red(), e);
                self.session_stats.failed_operations += 1;
            }

            println!();
        }

        Ok(())
    }

    pub async fn process_single_command(
        &mut self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.process_request(command).await
    }

    async fn show_welcome(&self) {
        println!(
            "{}",
            "🤖 Advanced AI Assistant with System Tools".cyan().bold()
        );
        println!(
            "Model: {} ({})",
            self.model.get_name().yellow(),
            if self.model.is_code_model() {
                "Code Model"
            } else {
                "Chat Model"
            }
            .blue()
        );
        println!();
        println!("{}", "Special Commands:".blue().bold());
        println!("  {} Show session statistics", "stats".yellow());
        println!("  {} Show performance metrics", "performance".yellow());
        println!("  {} Show resource usage", "resources".yellow());
        println!("  {} Clear logs and metrics", "clear logs".yellow());
        println!("  {} Show available commands", "help".yellow());
        println!("  {} Exit the session", "quit/exit".yellow());
        println!();
        println!(
            "{}",
            "LLM-powered tool selection for better accuracy!".green()
        );
        println!(
            "{}",
            "Type your request naturally - I'll figure out what tools to use.".dimmed()
        );
        
        // Show proactive tool mode status
        if let Ok(proactive_enabled) = self.tool_executor.is_proactive_tool_mode_enabled().await {
            if proactive_enabled {
                println!(
                    "{}",
                    "🔧 Proactive Tool Mode: ON - More likely to use tools for system queries".cyan()
                );
            } else {
                println!(
                    "{}",
                    "💬 Proactive Tool Mode: OFF - Conservative tool usage".dimmed()
                );
            }
        }
        println!();
    }

    fn show_help(&self) {
        println!("{}", "Available Commands and Examples:".cyan().bold());
        println!();

        println!("{}", "Model Configuration:".blue().bold());
        println!("  {} {}", "•".blue(), "set temperature to 0.8");
        println!("  {} {}", "•".blue(), "show model config");
        println!("  {} {}", "•".blue(), "switch to codellama");
        println!("  {} {}", "•".blue(), "toggle tool mode");
        println!();

        println!("{}", "Git Operations:".blue().bold());
        println!("  {} {}", "•".blue(), "git status");
        println!(
            "  {} {}",
            "•".blue(),
            "commit changes with message 'fix bug'"
        );
        println!("  {} {}", "•".blue(), "push to origin");
        println!("  {} {}", "•".blue(), "show git log");
        println!();

        println!("{}", "File Operations:".blue().bold());
        println!("  {} {}", "•".blue(), "read Cargo.toml");
        println!("  {} {}", "•".blue(), "search for *.rs files");
        println!("  {} {}", "•".blue(), "watch config.json for changes");
        println!("  {} {}", "•".blue(), "find TODO comments in src/");
        println!();

        println!("{}", "System Operations:".blue().bold());
        println!("  {} {}", "•".blue(), "system info");
        println!("  {} {}", "•".blue(), "memory usage");
        println!("  {} {}", "•".blue(), "disk space");
        println!("  {} {}", "•".blue(), "list processes");
        println!();

        println!("{}", "Docker Operations:".blue().bold());
        println!("  {} {}", "•".blue(), "list containers");
        println!("  {} {}", "•".blue(), "run nginx container with port 80:80");
        println!("  {} {}", "•".blue(), "stop container myapp");
        println!("  {} {}", "•".blue(), "show logs for webapp");
        println!();

        println!("{}", "Package Management:".blue().bold());
        println!("  {} {}", "•".blue(), "cargo build");
        println!("  {} {}", "•".blue(), "add serde dependency");
        println!("  {} {}", "•".blue(), "npm install");
        println!("  {} {}", "•".blue(), "pip list packages");
        println!();

        println!("{}", "Web & API:".blue().bold());
        println!("  {} {}", "•".blue(), "search for rust tutorials");
        println!("  {} {}", "•".blue(), "scrape https://example.com");
        println!("  {} {}", "•".blue(), "GET request to api.example.com");
        println!();

        println!("{}", "Text Processing:".blue().bold());
        println!("  {} {}", "•".blue(), "format this json data");
        println!("  {} {}", "•".blue(), "count words in text");
        println!("  {} {}", "•".blue(), "find emails in document");
        println!();

        println!(
            "{}",
            "And much more! Just describe what you want to do naturally.".green()
        );
        println!();
    }

    fn show_farewell(&self) {
        println!();
        println!("{}", "Session Summary:".cyan().bold());
        println!(
            "  Commands processed: {}",
            self.session_stats.commands_processed
        );
        println!("  Tools executed: {}", self.session_stats.tools_executed);
        println!(
            "  Success rate: {:.1}%",
            if self.session_stats.commands_processed > 0 {
                (self.session_stats.successful_operations as f64
                    / self.session_stats.commands_processed as f64)
                    * 100.0
            } else {
                0.0
            }
        );
        println!();
        println!("{} {}", "Goodbye!".cyan().bold(), "👋".cyan());
    }

    async fn process_request(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        self.session_stats.commands_processed += 1;

        // Create context-aware prompt
        let context_prompt = self.create_context_aware_prompt(user_input);

        // Use LLM to analyze and determine the best response approach
        println!("{} Analyzing request with AI...", "🧠".cyan());
        let response_decision = self
            .analyze_request_with_llm(&context_prompt, user_input)
            .await?;

        match response_decision {
            ResponseMode::CommandGeneration => {
                self.handle_command_generation_request(user_input).await?;
            }
            ResponseMode::ToolExecution(tools) => {
                self.handle_tool_request(&context_prompt, tools).await?;
            }
            ResponseMode::GeneralConversation => {
                self.handle_general_conversation(&context_prompt).await?;
            }
        }

        let duration = start_time.elapsed();
        self.session_stats.total_response_time += duration.as_secs_f64();
        self.session_stats.successful_operations += 1;

        println!(
            "{} Completed in {:.2}s",
            "⏱".dimmed(),
            duration.as_secs_f64()
        );

        Ok(())
    }

    async fn handle_general_conversation(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = self.build_conversation_context(user_input).await;
        let response = stream_response(&self.model, &context).await?;

        // Create conversation entry
        let entry = ConversationEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            user_input: user_input.to_string(),
            assistant_response: response,
            tools_used: vec![],
            metadata: None,
        };

        self.conversation_history.push(entry);

        // Implement proper memory management for conversation history
        self.manage_conversation_history_memory();

        Ok(())
    }

    async fn handle_tool_request(
        &mut self,
        user_input: &str,
        tools: Vec<AvailableTool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Executing {} tool(s)", "🔧".cyan(), tools.len());

        let mut tool_results = Vec::new();
        let mut tools_used = Vec::new();

        for (i, tool) in tools.iter().enumerate() {
            println!();
            println!("{} Tool {} of {}", "📝".blue(), i + 1, tools.len());

            if !self.permission_manager.request_permission(tool)? {
                println!("{} Skipping tool execution", "⏭".yellow());
                continue;
            }

            println!();
            match self.tool_executor.execute_tool(tool.clone()).await {
                Ok(result) => {
                    self.session_stats.tools_executed += 1;
                    tools_used.push(format!("{:?}", tool));

                    if result.success {
                        println!("{} Tool executed successfully", "✅".green());
                        if !result.output.is_empty() {
                            println!();
                            println!("{}", "📄 Output:".blue().bold());
                            self.display_tool_output(&result.output);
                        }
                        tool_results.push(result);
                    } else {
                        println!("{} Tool execution failed", "❌".red());
                        if let Some(error) = &result.error {
                            println!("{} {}", "Error:".red(), error);
                        }
                    }
                }
                Err(e) => {
                    println!("{} Tool execution error: {}", "❌".red(), e);
                }
            }
        }

        if !tool_results.is_empty() {
            println!();
            println!("{}", "🤖 Assistant Summary:".cyan().bold());
            let context = self.build_tool_context(user_input, &tool_results);
            let response = stream_response(&self.model, &context).await?;

            // Create conversation entry
            let entry = ConversationEntry {
                timestamp: chrono::Utc::now().to_rfc3339(),
                user_input: user_input.to_string(),
                assistant_response: response,
                tools_used,
                metadata: Some(serde_json::json!({
                    "tools_count": tool_results.len(),
                    "successful_tools": tool_results.iter().filter(|r| r.success).count()
                })),
            };

            self.conversation_history.push(entry);
            
            // Implement proper memory management for conversation history
            self.manage_conversation_history_memory();
        }

        Ok(())
    }

    fn display_tool_output(&self, output: &str) {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() > 30 {
            // Show first 15 and last 15 lines for very long output
            for line in lines.iter().take(15) {
                println!("  {}", line);
            }
            println!(
                "  {} ... ({} lines omitted) ...",
                "⋮".dimmed(),
                lines.len() - 30
            );
            for line in lines.iter().skip(lines.len() - 15) {
                println!("  {}", line);
            }
        } else {
            // Show all lines for shorter output
            for line in lines {
                println!("  {}", line);
            }
        }
    }

    async fn build_conversation_context(&self, user_input: &str) -> String {
        let mut context = String::new();

        // Add system prompt from config if available
        if let Ok(system_prompt) = self.tool_executor.get_system_prompt().await {
            if let Some(prompt) = system_prompt {
                context.push_str(&format!("System: {}\n\n", prompt));
            }
        }

        // Add model configuration context as fallback
        let model_config = crate::tools::model_config::get_current_model_config();
        if !model_config.system_prompt.is_empty()
            && model_config.system_prompt != "You are a helpful AI assistant."
        {
            context.push_str(&format!("System: {}\n\n", model_config.system_prompt));
        }

        // Add recent conversation history
        if !self.conversation_history.is_empty() {
            context.push_str("Recent conversation:\n");
            for entry in self.conversation_history.iter().rev().take(3).rev() {
                context.push_str(&format!(
                    "User: {}\nAssistant: {}\n\n",
                    entry.user_input,
                    if entry.assistant_response.len() > 200 {
                        format!("{}...", &entry.assistant_response[..200])
                    } else {
                        entry.assistant_response.clone()
                    }
                ));
            }
        }

        context.push_str(&format!("User: {}\n", user_input));
        context.push_str("Assistant: ");

        context
    }

    fn build_tool_context(
        &self,
        user_input: &str,
        results: &[crate::tools::core::ToolResult],
    ) -> String {
        let mut context = format!("User requested: {}\n\n", user_input);

        context.push_str("Tool execution results:\n");
        for (i, result) in results.iter().enumerate() {
            let preview = if result.output.len() > 800 {
                format!("{}...", &result.output[..800])
            } else {
                result.output.clone()
            };

            context.push_str(&format!(
                "Tool {}: {} (Success: {})\n{}\n\n",
                i + 1,
                if let Some(metadata) = &result.metadata {
                    metadata
                        .get("operation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                } else {
                    "Tool"
                },
                result.success,
                preview
            ));
        }

        context.push_str(
            "\nPlease provide a helpful summary and analysis of these results for the user. ",
        );
        context.push_str("Focus on the key findings and actionable insights.\nAssistant: ");
        context
    }

    fn get_user_input(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = if self.model.is_code_model() {
            "🤖 [Code Model] How can I help you?"
        } else {
            "🤖 How can I help you?"
        };

        let input = self.vim_handler.get_input(prompt)?;
        Ok(input)
    }

    fn is_exit_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(lower.as_str(), "quit" | "exit" | "bye" | "goodbye" | "q")
    }

    fn is_stats_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(lower.as_str(), "stats" | "statistics" | "session stats")
    }

    fn is_help_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(
            lower.as_str(),
            "help" | "?" | "commands" | "what can you do"
        )
    }

    fn is_switch_model_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        lower.starts_with("switch to")
            || lower.starts_with("use model")
            || lower.starts_with("change model")
    }

    fn is_performance_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(
            lower.as_str(),
            "performance" | "perf" | "metrics" | "show performance"
        )
    }

    fn is_resource_usage_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(
            lower.as_str(),
            "resources" | "resource usage" | "usage" | "show resources"
        )
    }

    fn is_clear_logs_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(
            lower.as_str(),
            "clear logs" | "clear log" | "reset logs" | "clear metrics"
        )
    }

    fn is_toggle_tool_mode_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(
            lower.as_str(),
            "toggle tool mode" | "toggle tools" | "tool mode" | "proactive mode"
        )
    }

    async fn handle_model_switch(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let lower = input.trim().to_lowercase();

        let model_name = if let Some(name) = lower.strip_prefix("switch to ") {
            name.trim()
        } else if let Some(name) = lower.strip_prefix("use model ") {
            name.trim()
        } else if let Some(name) = lower.strip_prefix("change model to ") {
            name.trim()
        } else {
            return Err("Invalid model switch command".into());
        };

        // Get available models
        let available_models = crate::client::fetch_models().await?;

        // Find the model (support partial matching)
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(model_name))
            .collect();

        match matching_models.len() {
            0 => {
                let available_names: Vec<String> =
                    available_models.iter().map(|m| m.name.clone()).collect();
                println!(
                    "{} Model '{}' not found. Available models:",
                    "❌".red(),
                    model_name
                );
                for name in available_names {
                    println!("  • {}", name.yellow());
                }
            }
            1 => {
                let model = matching_models[0];
                let old_model = self.model.name.clone();

                // Don't switch if it's the same model
                if old_model == model.name {
                    println!("{} Already using model '{}'", "ℹ️".blue(), model.name);
                    return Ok(());
                }

                // Update the global model config first
                let result = self.tool_executor.switch_model(&model.name).await?;

                if result.success {
                    // Update the session's model
                    self.model = crate::client::SelectedModel::from(model.clone());

                    println!(
                        "{} Successfully switched from '{}' to '{}'",
                        "✅".green(),
                        old_model,
                        model.name
                    );

                    // Show brief model info
                    println!("{} Model ready for your next request", "🤖".cyan());
                } else {
                    return Err(result
                        .error
                        .unwrap_or("Unknown error switching model".to_string())
                        .into());
                }
            }
            _ => {
                println!(
                    "{} Multiple models match '{}'. Please be more specific:",
                    "⚠️".yellow(),
                    model_name
                );
                for model in matching_models {
                    println!("  • {}", model.name.yellow());
                }
            }
        }

        Ok(())
    }

    async fn handle_toggle_tool_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_mode = self.tool_executor.is_proactive_tool_mode_enabled().await?;
        let new_mode = !current_mode;
        
        self.tool_executor.set_config(
            "enable_proactive_tool_mode", 
            serde_json::Value::Bool(new_mode)
        ).await?;
        
        let mode_text = if new_mode {
            "🔧 Proactive Tool Mode: ON".cyan()
        } else {
            "💬 Proactive Tool Mode: OFF".dimmed()
        };
        
        println!("{} Tool mode toggled: {}", "⚙️".cyan(), mode_text);
        
        if new_mode {
            println!("{}", "   → More likely to use tools for system queries, file operations, and information gathering".dimmed());
        } else {
            println!("{}", "   → Conservative tool usage, prefer conversational responses".dimmed());
        }
        
        Ok(())
    }

    fn show_session_stats(&self) {
        println!();
        println!("{}", "Session Statistics:".cyan().bold());
        println!(
            "  Commands processed: {}",
            self.session_stats.commands_processed.to_string().yellow()
        );
        println!(
            "  Tools executed: {}",
            self.session_stats.tools_executed.to_string().yellow()
        );
        println!(
            "  Successful operations: {}",
            self.session_stats.successful_operations.to_string().green()
        );
        println!(
            "  Failed operations: {}",
            self.session_stats.failed_operations.to_string().red()
        );

        if self.session_stats.commands_processed > 0 {
            let avg_response_time = self.session_stats.total_response_time
                / self.session_stats.commands_processed as f64;
            println!(
                "  Average response time: {:.2}s",
                avg_response_time.to_string().blue()
            );

            let success_rate = (self.session_stats.successful_operations as f64
                / self.session_stats.commands_processed as f64)
                * 100.0;
            println!("  Success rate: {:.1}%", success_rate.to_string().green());
        }

        println!(
            "  Conversation entries: {}",
            self.conversation_history.len().to_string().blue()
        );

        // Model info
        println!();
        println!("{}", "Current Model:".cyan().bold());
        println!("  Name: {}", self.model.get_name().yellow());
        println!("  Size: {:.1} GB", self.model.size_gb.to_string().blue());
        println!(
            "  Type: {}",
            if self.model.is_code_model() {
                "Code Model".green()
            } else {
                "Chat Model".blue()
            }
        );

        // Tool executor stats
        println!();
        println!("{}", "Tool Configuration:".cyan().bold());
        println!(
            "  Auto-approve safe operations: {}",
            if self.tool_executor.config.auto_approve_safe {
                "Yes".green()
            } else {
                "No".red()
            }
        );
        println!(
            "  Max file size: {} MB",
            (self.tool_executor.config.max_file_size / 1024 / 1024)
                .to_string()
                .blue()
        );
        println!(
            "  Default timeout: {}s",
            self.tool_executor.config.default_timeout.to_string().blue()
        );
        println!();
    }

    // Export conversation to different formats
    pub async fn export_conversation(
        &self,
        format: crate::tools::ExportFormat,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tool_executor.export_conversation(format, path).await?;
        Ok(())
    }

    // Get conversation history
    pub fn get_conversation_history(&self) -> &[ConversationEntry] {
        &self.conversation_history
    }

    // Clear conversation history
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
        println!("{} Conversation history cleared", "🧹".cyan());
    }

    // Update model configuration
    pub async fn update_model_config(
        &self,
        parameter: crate::tools::ModelParameter,
        value: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tool_executor
            .set_model_parameter(parameter, value)
            .await?;
        Ok(())
    }

    // Add workspace context to session
    pub fn add_workspace_context(
        &mut self,
        context: &WorkspaceContext,
        files: HashMap<PathBuf, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.workspace_context = Some(context.clone());
        self.workspace_files = files;

        println!(
            "{} Added workspace context: {} files",
            "📁".cyan(),
            self.workspace_files.len()
        );
        Ok(())
    }

    // Get workspace context
    pub fn get_workspace_context(&self) -> Option<&WorkspaceContext> {
        self.workspace_context.as_ref()
    }

    // Get workspace files
    pub fn get_workspace_files(&self) -> &HashMap<PathBuf, String> {
        &self.workspace_files
    }

    // Analyze request with LLM to determine response mode
    async fn analyze_request_with_llm(
        &self,
        context_prompt: &str,
        user_input: &str,
    ) -> Result<ResponseMode, Box<dyn std::error::Error>> {
        let is_proactive_tool_mode_enabled = self
            .tool_executor
            .is_proactive_tool_mode_enabled()
            .await
            .unwrap_or(false);

        let analysis_prompt = if is_proactive_tool_mode_enabled {
            format!(
                r#"You are an AI assistant that helps users with various tasks. Analyze the following user request and determine the most appropriate response mode.

User request: "{}"

Context: {}

Response modes:
1. COMMAND_GENERATION: Generate a specific command/script to be executed
2. TOOL_EXECUTION: Use available tools to perform actions (file operations, searches, system information, etc.)
3. GENERAL_CONVERSATION: Provide conversational response with explanations, tutorials, or information

PROACTIVE TOOL MODE is ENABLED. Guidelines:
- STRONGLY PREFER TOOL_EXECUTION for any request that could potentially benefit from system tools
- Use TOOL_EXECUTION for: system checks, file operations, searches, information gathering, status checks, package management, git operations, etc.
- Examples that should use TOOL_EXECUTION:
  * "check if hyprland is on my system" → TOOL_EXECUTION (check installed packages)
  * "what's my system info" → TOOL_EXECUTION (system information)
  * "list my files" → TOOL_EXECUTION (file listing)
  * "check git status" → TOOL_EXECUTION (git operations)
  * "is docker running" → TOOL_EXECUTION (system processes)
  * "search for X in my files" → TOOL_EXECUTION (file search)
- Only use GENERAL_CONVERSATION for pure explanations, tutorials, or when no tools could reasonably help
- Only use COMMAND_GENERATION if the user explicitly asks to "run", "execute", "generate command", or similar action-oriented requests

Respond with only one of: COMMAND_GENERATION, TOOL_EXECUTION, or GENERAL_CONVERSATION"#,
                user_input, context_prompt
            )
        } else {
            format!(
                r#"You are an AI assistant that helps users with various tasks. Analyze the following user request and determine the most appropriate response mode.

User request: "{}"

Context: {}

Response modes:
1. COMMAND_GENERATION: Generate a specific command/script to be executed (e.g., "how do I create a next.js app?" should get a conversational explanation, not a command)
2. TOOL_EXECUTION: Use available tools to perform actions (file operations, searches, etc.)
3. GENERAL_CONVERSATION: Provide conversational response with explanations, tutorials, or information

Guidelines:
- Only use COMMAND_GENERATION if the user explicitly asks to "run", "execute", "generate command", or similar action-oriented requests
- Questions starting with "how do I", "what is", "can you explain" should typically be GENERAL_CONVERSATION
- Use TOOL_EXECUTION for file operations, searches, system information, etc.
- When in doubt, prefer GENERAL_CONVERSATION for a better user experience

Respond with only one of: COMMAND_GENERATION, TOOL_EXECUTION, or GENERAL_CONVERSATION"#,
                user_input, context_prompt
            )
        };

        let response = generate_response_silent(&self.model, &analysis_prompt).await?;
        let decision = response.trim().to_uppercase();

        let is_command_generation_enabled = self
            .tool_executor
            .is_command_generation_enabled()
            .await
            .unwrap_or(false);

        match decision.as_str() {
            "COMMAND_GENERATION" if is_command_generation_enabled => {
                Ok(ResponseMode::CommandGeneration)
            }
            "TOOL_EXECUTION" => {
                // Use existing tool parsing logic
                let tools = self
                    .parser
                    .parse_request_with_llm(context_prompt, &self.model)
                    .await;
                if tools.is_empty() {
                    Ok(ResponseMode::GeneralConversation)
                } else {
                    Ok(ResponseMode::ToolExecution(tools))
                }
            }
            _ => Ok(ResponseMode::GeneralConversation),
        }
    }

    // Create context-aware prompt

    async fn handle_command_generation_request(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Generating command for your request...", "🤖".cyan());

        // Get workspace context as additional context
        let context = if let Some(workspace_context) = &self.workspace_context {
            Some(format!(
                "Project: {} ({})",
                workspace_context.root_path.display(),
                workspace_context
                    .project_type
                    .as_deref()
                    .unwrap_or("unknown")
            ))
        } else {
            None
        };

        // Generate command using the tool executor
        let generation_result = self
            .tool_executor
            .generate_command(user_input, context.as_deref())
            .await?;

        if !generation_result.success {
            println!(
                "{} Failed to generate command: {}",
                "❌".red(),
                generation_result
                    .error
                    .unwrap_or("Unknown error".to_string())
            );
            return Ok(());
        }

        // Use the generated prompt to get a command from the LLM
        let command_prompt = generation_result.output;
        let generated_command = stream_response(&self.model, &command_prompt).await?;

        // Clean up the response to get just the command
        let clean_command = generated_command
            .trim()
            .lines()
            .find(|line| !line.trim().is_empty() && !line.starts_with("Command:"))
            .unwrap_or(generated_command.trim())
            .trim();

        println!(
            "{} Generated command: {}",
            "💡".yellow(),
            clean_command.cyan()
        );

        // Ask user if they want to execute it
        use dialoguer::{theme::ColorfulTheme, Confirm};
        let should_execute = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("{} Execute this command?", "🚀".green()))
            .default(false)
            .interact()?;

        if should_execute {
            // Execute the command
            let tool = AvailableTool::ExecuteCommand {
                command: clean_command.to_string(),
            };

            if self.permission_manager.request_permission(&tool)? {
                println!("{} Executing command...", "⚡".cyan());
                let result = self.tool_executor.execute_tool(tool).await?;

                if result.success {
                    println!("{} Command executed successfully!", "✅".green());
                    if !result.output.is_empty() {
                        println!("{}", result.output);
                    }
                } else {
                    println!(
                        "{} Command failed: {}",
                        "❌".red(),
                        result.error.unwrap_or("Unknown error".to_string())
                    );
                }
            } else {
                println!("{} Command execution denied", "🚫".red());
            }
        } else {
            println!("{} Command execution cancelled", "🚫".yellow());
        }

        // Create conversation entry
        let entry = ConversationEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            user_input: user_input.to_string(),
            assistant_response: format!("Generated command: {}", clean_command),
            tools_used: vec!["command_generation".to_string()],
            metadata: Some(serde_json::json!({
                "generated_command": clean_command,
                "executed": should_execute
            })),
        };

        self.conversation_history.push(entry);
        
        // Implement proper memory management for conversation history
        self.manage_conversation_history_memory();

        Ok(())
    }

    fn manage_conversation_history_memory(&mut self) {
        const MAX_HISTORY_SIZE: usize = 50;
        const TRIM_TO_SIZE: usize = 30;
        const MAX_RESPONSE_LENGTH: usize = 1000;
        
        // Check if we need to trim history
        if self.conversation_history.len() > MAX_HISTORY_SIZE {
            // Remove oldest entries, keeping the most recent ones
            let remove_count = self.conversation_history.len() - TRIM_TO_SIZE;
            self.conversation_history.drain(0..remove_count);
        }
        
        // Trim excessively long responses to save memory
        for entry in &mut self.conversation_history {
            if entry.assistant_response.len() > MAX_RESPONSE_LENGTH {
                entry.assistant_response.truncate(MAX_RESPONSE_LENGTH);
                entry.assistant_response.push_str("... [truncated]");
            }
        }
        
        // Calculate approximate memory usage
        let total_memory: usize = self.conversation_history.iter()
            .map(|entry| {
                entry.user_input.len() + 
                entry.assistant_response.len() + 
                entry.tools_used.iter().map(|tool| tool.len()).sum::<usize>()
            })
            .sum();
        
        // If memory usage is still too high, be more aggressive
        const MAX_MEMORY_BYTES: usize = 100_000; // ~100KB
        if total_memory > MAX_MEMORY_BYTES {
            let target_size = std::cmp::max(TRIM_TO_SIZE / 2, 10);
            if self.conversation_history.len() > target_size {
                let remove_count = self.conversation_history.len() - target_size;
                self.conversation_history.drain(0..remove_count);
            }
        }
    }

    pub fn create_context_aware_prompt(&self, user_input: &str) -> String {
        let mut prompt = String::new();

        // Add workspace context if available
        if let Some(context) = &self.workspace_context {
            prompt.push_str(&format!("Project context:\n"));
            prompt.push_str(&format!("- Root path: {}\n", context.root_path.display()));
            if let Some(project_type) = &context.project_type {
                prompt.push_str(&format!("- Project type: {}\n", project_type));
            }
            prompt.push_str(&format!(
                "- Files in context: {}\n\n",
                context.included_files.len()
            ));

            // Add file contents if not too many
            if self.workspace_files.len() <= 10 {
                prompt.push_str("Relevant files:\n");
                for (path, content) in &self.workspace_files {
                    prompt.push_str(&format!("\n## {}\n```\n{}\n```\n", path.display(), content));
                }
            } else {
                prompt.push_str(&format!(
                    "Note: {} files are available in the workspace context.\n",
                    self.workspace_files.len()
                ));
            }

            prompt.push_str("\n---\n\n");
        }

        // Add user input
        prompt.push_str(&format!("User request: {}", user_input));

        prompt
    }
}
