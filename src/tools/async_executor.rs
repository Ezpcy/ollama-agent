use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};

use super::core::{ToolResult, AvailableTool};

pub struct AsyncToolExecutor {
    /// Semaphore to limit concurrent tool executions
    execution_semaphore: Arc<Semaphore>,
    /// Connection pool for HTTP requests
    http_client: reqwest::Client,
    /// Cache for tool results
    result_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    /// Resource limits
    resource_limits: ResourceLimits,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_concurrent_tools: usize,
    pub max_memory_mb: usize,
    pub max_cpu_usage: f64,
    pub max_network_requests_per_minute: usize,
    pub max_file_size_mb: usize,
    pub max_search_results: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_tools: 10,
            max_memory_mb: 1024,
            max_cpu_usage: 80.0,
            max_network_requests_per_minute: 100,
            max_file_size_mb: 100,
            max_search_results: 1000,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedResult {
    result: ToolResult,
    timestamp: std::time::Instant,
    ttl: Duration,
}

impl CachedResult {
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

impl AsyncToolExecutor {
    pub fn new(resource_limits: ResourceLimits) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            execution_semaphore: Arc::new(Semaphore::new(resource_limits.max_concurrent_tools)),
            http_client,
            result_cache: Arc::new(RwLock::new(HashMap::new())),
            resource_limits,
        }
    }

    pub async fn execute_tool_with_retry(
        &self,
        tool: &AvailableTool,
    ) -> Result<ToolResult, super::errors::ToolError> {
        let tool_name = self.get_tool_name(tool);
        
        // Check cache first
        if let Some(cached) = self.get_cached_result(&tool_name).await {
            return Ok(cached);
        }

        // Acquire semaphore to limit concurrent executions
        let _permit = self.execution_semaphore.acquire().await
            .map_err(|_e| super::errors::ToolError::timeout(tool_name.clone(), 0))?;

        let result = self.execute_tool_internal(tool).await?;

        // Cache successful results
        if result.success {
            self.cache_result(&tool_name, result.clone(), Duration::from_secs(300)).await;
        }

        Ok(result)
    }

    pub async fn get_resource_usage(&self) -> ResourceUsage {
        let cache = self.result_cache.read().await;
        let available_permits = self.execution_semaphore.available_permits();
        
        ResourceUsage {
            concurrent_executions: self.resource_limits.max_concurrent_tools - available_permits,
            cached_results: cache.len(),
            memory_usage_mb: self.estimate_memory_usage(),
            active_connections: 0, // Approximation - would need proper connection pool metrics
        }
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.result_cache.write().await;
        cache.clear();
    }

    pub async fn cleanup_expired_cache(&self) {
        let mut cache = self.result_cache.write().await;
        cache.retain(|_, result| !result.is_expired());
    }

    // Private helper methods
    
    async fn execute_tool_internal(&self, tool: &AvailableTool) -> Result<ToolResult, super::errors::ToolError> {
        // This would delegate to the actual tool implementation
        // For now, returning a placeholder
        Ok(ToolResult {
            success: true,
            output: format!("Executed tool: {:?}", tool),
            error: None,
            metadata: None,
        })
    }

    async fn get_cached_result(&self, key: &str) -> Option<ToolResult> {
        let cache = self.result_cache.read().await;
        cache.get(key).and_then(|cached| {
            if cached.is_expired() {
                None
            } else {
                Some(cached.result.clone())
            }
        })
    }

    async fn cache_result(&self, key: &str, result: ToolResult, ttl: Duration) {
        let mut cache = self.result_cache.write().await;
        cache.insert(key.to_string(), CachedResult {
            result,
            timestamp: std::time::Instant::now(),
            ttl,
        });
    }

    fn get_tool_name(&self, tool: &AvailableTool) -> String {
        match tool {
            AvailableTool::FileRead { path } => format!("file_read:{}", path),
            AvailableTool::FileWrite { path, .. } => format!("file_write:{}", path),
            AvailableTool::FileSearch { pattern, .. } => format!("file_search:{}", pattern),
            AvailableTool::ExecuteCommand { command, .. } => format!("execute:{}", command),
            AvailableTool::GitStatus { .. } => "git_status".to_string(),
            AvailableTool::WebSearch { query } => format!("web_search:{}", query),
            AvailableTool::WebScrape { url } => format!("web_scrape:{}", url),
            _ => "unknown_tool".to_string(),
        }
    }

    fn estimate_memory_usage(&self) -> usize {
        // Simple estimation - in a real implementation, this would use actual memory profiling
        std::mem::size_of::<Self>() / 1024 / 1024 // Convert to MB
    }
}

impl Clone for AsyncToolExecutor {
    fn clone(&self) -> Self {
        Self {
            execution_semaphore: Arc::clone(&self.execution_semaphore),
            http_client: self.http_client.clone(),
            result_cache: Arc::clone(&self.result_cache),
            resource_limits: self.resource_limits.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub concurrent_executions: usize,
    pub cached_results: usize,
    pub memory_usage_mb: usize,
    pub active_connections: usize,
}

impl ResourceUsage {
    pub fn display(&self) {
        use colored::Colorize;
        
        println!("{}", "Resource Usage:".cyan().bold());
        println!("  Concurrent executions: {}", self.concurrent_executions.to_string().yellow());
        println!("  Cached results: {}", self.cached_results.to_string().yellow());
        println!("  Memory usage: {} MB", self.memory_usage_mb.to_string().yellow());
        println!("  Active connections: {}", self.active_connections.to_string().yellow());
    }
}