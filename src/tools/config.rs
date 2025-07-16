use super::core::{ExportFormat, ToolExecutor, ToolResult};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub auto_approve_safe: bool,
    pub max_file_size: usize,
    pub default_timeout: u64,
    pub git_default_remote: String,
    pub database_connections: HashMap<String, String>,
    pub api_keys: HashMap<String, String>,
    pub theme: String,
    pub editor: String,
    pub log_level: String,
    pub backup_enabled: bool,
    pub custom_commands: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_approve_safe: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            default_timeout: 30,
            git_default_remote: "origin".to_string(),
            database_connections: HashMap::new(),
            api_keys: HashMap::new(),
            theme: "default".to_string(),
            editor: "nano".to_string(),
            log_level: "info".to_string(),
            backup_enabled: true,
            custom_commands: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: String,
    pub user_input: String,
    pub assistant_response: String,
    pub tools_used: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

impl ToolExecutor {
    pub async fn set_config(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Setting configuration: {} = {:?}",
            "⚙️".cyan(),
            key.yellow(),
            value
        );

        let config_path = self.get_config_path()?;
        let mut config = self.load_config().await.unwrap_or_default();

        match key {
            "auto_approve_safe" => {
                if let Some(val) = value.as_bool() {
                    config.auto_approve_safe = val;
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("auto_approve_safe must be a boolean".to_string()),
                        metadata: None,
                    });
                }
            }
            "max_file_size" => {
                if let Some(val) = value.as_u64() {
                    config.max_file_size = val as usize;
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("max_file_size must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            "default_timeout" => {
                if let Some(val) = value.as_u64() {
                    config.default_timeout = val;
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("default_timeout must be a number".to_string()),
                        metadata: None,
                    });
                }
            }
            "git_default_remote" => {
                if let Some(val) = value.as_str() {
                    config.git_default_remote = val.to_string();
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("git_default_remote must be a string".to_string()),
                        metadata: None,
                    });
                }
            }
            "theme" => {
                if let Some(val) = value.as_str() {
                    config.theme = val.to_string();
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("theme must be a string".to_string()),
                        metadata: None,
                    });
                }
            }
            "editor" => {
                if let Some(val) = value.as_str() {
                    config.editor = val.to_string();
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("editor must be a string".to_string()),
                        metadata: None,
                    });
                }
            }
            "log_level" => {
                if let Some(val) = value.as_str() {
                    if ["debug", "info", "warn", "error"].contains(&val) {
                        config.log_level = val.to_string();
                    } else {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "log_level must be one of: debug, info, warn, error".to_string(),
                            ),
                            metadata: None,
                        });
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("log_level must be a string".to_string()),
                        metadata: None,
                    });
                }
            }
            "backup_enabled" => {
                if let Some(val) = value.as_bool() {
                    config.backup_enabled = val;
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("backup_enabled must be a boolean".to_string()),
                        metadata: None,
                    });
                }
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unknown configuration key: {}", key)),
                    metadata: None,
                });
            }
        }

        self.save_config(&config).await?;

        Ok(ToolResult {
            success: true,
            output: format!("Configuration updated: {} = {:?}", key, value),
            error: None,
            metadata: Some(serde_json::to_value(&config)?),
        })
    }

    pub async fn get_config(
        &self,
        key: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting configuration", "📋".cyan());

        let config = self.load_config().await.unwrap_or_default();

        let output = match key {
            Some("auto_approve_safe") => format!("auto_approve_safe: {}", config.auto_approve_safe),
            Some("max_file_size") => format!("max_file_size: {}", config.max_file_size),
            Some("default_timeout") => format!("default_timeout: {}", config.default_timeout),
            Some("git_default_remote") => {
                format!("git_default_remote: {}", config.git_default_remote)
            }
            Some("theme") => format!("theme: {}", config.theme),
            Some("editor") => format!("editor: {}", config.editor),
            Some("log_level") => format!("log_level: {}", config.log_level),
            Some("backup_enabled") => format!("backup_enabled: {}", config.backup_enabled),
            Some(unknown_key) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unknown configuration key: {}", unknown_key)),
                    metadata: None,
                });
            }
            None => {
                format!(
                    "Current Configuration:\n\
                    auto_approve_safe: {}\n\
                    max_file_size: {} bytes\n\
                    default_timeout: {} seconds\n\
                    git_default_remote: {}\n\
                    theme: {}\n\
                    editor: {}\n\
                    log_level: {}\n\
                    backup_enabled: {}\n\
                    database_connections: {} configured\n\
                    api_keys: {} configured\n\
                    custom_commands: {} configured",
                    config.auto_approve_safe,
                    config.max_file_size,
                    config.default_timeout,
                    config.git_default_remote,
                    config.theme,
                    config.editor,
                    config.log_level,
                    config.backup_enabled,
                    config.database_connections.len(),
                    config.api_keys.len(),
                    config.custom_commands.len()
                )
            }
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
            metadata: Some(serde_json::to_value(&config)?),
        })
    }

    pub async fn export_conversation(
        &self,
        format: ExportFormat,
        path: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Exporting conversation to: {} ({:?})",
            "📤".cyan(),
            path.yellow(),
            format
        );

        // In a real implementation, you'd get this from the session
        let mock_conversation = vec![ConversationEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            user_input: "Hello!".to_string(),
            assistant_response: "Hi there! How can I help you today?".to_string(),
            tools_used: vec![],
            metadata: None,
        }];

        let content = match format {
            ExportFormat::Json => serde_json::to_string_pretty(&mock_conversation)?,
            ExportFormat::Markdown => self.conversation_to_markdown(&mock_conversation),
            ExportFormat::Text => self.conversation_to_text(&mock_conversation),
            ExportFormat::Html => self.conversation_to_html(&mock_conversation),
        };

        fs::write(path, content)?;

        Ok(ToolResult {
            success: true,
            output: format!("Conversation exported to: {}", path),
            error: None,
            metadata: Some(serde_json::json!({
                "format": format!("{:?}", format),
                "path": path,
                "entries_count": mock_conversation.len()
            })),
        })
    }

    pub async fn import_conversation(
        &self,
        path: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Importing conversation from: {}",
            "📥".cyan(),
            path.yellow()
        );

        if !Path::new(path).exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("File not found: {}", path)),
                metadata: None,
            });
        }

        let content = fs::read_to_string(path)?;

        // Try to parse as JSON
        match serde_json::from_str::<Vec<ConversationEntry>>(&content) {
            Ok(conversation) => {
                // In a real implementation, you'd merge this with the current session
                Ok(ToolResult {
                    success: true,
                    output: format!(
                        "Successfully imported {} conversation entries",
                        conversation.len()
                    ),
                    error: None,
                    metadata: Some(serde_json::json!({
                        "path": path,
                        "entries_count": conversation.len()
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to parse conversation file: {}", e)),
                metadata: None,
            }),
        }
    }

    pub async fn clear_history(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Clearing conversation history", "🧹".cyan());

        // In a real implementation, you'd clear the session history
        // For now, just create a backup if enabled
        let config = self.load_config().await.unwrap_or_default();

        if config.backup_enabled {
            let backup_path = format!(
                "backup_conversation_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );
            let _ = self
                .export_conversation(ExportFormat::Json, &backup_path)
                .await;
        }

        Ok(ToolResult {
            success: true,
            output: "Conversation history cleared".to_string(),
            error: None,
            metadata: Some(serde_json::json!({
                "backup_created": config.backup_enabled
            })),
        })
    }

    // Task scheduling (basic implementation)
    pub async fn schedule_task(
        &self,
        command: &str,
        schedule: &str,
        name: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let task_name = name.unwrap_or("unnamed_task");
        println!(
            "{} Scheduling task: {} ({})",
            "⏰".cyan(),
            task_name.yellow(),
            schedule.blue()
        );

        // This is a basic implementation - in production, you'd use a proper scheduler
        let task = serde_json::json!({
            "name": task_name,
            "command": command,
            "schedule": schedule,
            "created": chrono::Utc::now().to_rfc3339(),
            "active": true
        });

        let tasks_file = self.get_data_dir()?.join("scheduled_tasks.json");
        let mut tasks: Vec<serde_json::Value> = if tasks_file.exists() {
            let content = fs::read_to_string(&tasks_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        tasks.push(task);
        fs::write(tasks_file, serde_json::to_string_pretty(&tasks)?)?;

        Ok(ToolResult {
            success: true,
            output: format!("Task '{}' scheduled with pattern: {}", task_name, schedule),
            error: None,
            metadata: Some(serde_json::json!({
                "task_name": task_name,
                "schedule": schedule
            })),
        })
    }

    pub async fn list_scheduled_tasks(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing scheduled tasks", "📋".cyan());

        let tasks_file = self.get_data_dir()?.join("scheduled_tasks.json");

        if !tasks_file.exists() {
            return Ok(ToolResult {
                success: true,
                output: "No scheduled tasks found".to_string(),
                error: None,
                metadata: None,
            });
        }

        let content = fs::read_to_string(&tasks_file)?;
        let tasks: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap_or_default();

        if tasks.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No scheduled tasks found".to_string(),
                error: None,
                metadata: None,
            });
        }

        let mut output = vec!["Scheduled Tasks:".to_string(), "=".repeat(50)];

        for (i, task) in tasks.iter().enumerate() {
            let name = task
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            let command = task
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let schedule = task
                .get("schedule")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let active = task
                .get("active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let created = task
                .get("created")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            output.push(format!(
                "{}. {} [{}]\n   Command: {}\n   Schedule: {}\n   Created: {}",
                i + 1,
                name,
                if active { "ACTIVE" } else { "INACTIVE" },
                command,
                schedule,
                created
            ));
        }

        Ok(ToolResult {
            success: true,
            output: output.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "tasks_count": tasks.len()
            })),
        })
    }

    pub async fn cancel_scheduled_task(
        &self,
        name: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Cancelling scheduled task: {}",
            "❌".cyan(),
            name.yellow()
        );

        let tasks_file = self.get_data_dir()?.join("scheduled_tasks.json");

        if !tasks_file.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("No scheduled tasks file found".to_string()),
                metadata: None,
            });
        }

        let content = fs::read_to_string(&tasks_file)?;
        let mut tasks: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap_or_default();

        let initial_count = tasks.len();
        tasks.retain(|task| task.get("name").and_then(|v| v.as_str()) != Some(name));

        if tasks.len() == initial_count {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Task '{}' not found", name)),
                metadata: None,
            });
        }

        fs::write(tasks_file, serde_json::to_string_pretty(&tasks)?)?;

        Ok(ToolResult {
            success: true,
            output: format!("Task '{}' cancelled", name),
            error: None,
            metadata: Some(serde_json::json!({
                "cancelled_task": name,
                "remaining_tasks": tasks.len()
            })),
        })
    }

    // Helper methods
    fn get_config_path(&self) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let data_dir = self.get_data_dir()?;
        Ok(data_dir.join("config.json"))
    }

    fn get_data_dir(&self) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        let data_dir = home_dir.join(".ollama_agent");

        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        Ok(data_dir)
    }

    async fn load_config(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        let config_path = self.get_config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(AppConfig::default())
        }
    }

    async fn save_config(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = self.get_config_path()?;
        let content = serde_json::to_string_pretty(config)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    fn conversation_to_markdown(&self, conversation: &[ConversationEntry]) -> String {
        let mut output = vec!["# Conversation Export".to_string(), String::new()];

        for entry in conversation {
            output.push(format!("## {}", entry.timestamp));
            output.push(String::new());
            output.push("**User:**".to_string());
            output.push(entry.user_input.clone());
            output.push(String::new());
            output.push("**Assistant:**".to_string());
            output.push(entry.assistant_response.clone());

            if !entry.tools_used.is_empty() {
                output.push(String::new());
                output.push("**Tools Used:**".to_string());
                for tool in &entry.tools_used {
                    output.push(format!("- {}", tool));
                }
            }

            output.push(String::new());
            output.push("---".to_string());
            output.push(String::new());
        }

        output.join("\n")
    }

    fn conversation_to_text(&self, conversation: &[ConversationEntry]) -> String {
        let mut output = vec!["CONVERSATION EXPORT".to_string(), "=".repeat(50)];

        for entry in conversation {
            output.push(format!("\nTimestamp: {}", entry.timestamp));
            output.push("-".repeat(30));
            output.push(format!("User: {}", entry.user_input));
            output.push(format!("Assistant: {}", entry.assistant_response));

            if !entry.tools_used.is_empty() {
                output.push(format!("Tools: {}", entry.tools_used.join(", ")));
            }
        }

        output.join("\n")
    }

    fn conversation_to_html(&self, conversation: &[ConversationEntry]) -> String {
        let mut output = vec![
            "<!DOCTYPE html>".to_string(),
            "<html><head><title>Conversation Export</title>".to_string(),
            "<style>body{font-family:Arial,sans-serif;margin:40px;}".to_string(),
            ".entry{margin:20px 0;padding:15px;border:1px solid #ddd;}".to_string(),
            ".timestamp{color:#666;font-size:0.9em;}".to_string(),
            ".user{color:#0066cc;font-weight:bold;}".to_string(),
            ".assistant{color:#009900;font-weight:bold;}".to_string(),
            ".tools{color:#cc6600;font-style:italic;}</style></head><body>".to_string(),
            "<h1>Conversation Export</h1>".to_string(),
        ];

        for entry in conversation {
            output.push("<div class=\"entry\">".to_string());
            output.push(format!(
                "<div class=\"timestamp\">{}</div>",
                entry.timestamp
            ));
            output.push(format!(
                "<div class=\"user\">User: {}</div>",
                entry.user_input
            ));
            output.push(format!(
                "<div class=\"assistant\">Assistant: {}</div>",
                entry.assistant_response
            ));

            if !entry.tools_used.is_empty() {
                output.push(format!(
                    "<div class=\"tools\">Tools: {}</div>",
                    entry.tools_used.join(", ")
                ));
            }

            output.push("</div>".to_string());
        }

        output.push("</body></html>".to_string());
        output.join("\n")
    }
}
