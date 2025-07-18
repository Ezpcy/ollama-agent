use std::time::Instant;

use colored::Colorize;

use crate::{
    api_models::SelectedModel,
    tools::{
        core::{AvailableTool, ToolExecutor, ToolResult},
        parser::NaturalLanguageParser,
        perm::PermissionManager,
    },
};

pub struct AssistantSession {
    model: SelectedModel,
    tool_executor: ToolExecutor,
    permission_manager: PermissionManager,
    parser: NaturalLanguageParser,
    conversation_history: Vec<(String, String)>, // (user_input, assistant_response)
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

            println!(); // Add spacing between interactions
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
        println!("  {} Project analysis and management (Rust, Python, JS, Go, Java, and more)", "•".blue());
        println!("  {} System commands (with permission)", "•".blue());
        println!("  {} And anything else you can think of!", "•".blue());
        println!();
        println!("{}", "Type 'quit' or 'exit' to end the session".dimmed());
        println!();
    }

    async fn process_request(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        // Parse the request to identify tools
        let tools = self.parser.parse_request(user_input);

        if tools.is_empty() {
            // No tools identified, treat as general conversation
            self.handle_general_conversation(user_input).await?;
        } else {
            // Tools identified, handle tool execution
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

    async fn handle_general_conversation(
        &mut self,
        user_input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we can suggest tool usage
        if let Some(suggestion) = self.parser.suggest_clarification(user_input) {
            println!("{} {}", "💡".yellow(), suggestion.blue());
            println!();
        }

        // Generate LLM response for general conversation
        let context = self.build_conversation_context(user_input);
        let response = self.generate_llm_response(&context).await?;

        // Store in conversation history
        self.conversation_history
            .push((user_input.to_string(), response));

        Ok(())
    }

    async fn handle_tool_request(
        &mut self,
        user_input: &str,
        tools: Vec<AvailableTool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Identified {} tool(s) for your request",
            "🔧".cyan(),
            tools.len()
        );

        let mut tool_results = Vec::new();

        for (i, tool) in tools.iter().enumerate() {
            println!();
            println!("{} Tool {} of {}", "📝".blue(), i + 1, tools.len());

            // Request permission
            if !self.permission_manager.request_permission(tool)? {
                println!("{} Skipping tool execution", "⏭".yellow());
                continue;
            }

            // Execute tool
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

        // Generate contextual response with tool results
        if !tool_results.is_empty() {
            println!();
            println!("{}", "🤖 Assistant Summary:".cyan().bold());
            let context = self.build_tool_context(user_input, &tool_results);
            let response = self.generate_llm_response(&context).await?;
            self.conversation_history
                .push((user_input.to_string(), response));
        }

        Ok(())
    }

    fn display_tool_output(&self, output: &str) {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() > 20 {
            // Show first 10 and last 10 lines for long output
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
            // Show all lines for short output
            for line in lines {
                println!("  {}", line);
            }
        }
    }

    fn build_conversation_context(&self, user_input: &str) -> String {
        let mut context = String::new();

        // Add recent conversation history
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

    async fn generate_llm_response(
        &self,
        context: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Use the existing streaming function but capture the response
        let mut response = String::new();

        // Use the streaming LLM function to generate response
        let response = stream_response_for_context(&self.model, context).await?;
        Ok(response)
    }

    fn get_user_input(&self) -> Result<String, Box<dyn std::error::Error>> {
        use dialoguer::Input;

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

// Modified streaming function to work with context
async fn stream_response_for_context(
    model: &SelectedModel,
    context: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::client::make_ollama_request;
    use serde_json::json;
    use std::io::{self, Write};
    
    println!("🤖 Generating response based on context and tool results...\n");

    let request_body = json!({
        "model": model.name,
        "prompt": context,
        "stream": true,
        "options": {
            "temperature": 0.7,
            "top_p": 0.9,
            "top_k": 40
        }
    });

    let mut response_text = String::new();
    
    match make_ollama_request("/api/generate", &request_body).await {
        Ok(mut response) => {
            let mut buffer = Vec::new();
            
            while let Some(chunk) = response.chunk().await? {
                buffer.extend_from_slice(&chunk);
                
                // Process complete lines
                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
                    let line_str = String::from_utf8_lossy(&line[..line.len()-1]); // Remove newline
                    
                    if !line_str.trim().is_empty() {
                        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(&line_str) {
                            if let Some(response_part) = json_obj["response"].as_str() {
                                print!("{}", response_part);
                                io::stdout().flush().unwrap();
                                response_text.push_str(response_part);
                            }
                            
                            if json_obj["done"].as_bool().unwrap_or(false) {
                                break;
                            }
                        }
                    }
                }
            }
            
            println!(); // Add final newline
            Ok(response_text)
        }
        Err(e) => {
            println!("Error generating response: {}", e);
            Err(e)
        }
    }
}
