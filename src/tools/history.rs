use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use colored::Colorize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub user_input: String,
    pub assistant_response: String,
    pub tools_used: Vec<String>,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub entries: VecDeque<HistoryEntry>,
    pub max_entries: usize,
}

impl ConversationHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    pub fn add_entry(&mut self, entry: HistoryEntry) {
        self.entries.push_back(entry);
        
        // Keep only the last max_entries
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn get_recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                entry.user_input.to_lowercase().contains(&query_lower)
                    || entry.assistant_response.to_lowercase().contains(&query_lower)
            })
            .rev()
            .take(limit)
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn export_to_markdown(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = String::new();
        content.push_str("# Conversation History\n\n");
        
        for entry in &self.entries {
            content.push_str(&format!("## {}\n\n", entry.timestamp));
            content.push_str(&format!("**User**: {}\n\n", entry.user_input));
            content.push_str(&format!("**Assistant**: {}\n\n", entry.assistant_response));
            
            if !entry.tools_used.is_empty() {
                content.push_str("**Tools Used**: ");
                content.push_str(&entry.tools_used.join(", "));
                content.push_str("\n\n");
            }
            
            content.push_str("---\n\n");
        }
        
        fs::write(path, content)?;
        Ok(())
    }

    pub fn export_to_json(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn export_to_text(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = String::new();
        content.push_str("CONVERSATION HISTORY\n");
        content.push_str("===================\n\n");
        
        for entry in &self.entries {
            content.push_str(&format!("Time: {}\n", entry.timestamp));
            content.push_str(&format!("User: {}\n", entry.user_input));
            content.push_str(&format!("Assistant: {}\n", entry.assistant_response));
            
            if !entry.tools_used.is_empty() {
                content.push_str(&format!("Tools: {}\n", entry.tools_used.join(", ")));
            }
            
            content.push_str("\n---\n\n");
        }
        
        fs::write(path, content)?;
        Ok(())
    }
}

pub struct HistoryManager {
    history: ConversationHistory,
    file_path: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Self {
        let file_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ollama-cli-assistant")
            .join("history.json");
        
        let history = Self::load_from_file(&file_path).unwrap_or_else(|_| {
            ConversationHistory::new(100) // Default max entries
        });

        Self { history, file_path }
    }

    pub fn add_entry(&mut self, entry: HistoryEntry) {
        self.history.add_entry(entry);
        let _ = self.save_to_file(); // Ignore errors for now
    }

    pub fn get_recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.history.get_recent(count)
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<&HistoryEntry> {
        self.history.search(query, limit)
    }

    pub fn clear(&mut self) {
        self.history.clear();
        let _ = self.save_to_file();
    }

    pub fn export(&self, path: &str, format: &str) -> Result<(), Box<dyn std::error::Error>> {
        match format.to_lowercase().as_str() {
            "markdown" | "md" => self.history.export_to_markdown(path),
            "json" => self.history.export_to_json(path),
            "text" | "txt" => self.history.export_to_text(path),
            _ => Err(format!("Unsupported format: {}", format).into()),
        }
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create directory if it doesn't exist
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&self.history)?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    fn load_from_file(path: &PathBuf) -> Result<ConversationHistory, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let history: ConversationHistory = serde_json::from_str(&content)?;
        Ok(history)
    }

    pub fn show_entries(&self, entries: &[&HistoryEntry], detailed: bool) {
        if entries.is_empty() {
            println!("{} No history entries found", "ℹ️".blue());
            return;
        }

        println!("{} {} entries found", "📜".cyan(), entries.len());
        println!();

        for (i, entry) in entries.iter().enumerate() {
            println!("{} {} {}", "●".blue(), (i + 1).to_string().yellow(), entry.timestamp.dimmed());
            
            if detailed {
                println!("   {} {}", "User:".blue(), entry.user_input);
                let response_preview = if entry.assistant_response.len() > 150 {
                    format!("{}...", &entry.assistant_response[..150])
                } else {
                    entry.assistant_response.clone()
                };
                println!("   {} {}", "Assistant:".green(), response_preview);
                
                if !entry.tools_used.is_empty() {
                    println!("   {} {}", "Tools:".yellow(), entry.tools_used.join(", "));
                }
            } else {
                let input_preview = if entry.user_input.len() > 80 {
                    format!("{}...", &entry.user_input[..80])
                } else {
                    entry.user_input.clone()
                };
                println!("   {}", input_preview);
            }
            
            println!();
        }
    }
}