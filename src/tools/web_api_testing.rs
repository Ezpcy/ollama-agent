use super::core::{ToolExecutor, ToolResult};
use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::{Client, Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// API testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTestConfig {
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
    pub follow_redirects: bool,
    pub validate_ssl: bool,
    pub user_agent: String,
    pub default_headers: HashMap<String, String>,
}

impl Default for ApiTestConfig {
    fn default() -> Self {
        let mut default_headers = HashMap::new();
        default_headers.insert("Accept".to_string(), "application/json".to_string());
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());

        Self {
            timeout_seconds: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            follow_redirects: true,
            validate_ssl: true,
            user_agent: "API Testing Tool 1.0".to_string(),
            default_headers,
        }
    }
}

/// Comprehensive API test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTestResult {
    pub test_name: String,
    pub method: String,
    pub url: String,
    pub status_code: u16,
    pub success: bool,
    pub response_time_ms: u64,
    pub response_size_bytes: usize,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
    pub response_body: String,
    pub error_message: Option<String>,
    pub performance_metrics: ApiPerformanceMetrics,
    pub validation_results: Vec<ValidationResult>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// API performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPerformanceMetrics {
    pub dns_lookup_time_ms: Option<u64>,
    pub tcp_connect_time_ms: Option<u64>,
    pub tls_handshake_time_ms: Option<u64>,
    pub first_byte_time_ms: Option<u64>,
    pub download_time_ms: u64,
    pub total_time_ms: u64,
    pub redirects_count: u32,
}

/// Validation result for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validation_type: ValidationType,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub message: String,
}

/// Types of API validations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    StatusCode,
    ResponseTime,
    HeaderPresent,
    HeaderValue,
    JsonSchema,
    JsonPath,
    ContentType,
    ResponseSize,
    Custom,
}

/// API test suite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTestSuite {
    pub name: String,
    pub base_url: String,
    pub global_headers: HashMap<String, String>,
    pub tests: Vec<ApiTest>,
    pub setup_requests: Vec<ApiTest>,
    pub teardown_requests: Vec<ApiTest>,
}

/// Individual API test definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTest {
    pub name: String,
    pub method: String,
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub query_params: HashMap<String, String>,
    pub validations: Vec<ApiValidation>,
    pub depends_on: Vec<String>,
    pub extract_variables: HashMap<String, String>,
}

/// API validation definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiValidation {
    pub validation_type: ValidationType,
    pub expected_value: String,
    pub json_path: Option<String>,
    pub header_name: Option<String>,
    pub custom_script: Option<String>,
}

impl ToolExecutor {
    /// Perform comprehensive API testing
    pub async fn api_test_comprehensive(
        &self,
        url: &str,
        method: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        validations: Option<Vec<ApiValidation>>,
        config: Option<ApiTestConfig>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_default();
        let method = method.to_uppercase();
        
        println!("{} API Testing: {} {}", "🧪".cyan(), method.yellow(), url);

        let start_time = Instant::now();
        
        // Build request
        let client = self.build_api_test_client(&config)?;
        let mut request_headers = config.default_headers.clone();
        if let Some(additional_headers) = headers {
            request_headers.extend(additional_headers);
        }

        // Execute request with performance tracking
        let test_result = self.execute_api_test(
            &client,
            &method,
            url,
            request_headers,
            body,
            validations.unwrap_or_default(),
            &config,
            start_time,
        ).await?;

        Ok(ToolResult {
            success: test_result.success,
            output: self.format_api_test_result(&test_result),
            error: test_result.error_message.clone(),
            metadata: Some(serde_json::to_value(&test_result)?),
        })
    }

