use colored::Colorize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn to_string(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    fn color(&self) -> colored::Color {
        match self {
            LogLevel::Trace => colored::Color::BrightBlack,
            LogLevel::Debug => colored::Color::Cyan,
            LogLevel::Info => colored::Color::Green,
            LogLevel::Warn => colored::Color::Yellow,
            LogLevel::Error => colored::Color::Red,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

pub struct Logger {
    entries: Arc<RwLock<Vec<LogEntry>>>,
    max_entries: usize,
    min_level: LogLevel,
}

impl Logger {
    pub fn new(max_entries: usize, min_level: LogLevel) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            min_level,
        }
    }

    pub async fn log(&self, level: LogLevel, target: &str, message: &str, metadata: HashMap<String, String>) {
        if !self.should_log(&level) {
            return;
        }

        let entry = LogEntry {
            timestamp: Instant::now(),
            level: level.clone(),
            target: target.to_string(),
            message: message.to_string(),
            metadata,
        };

        // Print to console
        self.print_entry(&entry);

        // Store in memory
        let mut entries = self.entries.write().await;
        entries.push(entry);

        // Trim if needed
        if entries.len() > self.max_entries {
            entries.remove(0);
        }
    }

    pub async fn trace(&self, target: &str, message: &str) {
        self.log(LogLevel::Trace, target, message, HashMap::new()).await;
    }

    pub async fn debug(&self, target: &str, message: &str) {
        self.log(LogLevel::Debug, target, message, HashMap::new()).await;
    }

    pub async fn info(&self, target: &str, message: &str) {
        self.log(LogLevel::Info, target, message, HashMap::new()).await;
    }

    pub async fn warn(&self, target: &str, message: &str) {
        self.log(LogLevel::Warn, target, message, HashMap::new()).await;
    }

    pub async fn error(&self, target: &str, message: &str) {
        self.log(LogLevel::Error, target, message, HashMap::new()).await;
    }

    pub async fn log_with_metadata(&self, level: LogLevel, target: &str, message: &str, metadata: HashMap<String, String>) {
        self.log(level, target, message, metadata).await;
    }

    pub async fn get_entries(&self) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries.clone()
    }

    pub async fn get_entries_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries.iter()
            .filter(|entry| std::mem::discriminant(&entry.level) == std::mem::discriminant(&level))
            .cloned()
            .collect()
    }

    pub async fn get_entries_by_target(&self, target: &str) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries.iter()
            .filter(|entry| entry.target == target)
            .cloned()
            .collect()
    }

    pub async fn get_recent_entries(&self, duration: Duration) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        let cutoff = Instant::now() - duration;
        entries.iter()
            .filter(|entry| entry.timestamp > cutoff)
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn export_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let entries = self.entries.read().await;
        let mut content = String::new();
        
        for entry in entries.iter() {
            content.push_str(&format!(
                "{:?} [{}] {}: {}\n",
                entry.timestamp,
                entry.level.to_string(),
                entry.target,
                entry.message
            ));
            
            for (key, value) in &entry.metadata {
                content.push_str(&format!("  {}: {}\n", key, value));
            }
        }

        tokio::fs::write(path, content).await?;
        Ok(())
    }

    fn should_log(&self, level: &LogLevel) -> bool {
        let level_value = match level {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        };

        let min_level_value = match self.min_level {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        };

        level_value >= min_level_value
    }

    fn print_entry(&self, entry: &LogEntry) {
        let timestamp = chrono::DateTime::<chrono::Local>::from(
            std::time::SystemTime::now()
        ).format("%H:%M:%S%.3f");
        
        let level_str = entry.level.to_string().color(entry.level.color());
        let target_str = entry.target.dimmed();
        
        println!("{} {} [{}] {}", 
            timestamp.to_string().dimmed(),
            level_str,
            target_str,
            entry.message
        );

        // Print metadata if present
        if !entry.metadata.is_empty() {
            for (key, value) in &entry.metadata {
                println!("  {}: {}", key.blue(), value.yellow());
            }
        }
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            max_entries: self.max_entries,
            min_level: self.min_level.clone(),
        }
    }
}

