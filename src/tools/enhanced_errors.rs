use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub timestamp: SystemTime,
    pub operation: String,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<u32>,
    pub user_input: Option<String>,
    pub system_info: HashMap<String, String>,
    pub previous_errors: Vec<String>,
    pub suggested_actions: Vec<String>,
}

impl ErrorContext {
    pub fn new(operation: &str) -> Self {
        let mut system_info = HashMap::new();
        system_info.insert("os".to_string(), std::env::consts::OS.to_string());
        system_info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
        
        if let Ok(current_dir) = std::env::current_dir() {
            system_info.insert("current_dir".to_string(), current_dir.display().to_string());
        }

        Self {
            timestamp: SystemTime::now(),
            operation: operation.to_string(),
            file_path: None,
            line_number: None,
            user_input: None,
            system_info,
            previous_errors: Vec::new(),
            suggested_actions: Vec::new(),
        }
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line_number = Some(line);
        self
    }

    pub fn with_user_input(mut self, input: String) -> Self {
        self.user_input = Some(input);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggested_actions.push(suggestion);
        self
    }

    pub fn with_previous_error(mut self, error: String) -> Self {
        self.previous_errors.push(error);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,    // Recoverable, minor inconvenience
    Medium, // Significant issue, may affect functionality
    High,   // Critical error, major functionality broken
    Critical, // System-level failure
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "{}", "LOW".green()),
            ErrorSeverity::Medium => write!(f, "{}", "MEDIUM".yellow()),
            ErrorSeverity::High => write!(f, "{}", "HIGH".red()),
            ErrorSeverity::Critical => write!(f, "{}", "CRITICAL".red().bold()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    pub id: String,
    pub severity: ErrorSeverity,
    pub title: String,
    pub description: String,
    pub context: ErrorContext,
    pub recovery_suggestions: Vec<String>,
    pub related_errors: Vec<String>,
    pub help_links: Vec<String>,
    pub is_recoverable: bool,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl EnhancedError {
    pub fn new(severity: ErrorSeverity, title: &str, description: &str, context: ErrorContext) -> Self {
        let id = format!("ERR_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_uppercase());
        
        Self {
            id,
            severity,
            title: title.to_string(),
            description: description.to_string(),
            context,
            recovery_suggestions: Vec::new(),
            related_errors: Vec::new(),
            help_links: Vec::new(),
            is_recoverable: true,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.recovery_suggestions.push(suggestion);
        self
    }

    pub fn with_help_link(mut self, link: String) -> Self {
        self.help_links.push(link);
        self
    }

    pub fn with_related_error(mut self, error_id: String) -> Self {
        self.related_errors.push(error_id);
        self
    }

    pub fn set_non_recoverable(mut self) -> Self {
        self.is_recoverable = false;
        self
    }

    pub fn increment_retry(&mut self) -> bool {
        self.retry_count += 1;
        self.retry_count <= self.max_retries
    }

    pub fn display_detailed(&self) {
        println!();
        println!("{} {} [{}]", "🚨".red(), self.title.red().bold(), self.id.dimmed());
        println!("{} {}", "Severity:".blue(), self.severity);
        println!("{} {}", "Operation:".blue(), self.context.operation);
        
        if let Some(file_path) = &self.context.file_path {
            println!("{} {}", "File:".blue(), file_path.display());
        }
        
        if let Some(line) = self.context.line_number {
            println!("{} {}", "Line:".blue(), line);
        }
        
        if let Some(user_input) = &self.context.user_input {
            println!("{} {}", "User Input:".blue(), user_input.yellow());
        }
        
        println!();
        println!("{}", "Description:".blue());
        println!("  {}", self.description);
        
        if !self.recovery_suggestions.is_empty() {
            println!();
            println!("{}", "💡 Suggested Actions:".cyan());
            for (i, suggestion) in self.recovery_suggestions.iter().enumerate() {
                println!("  {}. {}", i + 1, suggestion);
            }
        }
        
        if !self.context.suggested_actions.is_empty() {
            println!();
            println!("{}", "🔧 System Suggestions:".cyan());
            for (i, suggestion) in self.context.suggested_actions.iter().enumerate() {
                println!("  {}. {}", i + 1, suggestion);
            }
        }
        
        if !self.help_links.is_empty() {
            println!();
            println!("{}", "📖 Documentation:".cyan());
            for link in &self.help_links {
                println!("  • {}", link.blue());
            }
        }
        
        if !self.context.previous_errors.is_empty() {
            println!();
            println!("{}", "⚠️ Previous Errors:".yellow());
            for error in &self.context.previous_errors {
                println!("  • {}", error.dimmed());
            }
        }
        
        if self.retry_count > 0 {
            println!();
            println!("{} Retry {}/{}", "🔄".yellow(), self.retry_count, self.max_retries);
        }
        
        println!();
    }

    pub fn display_compact(&self) {
        println!("{} {} [{}] {}", 
            "🚨".red(), 
            self.severity, 
            self.id.dimmed(), 
            self.title
        );
        
        if !self.recovery_suggestions.is_empty() {
            println!("  💡 Try: {}", self.recovery_suggestions[0]);
        }
    }

    pub fn to_user_friendly_message(&self) -> String {
        let mut message = format!("{}: {}", self.title, self.description);
        
        if !self.recovery_suggestions.is_empty() {
            message.push_str("\n\nSuggested action: ");
            message.push_str(&self.recovery_suggestions[0]);
        }
        
        message
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.title, self.description)
    }
}

impl std::error::Error for EnhancedError {}

pub struct ErrorManager {
    errors: Vec<EnhancedError>,
    max_stored_errors: usize,
}

impl ErrorManager {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            max_stored_errors: 100,
        }
    }

    pub fn add_error(&mut self, error: EnhancedError) -> String {
        let error_id = error.id.clone();
        
        self.errors.push(error);
        
        // Keep only the most recent errors
        if self.errors.len() > self.max_stored_errors {
            self.errors.remove(0);
        }
        
        error_id
    }

    pub fn get_error(&self, id: &str) -> Option<&EnhancedError> {
        self.errors.iter().find(|e| e.id == id)
    }

    pub fn get_errors_by_severity(&self, severity: ErrorSeverity) -> Vec<&EnhancedError> {
        self.errors.iter()
            .filter(|e| matches!(e.severity, ref s if std::mem::discriminant(s) == std::mem::discriminant(&severity)))
            .collect()
    }

    pub fn get_recent_errors(&self, count: usize) -> Vec<&EnhancedError> {
        self.errors.iter()
            .rev()
            .take(count)
            .collect()
    }

    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    pub fn get_error_statistics(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for error in &self.errors {
            let key = format!("{:?}", error.severity);
            *stats.entry(key).or_insert(0) += 1;
        }
        
        stats
    }

    pub fn display_error_summary(&self) {
        let stats = self.get_error_statistics();
        
        println!("{}", "📊 Error Summary:".cyan().bold());
        println!("  Total errors: {}", self.errors.len());
        
        for (severity, count) in stats {
            println!("  {}: {}", severity, count);
        }
        
        let recent_errors = self.get_recent_errors(5);
        if !recent_errors.is_empty() {
            println!();
            println!("{}", "🕐 Recent Errors:".cyan().bold());
            for error in recent_errors {
                error.display_compact();
            }
        }
    }
}

// Common error constructors
impl EnhancedError {
    pub fn file_not_found(path: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::Medium,
            "File Not Found",
            &format!("The file '{}' does not exist or is not accessible", path),
            context,
        )
        .with_suggestion("Check if the file path is correct".to_string())
        .with_suggestion("Verify file permissions".to_string())
        .with_help_link("https://docs.rs/std/latest/std/fs/".to_string())
    }

