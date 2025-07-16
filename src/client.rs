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
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelDetails {
    pub format: Option<String>,
    pub family: Option<String>,
    pub families: Option<Vec<String>>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SelectedModel {
    pub name: String,
    pub size_gb: f64,
    pub digest: String,
    pub modified_at: String,
    pub details: Option<ModelDetails>,
}

impl From<Model> for SelectedModel {
    fn from(model: Model) -> Self {
        SelectedModel {
            name: model.name,
            size_gb: model.size as f64 / 1_000_000_000.0,
            digest: model.digest,
            modified_at: model.modified_at,
            details: model.details,
        }
    }
}

impl SelectedModel {
    pub fn display_info(&self) {
        println!("{}", "Selected Model:".cyan().bold());
        println!("  {} {}", "Name:".blue(), self.name.white().bold());
        println!("  {} {:.2} GB", "Size:".blue(), self.size_gb);

        if let Some(details) = &self.details {
            if let Some(family) = &details.family {
                println!("  {} {}", "Family:".blue(), family.yellow());
            }
            if let Some(param_size) = &details.parameter_size {
                println!("  {} {}", "Parameters:".blue(), param_size.yellow());
            }
            if let Some(format) = &details.format {
                println!("  {} {}", "Format:".blue(), format.yellow());
            }
        }

        println!("  {} {}", "Modified:".blue(), self.modified_at.dimmed());
        println!();
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_code_model(&self) -> bool {
        self.name.to_lowercase().contains("code")
            || self.name.to_lowercase().contains("codellama")
            || self.name.to_lowercase().contains("starcoder")
    }

    pub fn is_chat_model(&self) -> bool {
        self.name.to_lowercase().contains("chat")
            || self.name.to_lowercase().contains("instruct")
            || !self.is_code_model()
    }
}

#[derive(Serialize, Debug)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
}

#[derive(Serialize, Debug)]
pub struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>, // max_tokens in Ollama
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>, // context_length in Ollama
}

#[derive(Deserialize, Debug)]
pub struct OllamaResponse {
    pub response: Option<String>,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub prompt_eval_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u32>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
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
            let size_gb = model.size as f64 / 1_000_000_000.0;
            let model_type = if model.name.to_lowercase().contains("code") {
                "📝 Code"
            } else if model.name.to_lowercase().contains("chat")
                || model.name.to_lowercase().contains("instruct")
            {
                "💬 Chat"
            } else {
                "🤖 General"
            };

            format!("{} {} ({:.1} GB)", model_type, model.name, size_gb)
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

    // Use enhanced request with current model configuration
    let request =
        crate::tools::model_config::create_enhanced_request(model.get_name(), prompt, true);

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
    let mut stats = ResponseStats::new();

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
                        stats.tokens_generated += 1;
                    }

                    if ollama_response.done {
                        // Extract performance statistics
                        if let Some(total_duration) = ollama_response.total_duration {
                            stats.total_duration_ns = total_duration;
                        }
                        if let Some(eval_count) = ollama_response.eval_count {
                            stats.eval_count = eval_count;
                        }
                        if let Some(eval_duration) = ollama_response.eval_duration {
                            stats.eval_duration_ns = eval_duration;
                        }

                        // Print performance stats
                        println!(); // New line after response
                        stats.print_stats();
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Ok(full_response)
}

// Non-streaming version for tool usage
pub async fn generate_response(
    model: &SelectedModel,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    let request = crate::tools::model_config::create_enhanced_request(
        model.get_name(),
        prompt,
        false, // Non-streaming
    );

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API request failed: {}", response.status()).into());
    }

    let ollama_response: OllamaResponse = response.json().await?;
    Ok(ollama_response.response.unwrap_or_default())
}

#[derive(Debug)]
struct ResponseStats {
    tokens_generated: u32,
    total_duration_ns: u64,
    eval_count: u32,
    eval_duration_ns: u64,
}