    /// Execute API test with comprehensive metrics
    async fn execute_api_test(
        &self,
        client: &Client,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
        validations: Vec<ApiValidation>,
        config: &ApiTestConfig,
        start_time: Instant,
    ) -> Result<ApiTestResult> {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < config.retry_attempts {
            match self.try_api_request(client, method, url, &headers, &body, config).await {
                Ok((response, response_time)) => {
                    let response_headers: HashMap<String, String> = response
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect();

                    let status_code = response.status().as_u16();
                    let response_body = response.text().await.unwrap_or_default();
                    let response_size = response_body.len();

                    // Perform validations
                    let validation_results = self.perform_validations(
                        &validations,
                        status_code,
                        &response_headers,
                        &response_body,
                        response_time.as_millis() as u64,
                    );

                    let success = validation_results.iter().all(|v| v.passed) && status_code < 400;

                    let performance_metrics = ApiPerformanceMetrics {
                        dns_lookup_time_ms: None,
                        tcp_connect_time_ms: None,
                        tls_handshake_time_ms: None,
                        first_byte_time_ms: None,
                        download_time_ms: response_time.as_millis() as u64,
                        total_time_ms: start_time.elapsed().as_millis() as u64,
                        redirects_count: 0,
                    };

                    return Ok(ApiTestResult {
                        test_name: "API Test".to_string(),
                        method: method.to_string(),
                        url: url.to_string(),
                        status_code,
                        success,
                        response_time_ms: response_time.as_millis() as u64,
                        response_size_bytes: response_size,
                        request_headers: headers,
                        response_headers,
                        response_body,
                        error_message: None,
                        performance_metrics,
                        validation_results,
                        timestamp: chrono::Utc::now(),
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;
                    
                    if attempt < config.retry_attempts {
                        println!("{} Attempt {} failed, retrying...", "⚠".yellow(), attempt);
                        tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
                    }
                }
            }
        }

        // All attempts failed
        let error_message = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        Ok(ApiTestResult {
            test_name: "API Test".to_string(),
            method: method.to_string(),
            url: url.to_string(),
            status_code: 0,
            success: false,
            response_time_ms: 0,
            response_size_bytes: 0,
            request_headers: headers,
            response_headers: HashMap::new(),
            response_body: String::new(),
            error_message: Some(error_message),
            performance_metrics: ApiPerformanceMetrics {
                dns_lookup_time_ms: None,
                tcp_connect_time_ms: None,
                tls_handshake_time_ms: None,
                first_byte_time_ms: None,
                download_time_ms: 0,
                total_time_ms: start_time.elapsed().as_millis() as u64,
                redirects_count: 0,
            },
            validation_results: vec![],
            timestamp: chrono::Utc::now(),
        })
    }

    /// Attempt single API request
    async fn try_api_request(
        &self,
        client: &Client,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &Option<String>,
        config: &ApiTestConfig,
    ) -> Result<(Response, Duration)> {
        let request_start = Instant::now();
        
        let method = Method::from_bytes(method.as_bytes())
            .map_err(|_| anyhow!("Invalid HTTP method: {}", method))?;

        let mut request = client.request(method, url);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body_content) = body {
            request = request.body(body_content.clone());
        }

        let response = timeout(
            Duration::from_secs(config.timeout_seconds),
            request.send()
        ).await??;

        let response_time = request_start.elapsed();
        Ok((response, response_time))
    }

    /// Build HTTP client for API testing
    fn build_api_test_client(&self, config: &ApiTestConfig) -> Result<Client> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent);

