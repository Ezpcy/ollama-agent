use colored::Colorize;
use std::time::Instant;

use crate::client::{stream_response, SelectedModel};
use crate::input::VimInputHandler;
use crate::tools::{
    AvailableTool, ConversationEntry, NaturalLanguageParser, PermissionManager,
    ToolExecutor, ToolResult,
};

pub struct AssistantSession {
    model: SelectedModel,
    tool_executor: ToolExecutor,
    permission_manager: PermissionManager,
    parser: NaturalLanguageParser,
    conversation_history: Vec<ConversationEntry>,
    session_stats: SessionStats,
    vim_handler: VimInputHandler,
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
        
        Self {
            model,
            tool_executor,
            permission_manager: PermissionManager::new(),
            parser: NaturalLanguageParser::new(),
            conversation_history: Vec::new(),
            session_stats: SessionStats::default(),
            vim_handler: VimInputHandler::new(),
        }
    }

    pub fn with_vim_mode(model: SelectedModel, tool_executor: ToolExecutor, vim_enabled: bool) -> Self {
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
        self.show_welcome();

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

    fn show_welcome(&self) {
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
        println!("{}", "Enhanced Features:".blue().bold());
        println!("  {} Advanced model parameter control", "•".blue());
        println!("  {} Git repository management", "•".blue());
        println!("  {} Docker container operations", "•".blue());
        println!("  {} Package management (Cargo, NPM, Pip)", "•".blue());
        println!(
            "  {} Database operations (SQLite, PostgreSQL, MySQL)",
            "•".blue()
        );
        println!("  {} System monitoring and information", "•".blue());
        println!("  {} Text processing and analysis", "•".blue());
        println!("  {} API calls and web scraping", "•".blue());
        println!("  {} File watching and automation", "•".blue());
        println!("  {} Configuration management", "•".blue());
        println!();
        println!("{}", "Special Commands:".blue().bold());
        println!("  {} Show session statistics", "stats".yellow());
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
        println!();
    }

    fn show_help(&self) {
        println!("{}", "Available Commands and Examples:".cyan().bold());
        println!();

        println!("{}", "Model Configuration:".blue().bold());
        println!("  {} {}", "•".blue(), "set temperature to 0.8");
        println!("  {} {}", "•".blue(), "show model config");
        println!("  {} {}", "•".blue(), "switch to codellama");
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

        // Use LLM to analyze and determine tools
        println!("{} Analyzing request with AI...", "🧠".cyan());
        let tools = self
            .parser
            .parse_request_with_llm(user_input, &self.model)
            .await;

        if tools.is_empty() {
            self.handle_general_conversation(user_input).await?;
        } else {
            self.handle_tool_request(user_input, tools).await?;
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
        let context = self.build_conversation_context(user_input);
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

        // Trim history if it gets too long
        if self.conversation_history.len() > 20 {
            self.conversation_history.drain(0..5);
        }

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

    fn build_conversation_context(&self, user_input: &str) -> String {
        let mut context = String::new();

        // Add model configuration context
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

    fn build_tool_context(&self, user_input: &str, results: &[ToolResult]) -> String {
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
        lower.starts_with("switch to") || lower.starts_with("use model") || lower.starts_with("change model")
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
        
        // Find the model
        if let Some(model) = available_models.iter().find(|m| m.name.to_lowercase().contains(model_name)) {
            let old_model = self.model.name.clone();
            
            // Update the global model config first
            let result = self.tool_executor.switch_model(&model.name).await?;
            
            if result.success {
                // Update the session's model
                self.model = crate::client::SelectedModel::from(model.clone());
                
                println!("{} Successfully switched from '{}' to '{}'", 
                        "✅".green(), old_model, model.name);
                self.model.display_info();
            } else {
                return Err(result.error.unwrap_or("Unknown error switching model".to_string()).into());
            }
        } else {
            let available_names: Vec<String> = available_models.iter().map(|m| m.name.clone()).collect();
            println!("{} Model '{}' not found. Available models:", "❌".red(), model_name);
            for name in available_names {
                println!("  • {}", name.yellow());
            }
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
}