impl ResponseStats {
    fn new() -> Self {
        Self {
            tokens_generated: 0,
            total_duration_ns: 0,
            eval_count: 0,
            eval_duration_ns: 0,
        }
    }

    fn print_stats(&self) {
        if self.total_duration_ns > 0 {
            let total_seconds = self.total_duration_ns as f64 / 1_000_000_000.0;
            let tokens_per_second = if total_seconds > 0.0 {
                self.eval_count as f64 / total_seconds
            } else {
                0.0
            };

            println!();
            println!("{}", "Performance Stats:".dimmed());
            println!("  {} {:.2}s", "Total time:".dimmed(), total_seconds);
            println!("  {} {}", "Tokens generated:".dimmed(), self.eval_count);
            if tokens_per_second > 0.0 {
                println!("  {} {:.1} tokens/s", "Speed:".dimmed(), tokens_per_second);
            }
        }
    }
}

// Model management functions
pub async fn pull_model(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Pulling model: {}", "⬇️".cyan(), model_name.yellow());

    let client = Client::new();
    let request = serde_json::json!({
        "name": model_name
    });

    let response = client
        .post("http://localhost:11434/api/pull")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to pull model: {}", response.status()).into());
    }

    // Handle streaming pull response
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = FuturesStreamExt::next(&mut stream).await {
        let chunk = chunk_result?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(status) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(status_msg) = status.get("status").and_then(|s| s.as_str()) {
                    println!("  {}", status_msg.blue());
                }
            }
        }
    }

    println!("{} Model pulled successfully", "✅".green());
    Ok(())
}

pub async fn delete_model(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Deleting model: {}", "🗑️".cyan(), model_name.yellow());

    let client = Client::new();
    let request = serde_json::json!({
        "name": model_name
    });

    let response = client
        .delete("http://localhost:11434/api/delete")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        println!("{} Model deleted successfully", "✅".green());
        Ok(())
    } else {
        Err(format!("Failed to delete model: {}", response.status()).into())
    }
}

pub async fn show_model_info(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{} Getting model info: {}",
        "ℹ️".cyan(),
        model_name.yellow()
    );

    let client = Client::new();
    let request = serde_json::json!({
        "name": model_name
    });

    let response = client
        .post("http://localhost:11434/api/show")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get model info: {}", response.status()).into());
    }

    let info: serde_json::Value = response.json().await?;

    println!("{}", "Model Information:".cyan().bold());
    if let Some(modelfile) = info.get("modelfile").and_then(|v| v.as_str()) {
        println!(
            "  {} {}",
            "Modelfile:".blue(),
            modelfile.lines().next().unwrap_or("N/A")
        );
    }
    if let Some(parameters) = info.get("parameters").and_then(|v| v.as_str()) {
        println!("  {} {}", "Parameters:".blue(), parameters);
    }
    if let Some(template) = info.get("template").and_then(|v| v.as_str()) {
        println!(
            "  {} {}",
            "Template:".blue(),
            if template.len() > 100 {
                format!("{}...", &template[..100])
            } else {
                template.to_string()
            }
        );
    }
    if let Some(details) = info.get("details") {
        println!("  {} {}", "Details:".blue(), details);
    }

    Ok(())
}

// Health check function
pub async fn check_ollama_health() -> Result<bool, Box<dyn std::error::Error>> {
    let client = Client::new();

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

// List available models with filtering
pub async fn list_models_filtered(
    filter: Option<&str>,
) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let all_models = fetch_models().await?;

    if let Some(filter_term) = filter {
        let filtered = all_models
            .into_iter()
            .filter(|model| {
                model
                    .name
                    .to_lowercase()
                    .contains(&filter_term.to_lowercase())
                    || model
                        .details
                        .as_ref()
                        .and_then(|d| d.family.as_ref())
                        .map(|f| f.to_lowercase().contains(&filter_term.to_lowercase()))
                        .unwrap_or(false)
            })
            .collect();
        Ok(filtered)
    } else {
        Ok(all_models)
    }
}