    pub fn permission_denied(operation: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::High,
            "Permission Denied",
            &format!("Access denied for operation: {}", operation),
            context,
        )
        .with_suggestion("Check file/directory permissions".to_string())
        .with_suggestion("Run with appropriate privileges if needed".to_string())
        .with_suggestion("Verify the file is not locked by another process".to_string())
    }

    pub fn network_error(url: &str, error: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::Medium,
            "Network Error",
            &format!("Failed to access {}: {}", url, error),
            context,
        )
        .with_suggestion("Check your internet connection".to_string())
        .with_suggestion("Verify the URL is correct".to_string())
        .with_suggestion("Try again later if the server is temporarily unavailable".to_string())
    }

    pub fn model_error(model: &str, error: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::High,
            "Model Error",
            &format!("Error with model '{}': {}", model, error),
            context,
        )
        .with_suggestion("Check if the model is installed".to_string())
        .with_suggestion("Verify Ollama is running".to_string())
        .with_suggestion("Try switching to a different model".to_string())
        .with_help_link("https://ollama.ai/library".to_string())
    }

    pub fn tool_execution_failed(tool_name: &str, error: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::Medium,
            "Tool Execution Failed",
            &format!("Tool '{}' failed: {}", tool_name, error),
            context,
        )
        .with_suggestion("Check tool dependencies".to_string())
        .with_suggestion("Verify tool configuration".to_string())
        .with_suggestion("Try running the tool manually to diagnose the issue".to_string())
    }

    pub fn parsing_error(input: &str, error: &str, context: ErrorContext) -> Self {
        Self::new(
            ErrorSeverity::Low,
            "Parsing Error",
            &format!("Failed to parse input '{}': {}", input, error),
            context,
        )
        .with_suggestion("Check input syntax".to_string())
        .with_suggestion("Refer to the documentation for correct format".to_string())
        .with_suggestion("Try rephrasing your request".to_string())
    }
}

// Global error manager instance
lazy_static::lazy_static! {
    static ref GLOBAL_ERROR_MANAGER: std::sync::Mutex<ErrorManager> = std::sync::Mutex::new(ErrorManager::new());
}

pub fn add_error(error: EnhancedError) -> String {
    GLOBAL_ERROR_MANAGER.lock().unwrap().add_error(error)
}

pub fn get_error(id: &str) -> Option<EnhancedError> {
    GLOBAL_ERROR_MANAGER.lock().unwrap().get_error(id).cloned()
}

pub fn display_error_summary() {
    GLOBAL_ERROR_MANAGER.lock().unwrap().display_error_summary();
}

pub fn clear_errors() {
    GLOBAL_ERROR_MANAGER.lock().unwrap().clear_errors();
}