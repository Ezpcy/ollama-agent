use colored::Colorize;
use dialoguer::Select;
use futures::StreamExt as FuturesStreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tokio_stream::StreamExt;

#[derive(Deserialize, Debug)]
pub struct ModelsResponse {
    pub models: Vec<Model>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Model {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

#[derive(Debug, Clone)]
pub struct SelectedModel {
    pub name: String,
    pub size_gb: f64,
    pub digest: String,
    pub modified_at: String,
}

impl From<Model> for SelectedModel {
    fn from(model: Model) -> Self {
        SelectedModel {
            name: model.name,
            size_gb: model.size as f64 / 1_000_000_000.0,
            digest: model.digest,
            modified_at: model.modified_at,
        }
    }
}

impl SelectedModel {
    pub fn display_info(&self) {
        println!("{}", "Selected Model:".cyan().bold());
        println!("  {} {}", "Name:".blue(), self.name.white().bold());
        println!("  {} {:.2} GB", "Size:".blue(), self.size_gb);
        println!();
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Serialize, Debug)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct OllamaResponse {
    pub response: Option<String>,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
}

pub async fn fetch_models() -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get("http://localhost:11434/api/tags").send().await?;

    if !response.status().is_success() {
        return Err(format!("API request failed: {}", response.status()).into());
    }

    let models_response: ModelsResponse = response.json().await?;
    Ok(models_response.models)
}

pub fn select_model(models: &[Model]) -> Result<SelectedModel, Box<dyn std::error::Error>> {
    if models.is_empty() {
        return Err("No models available".into());
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
        .with_prompt("Select a model")
        .items(&model_options)
        .default(0)
        .interact()?;

    Ok(SelectedModel::from(models[selection].clone()))
}

pub async fn stream_response(
    model: &SelectedModel,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

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
        return Err(format!("API request failed: {}", response.status()).into());
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();

    while let Some(chunk_result) = FuturesStreamExt::next(&mut stream).await {
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
                        full_response.push_str(&token);
                    }

                    if ollama_response.done {
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    println!(); // New line after response
    Ok(full_response)
}