// Performance monitoring
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub tool_executions: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    pub error_counts: Arc<RwLock<HashMap<String, usize>>>,
    pub success_counts: Arc<RwLock<HashMap<String, usize>>>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            tool_executions: Arc::new(RwLock::new(HashMap::new())),
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            success_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_execution(&self, tool_name: &str, duration: Duration) {
        let mut executions = self.tool_executions.write().await;
        executions.entry(tool_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    pub async fn record_success(&self, tool_name: &str) {
        let mut counts = self.success_counts.write().await;
        *counts.entry(tool_name.to_string()).or_insert(0) += 1;
    }

    pub async fn record_error(&self, tool_name: &str) {
        let mut counts = self.error_counts.write().await;
        *counts.entry(tool_name.to_string()).or_insert(0) += 1;
    }

    pub async fn get_average_execution_time(&self, tool_name: &str) -> Option<Duration> {
        let executions = self.tool_executions.read().await;
        if let Some(durations) = executions.get(tool_name) {
            if durations.is_empty() {
                return None;
            }
            let total: Duration = durations.iter().sum();
            Some(total / durations.len() as u32)
        } else {
            None
        }
    }

    pub async fn get_success_rate(&self, tool_name: &str) -> Option<f64> {
        let successes = self.success_counts.read().await;
        let errors = self.error_counts.read().await;
        
        let success_count = successes.get(tool_name).copied().unwrap_or(0);
        let error_count = errors.get(tool_name).copied().unwrap_or(0);
        
        let total = success_count + error_count;
        if total > 0 {
            Some(success_count as f64 / total as f64)
        } else {
            None
        }
    }

    pub async fn get_stats_summary(&self) -> String {
        let executions = self.tool_executions.read().await;
        let successes = self.success_counts.read().await;
        let errors = self.error_counts.read().await;
        
        let mut summary = String::new();
        summary.push_str(&format!("{}\n", "Performance Summary:".cyan().bold()));
        
        let mut all_tools: std::collections::HashSet<String> = std::collections::HashSet::new();
        all_tools.extend(executions.keys().cloned());
        all_tools.extend(successes.keys().cloned());
        all_tools.extend(errors.keys().cloned());
        
        for tool in all_tools {
            let avg_time = if let Some(durations) = executions.get(&tool) {
                if !durations.is_empty() {
                    let total: Duration = durations.iter().sum();
                    Some(total / durations.len() as u32)
                } else {
                    None
                }
            } else {
                None
            };
            
            let success_count = successes.get(&tool).copied().unwrap_or(0);
            let error_count = errors.get(&tool).copied().unwrap_or(0);
            let total_count = success_count + error_count;
            
            let success_rate = if total_count > 0 {
                success_count as f64 / total_count as f64
            } else {
                0.0
            };
            
            summary.push_str(&format!(
                "  {}: {} executions, {:.1}% success rate",
                tool.yellow(),
                total_count,
                success_rate * 100.0
            ));
            
            if let Some(avg) = avg_time {
                summary.push_str(&format!(", avg: {:.2}s", avg.as_secs_f64()));
            }
            
            summary.push('\n');
        }
        
        summary
    }

    pub async fn clear(&self) {
        let mut executions = self.tool_executions.write().await;
        let mut successes = self.success_counts.write().await;
        let mut errors = self.error_counts.write().await;
        
        executions.clear();
        successes.clear();
        errors.clear();
    }
}

// Global logger instance
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GLOBAL_LOGGER: Logger = Logger::new(10000, LogLevel::Info);
    pub static ref GLOBAL_METRICS: PerformanceMetrics = PerformanceMetrics::new();
}

// Convenience macros
#[macro_export]
macro_rules! log_trace {
    ($target:expr, $($arg:tt)*) => {
        tokio::spawn(async move {
            crate::tools::logging::GLOBAL_LOGGER.trace($target, &format!($($arg)*)).await;
        });
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $($arg:tt)*) => {
        tokio::spawn(async move {
            crate::tools::logging::GLOBAL_LOGGER.debug($target, &format!($($arg)*)).await;
        });
    };
}

#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)*) => {
        tokio::spawn(async move {
            crate::tools::logging::GLOBAL_LOGGER.info($target, &format!($($arg)*)).await;
        });
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)*) => {
        tokio::spawn(async move {
            crate::tools::logging::GLOBAL_LOGGER.warn($target, &format!($($arg)*)).await;
        });
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)*) => {
        tokio::spawn(async move {
            crate::tools::logging::GLOBAL_LOGGER.error($target, &format!($($arg)*)).await;
        });
    };
}

pub async fn show_performance_summary() {
    let summary = GLOBAL_METRICS.get_stats_summary().await;
    println!("{}", summary);
}

pub async fn clear_logs() {
    GLOBAL_LOGGER.clear().await;
    GLOBAL_METRICS.clear().await;
    println!("{}", "Logs and metrics cleared".green());
}