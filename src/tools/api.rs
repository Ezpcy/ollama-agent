use super::core::{ApiAuth, HttpMethod, RestOperation, ToolExecutor, ToolResult};
use colored::Colorize;
use std::collections::HashMap;
use std::time::Duration;

impl ToolExecutor {
    pub async fn http_request(
        &self,
        method: HttpMethod,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        timeout_seconds: Option<u64>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Making HTTP {} request to: {}",
            "🌐".cyan(),
            format!("{:?}", method).yellow(),
            url.blue()
        );

        let mut request = match method {
            HttpMethod::GET => self.web_client.get(url),
            HttpMethod::POST => self.web_client.post(url),
            HttpMethod::PUT => self.web_client.put(url),
            HttpMethod::PATCH => self.web_client.patch(url),
            HttpMethod::DELETE => self.web_client.delete(url),
            HttpMethod::HEAD => self.web_client.head(url),
            HttpMethod::OPTIONS => self.web_client.request(reqwest::Method::OPTIONS, url),
        };

        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        // Add body for POST/PUT/PATCH
        if let Some(body) = body {
            request = request.body(body);
        }

        // Set timeout
        let timeout = timeout_seconds.unwrap_or(self.config.default_timeout);
        request = request.timeout(Duration::from_secs(timeout));

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let headers_info = response.headers().clone();

                match response.text().await {
                    Ok(text) => {
                        let metadata = serde_json::json!({
                            "status_code": status.as_u16(),
                            "headers": headers_info.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect::<HashMap<_, _>>()
                        });

                        Ok(ToolResult {
                            success: status.is_success(),
                            output: text,
                            error: if status.is_success() {
                                None
                            } else {
                                Some(format!("HTTP {}", status))
                            },
                            metadata: Some(metadata),
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to read response: {}", e)),
                        metadata: None,
                    }),
                }
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Request failed: {}", e)),
                metadata: None,
            }),
        }
    }

    pub async fn rest_api_call(
        &self,
        endpoint: &str,
        operation: RestOperation,
        _data: Option<serde_json::Value>,
        auth: Option<ApiAuth>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Making REST API call to: {}",
            "🔗".cyan(),
            endpoint.yellow()
        );

        let (method, url, body) = match operation {
            RestOperation::Get => (HttpMethod::GET, endpoint.to_string(), None),
            RestOperation::Create { data } => (
                HttpMethod::POST,
                endpoint.to_string(),
                Some(data.to_string()),
            ),
            RestOperation::Update { id, data } => (
                HttpMethod::PUT,
                format!("{}/{}", endpoint, id),
                Some(data.to_string()),
            ),
            RestOperation::Delete { id } => {
                (HttpMethod::DELETE, format!("{}/{}", endpoint, id), None)
            }
        };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Add authentication
        if let Some(auth) = auth {
            match auth {
                ApiAuth::Bearer { token } => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
                ApiAuth::Basic { username, password } => {
                    let encoded = base64::encode(&format!("{}:{}", username, password));
                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
                ApiAuth::ApiKey { key, header } => {
                    headers.insert(header, key);
                }
            }
        }

        self.http_request(method, &url, Some(headers), body, None)
            .await
    }

    pub async fn graphql_query(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<serde_json::Value>,
        auth: Option<ApiAuth>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Making GraphQL query to: {}",
            "🔍".cyan(),
            endpoint.yellow()
        );

        let mut request_body = serde_json::json!({
            "query": query
        });

        if let Some(variables) = variables {
            request_body["variables"] = variables;
        }

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Add authentication
        if let Some(auth) = auth {
            match auth {
                ApiAuth::Bearer { token } => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
                ApiAuth::Basic { username, password } => {
                    let encoded = base64::encode(&format!("{}:{}", username, password));
                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
                ApiAuth::ApiKey { key, header } => {
                    headers.insert(header, key);
                }
            }
        }

        self.http_request(
            HttpMethod::POST,
            endpoint,
            Some(headers),
            Some(request_body.to_string()),
            None,
        )
        .await
    }
}

// Helper function for base64 encoding (you might want to add the base64 crate to dependencies)
mod base64 {
    pub fn encode(input: &str) -> String {

        // Simple base64 implementation - in production, use the base64 crate
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        let bytes = input.as_bytes();

        for chunk in bytes.chunks(3) {
            let mut buf = [0u8; 3];
            for (i, &b) in chunk.iter().enumerate() {
                buf[i] = b;
            }

            let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);

            result.push(chars.chars().nth(((b >> 18) & 63) as usize).unwrap());
            result.push(chars.chars().nth(((b >> 12) & 63) as usize).unwrap());

            if chunk.len() > 1 {
                result.push(chars.chars().nth(((b >> 6) & 63) as usize).unwrap());
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(chars.chars().nth((b & 63) as usize).unwrap());
            } else {
                result.push('=');
            }
        }

        result
    }
}
