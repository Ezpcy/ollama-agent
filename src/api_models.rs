use colored::Colorize;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Serialize, Debug)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
}

// Response struct for streaming
#[derive(Deserialize, Debug)]
pub struct OllamaResponse {
    pub response: Option<String>,
    pub done: bool,
    #[serde(default)]
    pub context: Vec<i32>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Request {
    pub model: String,
    pub prompt: String,
    pub suffix: Option<String>,
    pub images: Option<String>,
    pub think: Option<String>,
    pub stream: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ModelsResponse {
    pub models: Vec<Model>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Model {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

#[derive(Deserialize, Debug)]
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
        println!("  {} {}", "Digest:".blue(), &self.digest[..12].dimmed());
        println!();
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
}