        if config.follow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::limited(10));
        } else {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }

        if !config.validate_ssl {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        client_builder.build().map_err(|e| anyhow!("Failed to build HTTP client: {}", e))
    }

    /// Perform comprehensive validations on API response
    fn perform_validations(
        &self,
        validations: &[ApiValidation],
        status_code: u16,
        headers: &HashMap<String, String>,
        body: &str,
        response_time_ms: u64,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for validation in validations {
            let result = match validation.validation_type {
                ValidationType::StatusCode => self.validate_status_code(status_code, &validation.expected_value),
                ValidationType::ResponseTime => self.validate_response_time(response_time_ms, &validation.expected_value),
                ValidationType::HeaderPresent => self.validate_header_present(headers, &validation.header_name.as_ref().unwrap_or(&String::new())),
                ValidationType::HeaderValue => self.validate_header_value(headers, &validation.header_name.as_ref().unwrap_or(&String::new()), &validation.expected_value),
                ValidationType::JsonSchema => self.validate_json_schema(body, &validation.expected_value),
                ValidationType::JsonPath => self.validate_json_path(body, &validation.json_path.as_ref().unwrap_or(&String::new()), &validation.expected_value),
                ValidationType::ContentType => self.validate_content_type(headers, &validation.expected_value),
                ValidationType::ResponseSize => self.validate_response_size(body.len(), &validation.expected_value),
                ValidationType::Custom => self.validate_custom(body, &validation.custom_script.as_ref().unwrap_or(&String::new())),
            };

            results.push(result);
        }

        results
    }

    /// Validate HTTP status code
    fn validate_status_code(&self, actual: u16, expected: &str) -> ValidationResult {
        let expected_code: u16 = expected.parse().unwrap_or(200);
        let passed = actual == expected_code;

        ValidationResult {
            validation_type: ValidationType::StatusCode,
            passed,
            expected: expected.to_string(),
            actual: actual.to_string(),
            message: if passed {
                "Status code matches expected value".to_string()
            } else {
                format!("Expected status code {}, got {}", expected_code, actual)
            },
        }
    }

    /// Validate response time
    fn validate_response_time(&self, actual_ms: u64, expected: &str) -> ValidationResult {
        let expected_ms: u64 = expected.parse().unwrap_or(5000);
        let passed = actual_ms <= expected_ms;

        ValidationResult {
            validation_type: ValidationType::ResponseTime,
            passed,
            expected: format!("≤ {}ms", expected_ms),
            actual: format!("{}ms", actual_ms),
            message: if passed {
                "Response time within acceptable range".to_string()
            } else {
                format!("Response time {}ms exceeds limit of {}ms", actual_ms, expected_ms)
            },
        }
    }

    /// Validate header presence
    fn validate_header_present(&self, headers: &HashMap<String, String>, header_name: &str) -> ValidationResult {
        let passed = headers.contains_key(header_name);

        ValidationResult {
            validation_type: ValidationType::HeaderPresent,
            passed,
            expected: format!("Header '{}' present", header_name),
            actual: if passed { "Present" } else { "Missing" }.to_string(),
            message: if passed {
                format!("Header '{}' is present", header_name)
            } else {
                format!("Header '{}' is missing", header_name)
            },
        }
    }

    /// Validate header value
    fn validate_header_value(&self, headers: &HashMap<String, String>, header_name: &str, expected: &str) -> ValidationResult {
        let actual = headers.get(header_name).cloned().unwrap_or_default();
        let passed = actual == expected;

        ValidationResult {
            validation_type: ValidationType::HeaderValue,
            passed,
            expected: expected.to_string(),
            actual: actual.clone(),
            message: if passed {
                format!("Header '{}' has expected value", header_name)
            } else {
                format!("Header '{}' value mismatch: expected '{}', got '{}'", header_name, expected, actual)
            },
        }
    }

    /// Validate JSON schema (simplified)
    fn validate_json_schema(&self, body: &str, _schema: &str) -> ValidationResult {
        let passed = serde_json::from_str::<serde_json::Value>(body).is_ok();

        ValidationResult {
            validation_type: ValidationType::JsonSchema,
            passed,
            expected: "Valid JSON".to_string(),
            actual: if passed { "Valid JSON" } else { "Invalid JSON" }.to_string(),
            message: if passed {
                "Response contains valid JSON".to_string()
            } else {
                "Response does not contain valid JSON".to_string()
            },
        }
    }

    /// Validate JSON path value (simplified)
    fn validate_json_path(&self, body: &str, path: &str, expected: &str) -> ValidationResult {
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(json) => {
                // Simple JSON path evaluation (for demonstration)
                let actual = self.simple_json_path_eval(&json, path);
                let passed = actual == expected;

                ValidationResult {
                    validation_type: ValidationType::JsonPath,
                    passed,
                    expected: expected.to_string(),
                    actual: actual.clone(),
                    message: if passed {
                        format!("JSON path '{}' has expected value", path)
                    } else {
                        format!("JSON path '{}' value mismatch: expected '{}', got '{}'", path, expected, actual)
                    },
                }
            }
            Err(_) => ValidationResult {
                validation_type: ValidationType::JsonPath,
                passed: false,
                expected: expected.to_string(),
                actual: "Invalid JSON".to_string(),
                message: "Cannot evaluate JSON path on invalid JSON".to_string(),
            },
        }
    }

    /// Simple JSON path evaluation
    fn simple_json_path_eval(&self, json: &serde_json::Value, path: &str) -> String {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            if part.is_empty() {
                continue;
            }

            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return "null".to_string();
                }
            } else {
                return "null".to_string();
            }
        }

        current.to_string()
    }

    /// Validate content type
    fn validate_content_type(&self, headers: &HashMap<String, String>, expected: &str) -> ValidationResult {
        let actual = headers.get("content-type")
            .or_else(|| headers.get("Content-Type"))
            .cloned()
            .unwrap_or_default();
        
        let passed = actual.contains(expected);

        ValidationResult {
            validation_type: ValidationType::ContentType,
            passed,
            expected: expected.to_string(),
            actual: actual.clone(),
            message: if passed {
                "Content-Type matches expected value".to_string()
            } else {
                format!("Content-Type mismatch: expected '{}', got '{}'", expected, actual)
            },
        }
    }

    /// Validate response size
    fn validate_response_size(&self, actual_size: usize, expected: &str) -> ValidationResult {
        let expected_size: usize = expected.parse().unwrap_or(0);
        let passed = actual_size >= expected_size;

        ValidationResult {
            validation_type: ValidationType::ResponseSize,
            passed,
            expected: format!("≥ {} bytes", expected_size),
            actual: format!("{} bytes", actual_size),
            message: if passed {
                "Response size meets minimum requirement".to_string()
            } else {
                format!("Response size {} bytes is less than minimum {}", actual_size, expected_size)
            },
        }
    }

    /// Custom validation (placeholder)
    fn validate_custom(&self, _body: &str, _script: &str) -> ValidationResult {
        ValidationResult {
            validation_type: ValidationType::Custom,
            passed: true,
            expected: "Custom validation".to_string(),
            actual: "Passed".to_string(),
            message: "Custom validation not implemented".to_string(),
        }
    }

    /// Format API test result for display
    fn format_api_test_result(&self, result: &ApiTestResult) -> String {
        let mut output = Vec::new();

        output.push(format!("🧪 API Test Result: {}", result.test_name));
        output.push("=".repeat(50));

        output.push(format!("🔗 Request: {} {}", result.method, result.url));
        output.push(format!("📊 Status: {} {}", 
            if result.success { "✅" } else { "❌" },
            result.status_code
        ));

        output.push(format!("⏱️  Response Time: {}ms", result.response_time_ms));
        output.push(format!("📦 Response Size: {} bytes", result.response_size_bytes));

        if let Some(error) = &result.error_message {
            output.push(format!("❌ Error: {}", error));
        }

        // Validation results
        if !result.validation_results.is_empty() {
            output.push("\n🔍 Validation Results:".to_string());
            for (i, validation) in result.validation_results.iter().enumerate() {
                let status = if validation.passed { "✅" } else { "❌" };
                output.push(format!("   {}. {} {:?}: {}", 
                    i + 1, 
                    status, 
                    validation.validation_type,
                    validation.message
                ));
            }
        }

        // Performance metrics
        output.push("\n📈 Performance:".to_string());
        output.push(format!("   • Total Time: {}ms", result.performance_metrics.total_time_ms));
        output.push(format!("   • Download Time: {}ms", result.performance_metrics.download_time_ms));

        // Response headers (limited)
        if !result.response_headers.is_empty() {
            output.push("\n📋 Key Response Headers:".to_string());
            for (key, value) in result.response_headers.iter().take(5) {
                output.push(format!("   • {}: {}", key, value));
            }
        }

        // Response body preview
        if !result.response_body.is_empty() {
            output.push("\n📄 Response Body Preview:".to_string());
            let preview = if result.response_body.len() > 500 {
                format!("{}...", result.response_body.chars().take(500).collect::<String>())
            } else {
                result.response_body.clone()
            };
            output.push(preview);
        }

        output.join("\n")
    }

    /// Run API load test
    pub async fn api_load_test(
        &self,
        url: &str,
        method: &str,
        concurrent_requests: usize,
        total_requests: usize,
        config: Option<ApiTestConfig>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_default();
        
        println!("{} API Load Testing: {} concurrent requests, {} total", 
            "⚡".cyan(), 
            concurrent_requests, 
            total_requests
        );

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrent_requests));
        let client = self.build_api_test_client(&config)?;
        
        let start_time = Instant::now();
        let mut tasks = Vec::new();

        for i in 0..total_requests {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let url = url.to_string();
            let method = method.to_string();
            let config = config.clone();

            let task = tokio::spawn(async move {
                let _permit = permit;
                let request_start = Instant::now();
                
                let result = timeout(
                    Duration::from_secs(config.timeout_seconds),
                    client.request(
                        Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET),
                        &url
                    ).send()
                ).await;

                match result {
                    Ok(Ok(response)) => {
                        let status = response.status().as_u16();
                        let response_time = request_start.elapsed();
                        (i, true, status, response_time.as_millis() as u64, None)
                    }
                    Ok(Err(e)) => {
                        (i, false, 0, request_start.elapsed().as_millis() as u64, Some(e.to_string()))
                    }
                    Err(_) => {
                        (i, false, 0, config.timeout_seconds * 1000, Some("Timeout".to_string()))
                    }
                }
            });

            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(result) => results.push(result),
                Err(e) => println!("{} Task failed: {}", "❌".red(), e),
            }
        }

        let total_time = start_time.elapsed();
        let summary = self.create_load_test_summary(results, total_time, concurrent_requests, total_requests);

        Ok(ToolResult {
            success: true,
            output: summary,
            error: None,
            metadata: None,
        })
    }

    /// Create load test summary
    fn create_load_test_summary(
        &self,
        results: Vec<(usize, bool, u16, u64, Option<String>)>,
        total_time: Duration,
        concurrent_requests: usize,
        total_requests: usize,
    ) -> String {
        let mut output = Vec::new();

        output.push(format!("⚡ API Load Test Summary"));
        output.push("=".repeat(50));

        let successful = results.iter().filter(|(_, success, _, _, _)| *success).count();
        let failed = results.len() - successful;

        output.push(format!("Total Requests: {}", total_requests));
        output.push(format!("Concurrent: {}", concurrent_requests));
        output.push(format!("Successful: {} ({:.1}%)", successful, (successful as f64 / total_requests as f64) * 100.0));
        output.push(format!("Failed: {} ({:.1}%)", failed, (failed as f64 / total_requests as f64) * 100.0));

        output.push(format!("Total Time: {:.2}s", total_time.as_secs_f64()));
        output.push(format!("Requests/sec: {:.2}", total_requests as f64 / total_time.as_secs_f64()));

        // Response time statistics
        let response_times: Vec<u64> = results.iter().map(|(_, _, _, time, _)| *time).collect();
        if !response_times.is_empty() {
            let avg_time = response_times.iter().sum::<u64>() as f64 / response_times.len() as f64;
            let min_time = *response_times.iter().min().unwrap();
            let max_time = *response_times.iter().max().unwrap();

            output.push(format!("Response Times:"));
            output.push(format!("   • Average: {:.2}ms", avg_time));
            output.push(format!("   • Min: {}ms", min_time));
            output.push(format!("   • Max: {}ms", max_time));
        }

        // Status code breakdown
        let mut status_codes = HashMap::new();
        for (_, success, status, _, _) in &results {
            if *success {
                *status_codes.entry(*status).or_insert(0) += 1;
            }
        }

        if !status_codes.is_empty() {
            output.push("Status Code Distribution:".to_string());
            for (status, count) in status_codes {
                output.push(format!("   • {}: {}", status, count));
            }
        }

        output.join("\n")
    }
}