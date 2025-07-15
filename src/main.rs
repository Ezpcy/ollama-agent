use colored::Colorize;

use crate::{
    client::{fetch_models, select_model},
    session::AssistantSession,
};

pub mod api_models;
pub mod cli;
pub mod client;
pub mod session;
pub mod tools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🚀 AI Assistant Startup".cyan().bold());
    println!();

    // Fetch and select model
    let models = fetch_models().await.map_err(|e| {
        println!("{} Failed to connect to Ollama: {}", "❌".red(), e);
        println!(
            "{} Make sure Ollama is running: ollama serve",
            "💡".yellow()
        );
        e
    })?;

    if models.is_empty() {
        println!(
            "{} No models available. Install one with: ollama pull llama2",
            "⚠".yellow()
        );
        return Ok(());
    }

    let selected_model = select_model(&models)?;
    selected_model.display_info();

    // Start assistant session
    let mut session = AssistantSession::new(selected_model);
    session.run().await?;

    Ok(())
}
