use super::core::{ModelParameter, ToolExecutor, ToolResult};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub top_k: u32,
    pub repeat_penalty: f32,
    pub system_prompt: String,
    pub context_length: u32,
    pub current_model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            system_prompt: "You are a helpful AI assistant.".to_string(),
            context_length: 4096,
            current_model: "llama2".to_string(),
        }
    }
}

// Global model configuration state
lazy_static::lazy_static! {
    static ref MODEL_CONFIG: Arc<Mutex<ModelConfig>> = Arc::new(Mutex::new(ModelConfig::default()));
}

impl ToolExecutor {
    pub async fn set_model_parameter(
        &self,
        parameter: ModelParameter,
        value: serde_json::Value,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Setting model parameter: {:?}", "⚙️".cyan(), parameter);

        let mut config = MODEL_CONFIG
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;

        let result = match parameter {
            ModelParameter::Temperature => {
                if let Some(temp) = value.as_f64() {
                    if temp >= 0.0 && temp <= 2.0 {
                        config.temperature = temp as f32;
                        format!("Temperature set to {}", temp)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Temperature must be between 0.0 and 2.0".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Temperature must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::MaxTokens => {
                if let Some(tokens) = value.as_u64() {
                    if tokens > 0 && tokens <= 32768 {
                        config.max_tokens = tokens as u32;
                        format!("Max tokens set to {}", tokens)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Max tokens must be between 1 and 32768".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Max tokens must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::TopP => {
                if let Some(top_p) = value.as_f64() {
                    if top_p >= 0.0 && top_p <= 1.0 {
                        config.top_p = top_p as f32;
                        format!("Top-p set to {}", top_p)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Top-p must be between 0.0 and 1.0".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Top-p must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::TopK => {
                if let Some(top_k) = value.as_u64() {
                    if top_k > 0 && top_k <= 100 {
                        config.top_k = top_k as u32;
                        format!("Top-k set to {}", top_k)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Top-k must be between 1 and 100".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Top-k must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::RepeatPenalty => {
                if let Some(penalty) = value.as_f64() {
                    if penalty >= 0.5 && penalty <= 2.0 {
                        config.repeat_penalty = penalty as f32;
                        format!("Repeat penalty set to {}", penalty)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Repeat penalty must be between 0.5 and 2.0".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Repeat penalty must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::SystemPrompt => {
                if let Some(prompt) = value.as_str() {
                    config.system_prompt = prompt.to_string();
                    format!("System prompt updated ({} characters)", prompt.len())
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("System prompt must be a string".to_string()),
                        metadata: None,
                    });
                }
            }
            ModelParameter::ContextLength => {
                if let Some(length) = value.as_u64() {
                    if length >= 512 && length <= 32768 {
                        config.context_length = length as u32;
                        format!("Context length set to {}", length)
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Context length must be between 512 and 32768".to_string()),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Context length must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
        };

        Ok(ToolResult {
            success: true,
            output: result,
            error: None,
            metadata: Some(serde_json::to_value(&*config)?),
        })
    }

    pub async fn get_model_parameter(
        &self,
        parameter: Option<ModelParameter>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting model parameters", "📊".cyan());

        let config = MODEL_CONFIG
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;

        let output = match parameter {
            Some(ModelParameter::Temperature) => format!("Temperature: {}", config.temperature),
            Some(ModelParameter::MaxTokens) => format!("Max tokens: {}", config.max_tokens),
            Some(ModelParameter::TopP) => format!("Top-p: {}", config.top_p),
            Some(ModelParameter::TopK) => format!("Top-k: {}", config.top_k),
            Some(ModelParameter::RepeatPenalty) => {
                format!("Repeat penalty: {}", config.repeat_penalty)
            }
            Some(ModelParameter::SystemPrompt) => {
                format!("System prompt: {}", config.system_prompt)
            }
            Some(ModelParameter::ContextLength) => {
                format!("Context length: {}", config.context_length)
            }
            None => {
                format!(
                    "Current Model Configuration:\n\
                    Model: {}\n\
                    Temperature: {}\n\
                    Max Tokens: {}\n\
                    Top-p: {}\n\
                    Top-k: {}\n\
                    Repeat Penalty: {}\n\
                    Context Length: {}\n\
                    System Prompt: {}",
                    config.current_model,
                    config.temperature,
                    config.max_tokens,
                    config.top_p,
                    config.top_k,
                    config.repeat_penalty,
                    config.context_length,
                    if config.system_prompt.len() > 100 {
                        format!("{}...", &config.system_prompt[..100])
                    } else {
                        config.system_prompt.clone()
                    }
                )
            }
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
            metadata: Some(serde_json::to_value(&*config)?),
        })
    }

    pub async fn switch_model(
        &self,
        model_name: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Switching to model: {}",
            "🔄".cyan(),
            model_name.yellow()
        );

        // First, check if the model is available
        let available_models = crate::client::fetch_models().await?;
        let model_exists = available_models.iter().any(|m| m.name == model_name);

        if !model_exists {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Model '{}' not found. Available models: {}",
                    model_name,
                    available_models
                        .iter()
                        .map(|m| m.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                metadata: None,
            });
        }

        let mut config = MODEL_CONFIG
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;
        let old_model = config.current_model.clone();
        config.current_model = model_name.to_string();

        Ok(ToolResult {
            success: true,
            output: format!("Model switched from '{}' to '{}'", old_model, model_name),
            error: None,
            metadata: Some(serde_json::to_value(&*config)?),
        })
    }
}

// Enhanced Ollama request structure with parameters
#[derive(serde::Serialize, Debug)]
pub struct EnhancedOllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: OllamaOptions,
}

#[derive(serde::Serialize, Debug)]
pub struct OllamaOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub repeat_penalty: f32,
    pub num_predict: u32, // max_tokens in Ollama
    pub num_ctx: u32,     // context_length in Ollama
}

impl From<&ModelConfig> for OllamaOptions {
    fn from(config: &ModelConfig) -> Self {
        Self {
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k,
            repeat_penalty: config.repeat_penalty,
            num_predict: config.max_tokens,
            num_ctx: config.context_length,
        }
    }
}

// Function to get current model config for use in client.rs
pub fn get_current_model_config() -> ModelConfig {
    MODEL_CONFIG.lock().unwrap().clone()
}

// Function to update the current model in the global config
pub fn set_current_model(model_name: &str) {
    if let Ok(mut config) = MODEL_CONFIG.lock() {
        config.current_model = model_name.to_string();
    }
}

// Function to create enhanced request with current parameters
pub fn create_enhanced_request(_model: &str, prompt: &str, stream: bool) -> EnhancedOllamaRequest {
    let config = get_current_model_config();

    EnhancedOllamaRequest {
        model: config.current_model.clone(), // Use the current model from config, not the parameter
        prompt: if config.system_prompt.is_empty() || config.system_prompt == "You are a helpful AI assistant." {
            prompt.to_string()
        } else {
            format!("{}\n\nUser: {}", config.system_prompt, prompt)
        },
        stream,
        options: OllamaOptions::from(&config),
    }
}
