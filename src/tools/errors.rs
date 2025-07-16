use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ToolError {
    #[error("File operation failed: {message}")]
    FileSystem {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Network request failed: {url} - {message}")]
    Network {
        url: String,
        message: String,
    },

    #[error("Model operation failed: {model} - {message}")]
    Model {
        model: String,
        message: String,
    },

    #[error("Git operation failed: {operation} - {message}")]
    Git {
        operation: String,
        message: String,
    },

    #[error("Docker operation failed: {operation} - {message}")]
    Docker {
        operation: String,
        message: String,
    },

    #[error("Database operation failed: {operation} - {message}")]
    Database {
        operation: String,
        message: String,
    },

    #[error("Package manager operation failed: {manager} - {message}")]
    PackageManager {
        manager: String,
        message: String,
    },

    #[error("Permission denied: {operation}")]
    Permission {
        operation: String,
        required_permission: String,
    },

    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    #[error("Invalid tool configuration: {config} - {message}")]
    InvalidConfig {
        config: String,
        message: String,
    },

    #[error("Tool execution timeout: {tool_name} (timeout: {timeout_ms}ms)")]
    Timeout {
        tool_name: String,
        timeout_ms: u64,
    },

    #[error("Tool chain execution failed at step {step}: {message}")]
    ChainExecution {
        step: usize,
        message: String,
    },

    #[error("Search operation failed: {query} - {message}")]
    Search {
        query: String,
        message: String,
    },

    #[error("Parse error: {input} - {message}")]
    Parse {
        input: String,
        message: String,
    },

    #[error("Validation error: {field}")]
    Validation {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("External command failed: {command}")]
    ExternalCommand {
        command: String,
        exit_code: Option<i32>,
        stderr: Option<String>,
    },

    #[error("Authentication failed: {service} - {message}")]
    Authentication {
        service: String,
        message: String,
    },

    #[error("Rate limited: {service}")]
    RateLimit {
        service: String,
        retry_after: Option<u64>,
    },
}

impl ToolError {
    pub fn file_system(source: std::io::Error, path: Option<PathBuf>) -> Self {
        Self::FileSystem { 
            message: source.to_string(),
            path 
        }
    }

    pub fn network(url: String, source: reqwest::Error) -> Self {
        Self::Network { 
            url,
            message: source.to_string(),
        }
    }

    pub fn model(model: String, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Model { 
            model,
            message: source.to_string(),
        }
    }

    pub fn git(operation: String, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Git { 
            operation,
            message: source.to_string(),
        }
    }

    pub fn docker(operation: String, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Docker { 
            operation,
            message: source.to_string(),
        }
    }

    pub fn database(operation: String, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Database { 
            operation,
            message: source.to_string(),
        }
    }

    pub fn permission(operation: String, required_permission: String) -> Self {
        Self::Permission {
            operation,
            required_permission,
        }
    }

    pub fn tool_not_found(tool_name: String) -> Self {
        Self::ToolNotFound { tool_name }
    }

    pub fn timeout(tool_name: String, timeout_ms: u64) -> Self {
        Self::Timeout {
            tool_name,
            timeout_ms,
        }
    }

    pub fn chain_execution(
        step: usize,
        message: String,
    ) -> Self {
        Self::ChainExecution {
            step,
            message,
        }
    }

    pub fn search(query: String, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Search { 
            query,
            message: source.to_string(),
        }
    }

    pub fn external_command(command: String, exit_code: Option<i32>, stderr: Option<String>) -> Self {
        Self::ExternalCommand {
            command,
            exit_code,
            stderr,
        }
    }

    pub fn validation(field: String, expected: String, actual: String) -> Self {
        Self::Validation {
            field,
            expected,
            actual,
        }
    }

    pub fn is_recoverable(&self) -> bool {
        match self {
            ToolError::Network { .. } => true,
            ToolError::Timeout { .. } => true,
            ToolError::RateLimit { .. } => true,
            ToolError::ExternalCommand { .. } => true,
            ToolError::Database { .. } => true,
            _ => false,
        }
    }

    pub fn retry_delay(&self) -> Option<std::time::Duration> {
        match self {
            ToolError::Network { .. } => Some(std::time::Duration::from_secs(1)),
            ToolError::Timeout { .. } => Some(std::time::Duration::from_millis(500)),
            ToolError::RateLimit { retry_after, .. } => {
                retry_after.map(std::time::Duration::from_secs)
            }
            _ => None,
        }
    }

    pub fn to_user_message(&self) -> String {
        match self {
            ToolError::FileSystem { message, path } => {
                let path_str = path.as_ref()
                    .map(|p| format!(" ({})", p.display()))
                    .unwrap_or_default();
                format!("File operation failed{}: {}", path_str, message)
            }
            ToolError::Network { url, message } => {
                format!("Network request to '{}' failed: {}", url, message)
            }
            ToolError::Permission { operation, required_permission } => {
                format!("Permission denied for '{}'. Required: {}", operation, required_permission)
            }
            ToolError::Timeout { tool_name, timeout_ms } => {
                format!("Tool '{}' timed out after {}ms", tool_name, timeout_ms)
            }
            ToolError::ToolNotFound { tool_name } => {
                format!("Tool '{}' not found or not available", tool_name)
            }
            ToolError::Validation { field, expected, actual } => {
                format!("Validation failed for '{}': expected '{}', got '{}'", field, expected, actual)
            }
            ToolError::RateLimit { service, retry_after } => {
                let retry_msg = retry_after
                    .map(|s| format!(" (retry after {}s)", s))
                    .unwrap_or_default();
                format!("Rate limited by '{}'{}", service, retry_msg)
            }
            _ => self.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub async fn execute<F, R, E>(&self, operation: F) -> Result<R, E>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, E>> + Send>>,
        E: Into<ToolError> + Clone,
    {
        let mut attempts = 0;
        let mut delay = self.config.base_delay;

        loop {
            let result = operation().await;
            
            match result {
                Ok(value) => return Ok(value),
                Err(error) => {
                    attempts += 1;
                    
                    if attempts > self.config.max_retries {
                        return Err(error);
                    }
                    
                    let tool_error: ToolError = error.clone().into();
                    if !tool_error.is_recoverable() {
                        return Err(error);
                    }
                    
                    // Use error-specific delay if available, otherwise use exponential backoff
                    let sleep_duration = tool_error.retry_delay().unwrap_or_else(|| {
                        let mut sleep_time = delay;
                        
                        // Add jitter if enabled
                        if self.config.jitter {
                            use rand::Rng;
                            let jitter_factor = rand::thread_rng().gen_range(0.8..1.2);
                            sleep_time = std::time::Duration::from_millis(
                                ((sleep_time.as_millis() as f64) * jitter_factor) as u64
                            );
                        }
                        
                        sleep_time
                    });
                    
                    tokio::time::sleep(sleep_duration).await;
                    
                    // Exponential backoff
                    delay = std::cmp::min(
                        std::time::Duration::from_millis(
                            ((delay.as_millis() as f64) * self.config.backoff_multiplier) as u64
                        ),
                        self.config.max_delay,
                    );
                }
            }
        }
    }
}

pub type ToolResult<T> = Result<T, ToolError>;