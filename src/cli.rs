use std::{
    io::{self, Write},
    time::Duration,
};

use colored::Colorize;
use console::Term;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tabled::{Table, Tabled, settings::Style};
use tokio_stream::StreamExt;

use crate::api_models::{Model, ModelsResponse, OllamaRequest, OllamaResponse, SelectedModel};

#[allow(dead_code)]
#[derive(Tabled)]
pub struct ModelDisplay {
    #[tabled(rename = "Index")]
    pub index: usize,
    #[tabled(rename = "📦 Model")]
    pub name: String,
    #[tabled(rename = "💾 Size")]
    pub size: String,
    #[tabled(rename = "📅 Modified")]
    pub modified: String,
}

// Model Selection
pub async fn fetch_models() -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    spinner.set_message("Fetching models from Ollama...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let client = Client::new();

    let response = client.get("http://localhost:11434/api/tags").send().await?;

    if !response.status().is_success() {
        spinner.finish_with_message("✗ Connection failed".red().to_string());
        return Err(format!("API request failed: {}", response.status()).into());
    }

    let models_response: ModelsResponse = response.json().await?;
    spinner.finish_with_message("Models loaded succesfully".green().to_string());

    Ok(models_response.models)
}

pub fn display_models_table(models: &[Model]) {
    let model_displays: Vec<ModelDisplay> = models
        .iter()
        .enumerate()
        .map(|(index, model)| ModelDisplay {
            index: index + 1,
            name: model.name.clone(),
            size: format!("{:.2} GB", model.size as f64 / 1_000_000_000.0),
            modified: model
                .modified_at
                .split('T')
                .next()
                .unwrap_or("Unknown")
                .to_string(),
        })
        .collect();

    let mut table = Table::new(model_displays);
    table.with(Style::modern());

    println!("{}", "Available Models:".cyan().bold());
    println!("{}", table);
    println!();
}

pub fn select_model(models: &[Model]) -> Result<SelectedModel, Box<dyn std::error::Error>> {
    if models.is_empty() {
        return Err("No models available.".into());
    }

    let model_options: Vec<String> = models
        .iter()
        .map(|model| {
            format!(
                "{} ({:.2} GB)",
                model.name,
                model.size as f64 / 1_000_000_000.0
            )
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select model for your session")
        .items(&model_options)
        .default(0)
        .interact()?;

    let selected_model = SelectedModel::from(models[selection].clone());

    Ok(selected_model)
}

pub fn print_header() {
    let term = Term::stdout();
    let _ = term.clear_screen();

    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║          🦙 Ollama API               ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
}

pub fn print_separator() {
    println!("{}", "─".repeat(50).dimmed());
}

pub fn get_user_prompt() -> Result<String, Box<dyn std::error::Error>> {
    print_separator();

    let prompt: String = Input::new()
        .with_prompt("")
        .allow_empty(false)
        .interact_text()?;

    Ok(prompt)
}

pub fn print_response_header(model_name: &str) {
    println!();
    println!("{}", format!("🤖 {} Response:", model_name).cyan().bold());
    println!("{}", "─".repeat(50).blue());
}

pub fn print_response_footer(duration: Option<u64>) {
    println!();
    println!("{}", "─".repeat(50).blue());

    if let Some(duration_ns) = duration {
        let duration_secs = duration_ns as f64 / 1_000_000_000.0;
        println!("{} {:.2}s", "⏱ Generation time:".dimmed(), duration_secs);
    }

    println!("{}", "✓ Response complete".green());
}

pub async fn stream_response(
    model: &SelectedModel,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    spinner.set_message("Generating response...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let request = OllamaRequest {
        model: model.get_name().to_string(),
        prompt: prompt.to_string(),
        stream: true,
    };

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        spinner.finish_with_message("✗ Request failed".red().to_string());
        return Err(format!("API request failed: {}", response.status()).into());
    }

    spinner.finish_and_clear();
    println!();

    let mut stream = response.bytes_stream();
    let mut total_duration = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<OllamaResponse>(line) {
                Ok(ollama_response) => {
                    if let Some(token) = ollama_response.response {
                        print!("{}", token);
                        io::stdout().flush().unwrap();
                    }

                    if ollama_response.done {
                        total_duration = ollama_response.total_duration;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                }
            }
        }
    }
    println!("\n");
    if let Some(duration) = total_duration {
        println!("Generation time: {:.2}s", duration as f64 / 1_000_000_000.0);
    }

    Ok(())
}
