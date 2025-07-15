use colored::Colorize;
use dialoguer::Input;
use std::time::Instant;

use crate::client::{stream_response, SelectedModel};
use crate::tools::{
    AvailableTool, NaturalLanguageParser, PermissionManager, ToolExecutor, ToolResult,
};

pub struct AssistantSession {
    model: SelectedModel,
    tool_executor: ToolExecutor,
    permission_manager: PermissionManager,
    parser: NaturalLanguageParser,
    conversation_history: Vec<(String, String)>,
}

impl AssistantSession {
    pub fn new(model: SelectedModel) -> Self {
        Self {
            model,
            tool_executor: ToolExecutor::new(),
            permission_manager: PermissionManager::new(),
            parser: NaturalLanguageParser::new(),
            conversation_history: Vec::new(),
        }
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
                println!("{}", "Goodbye! 👋".cyan());
                break;
            }

            if let Err(e) = self.process_request(&user_input).await {
                println!("{} {}", "Error processing request:".red(), e);
            }

            println!();
        }

        Ok(())
    }

    fn show_welcome(&self) {
        println!("{}", "🤖 AI Assistant with System Tools".cyan().bold());
        println!("Model: {}", self.model.get_name().yellow());
        println!();
        println!("{}", "I can help you with:".blue());
        println!("  {} Web searches and scraping", "•".blue());
        println!("  {} File operations (read, write, search)", "•".blue());
        println!("  {} Project creation (Rust, Python, JS)", "•".blue());
        println!("  {} System commands (with permission)", "•".blue());
        println!("  {} And anything else you can think of!", "•".blue());
        println!();
        println!(
            "{}",
            "Now using LLM-powered tool selection for better accuracy!".green()
        );
        println!("{}", "Type 'quit' or 'exit' to end the session".dimmed());
        println!();
    }

    async fn process_request(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();

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
        println!(
            "{} Completed in {:.2}s",
            "⏱".dimmed(),
            duration.as_secs_f64()
        );

        Ok(())
    }

    // ... rest of the methods remain the same as before

    async fn handle_general_conversation(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = self.build_conversation_context(user_input);
        let response = stream_response(&self.model, &context).await?;

        self.conversation_history
            .push((user_input.to_string(), response));

        Ok(())
    }

    async fn handle_tool_request(
        &mut self,
        user_input: &str,
        tools: Vec<AvailableTool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Executing {} tool(s)", "🔧".cyan(), tools.len());

        let mut tool_results = Vec::new();

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
            self.conversation_history
                .push((user_input.to_string(), response));
        }

        Ok(())
    }

    fn display_tool_output(&self, output: &str) {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() > 20 {
            for line in lines.iter().take(10) {
                println!("  {}", line);
            }
            println!(
                "  {} ... ({} lines omitted) ...",
                "⋮".dimmed(),
                lines.len() - 20
            );
            for line in lines.iter().skip(lines.len() - 10) {
                println!("  {}", line);
            }
        } else {
            for line in lines {
                println!("  {}", line);
            }
        }
    }

    fn build_conversation_context(&self, user_input: &str) -> String {
        let mut context = String::new();

        if !self.conversation_history.is_empty() {
            context.push_str("Recent conversation:\n");
            for (user, assistant) in self.conversation_history.iter().rev().take(3).rev() {
                context.push_str(&format!("User: {}\nAssistant: {}\n\n", user, assistant));
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
            context.push_str(&format!(
                "Tool {}: {}\n",
                i + 1,
                if result.output.len() > 500 {
                    format!("{}...", &result.output[..500])
                } else {
                    result.output.clone()
                }
            ));
        }

        context.push_str("\nPlease provide a helpful summary and analysis of these results for the user.\nAssistant: ");
        context
    }

    fn get_user_input(&self) -> Result<String, Box<dyn std::error::Error>> {
        let input: String = Input::new()
            .with_prompt("🤖 How can I help you?")
            .interact_text()?;

        Ok(input)
    }

    fn is_exit_command(&self, input: &str) -> bool {
        let lower = input.trim().to_lowercase();
        matches!(lower.as_str(), "quit" | "exit" | "bye" | "goodbye")
    }
}
