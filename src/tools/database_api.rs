use super::core::{
    ApiAuth, DatabaseType, HttpMethod, RestOperation, TextOperation, ToolExecutor, ToolResult,
};
use colored::Colorize;
use regex::Regex;
use serde_json::Value;
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
            "{} HTTP {:?} request to: {}",
            "🌐".cyan(),
            method,
            url.yellow()
        );

        let timeout = Duration::from_secs(timeout_seconds.unwrap_or(30));

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

        // Add body for applicable methods
        if let Some(body) = body {
            request = request.body(body);
        }

        // Set timeout
        request = request.timeout(timeout);

        let response = request.send().await?;
        let status = response.status();
        let headers_map: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response.text().await?;

        let success = status.is_success();
        let output = format!(
            "Status: {}\nHeaders: {}\nBody: {}",
            status,
            serde_json::to_string_pretty(&headers_map)?,
            body
        );

        Ok(ToolResult {
            success,
            output,
            error: if success {
                None
            } else {
                Some(format!("HTTP error: {}", status))
            },
            metadata: Some(serde_json::json!({
                "method": format!("{:?}", method),
                "url": url,
                "status": status.as_u16(),
                "headers": headers_map
            })),
        })
    }

    pub async fn rest_api_call(
        &self,
        endpoint: &str,
        operation: RestOperation,
        data: Option<Value>,
        auth: Option<ApiAuth>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} REST API call: {:?} to {}",
            "🔗".cyan(),
            operation,
            endpoint.yellow()
        );

        let mut request = match operation {
            RestOperation::Get => self.web_client.get(endpoint),
            RestOperation::Create { .. } => self.web_client.post(endpoint),
            RestOperation::Update { ref id, .. } => {
                let url = format!("{}/{}", endpoint, id);
                self.web_client.put(&url)
            }
            RestOperation::Delete { ref id } => {
                let url = format!("{}/{}", endpoint, id);
                self.web_client.delete(&url)
            }
        };

        // Add authentication
        if let Some(auth) = auth {
            request = match auth {
                ApiAuth::Bearer { token } => request.bearer_auth(token),
                ApiAuth::Basic { username, password } => {
                    request.basic_auth(username, Some(password))
                }
                ApiAuth::ApiKey { key, header } => request.header(header, key),
            };
        }

        // Add data for applicable operations
        match operation {
            RestOperation::Create { ref data } => {
                request = request.json(data);
            }
            RestOperation::Update { ref data, .. } => {
                request = request.json(data);
            }
            _ => {}
        }

        // Add default headers
        request = request.header("Content-Type", "application/json");

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;

        let success = status.is_success();

        Ok(ToolResult {
            success,
            output: format!("Status: {}\nResponse: {}", status, body),
            error: if success {
                None
            } else {
                Some(format!("REST API error: {}", status))
            },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "endpoint": endpoint,
                "status": status.as_u16()
            })),
        })
    }

    pub async fn graphql_query(
        &self,
        endpoint: &str,
        query: &str,
        variables: Option<Value>,
        auth: Option<ApiAuth>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} GraphQL query to: {}", "🔍".cyan(), endpoint.yellow());

        let mut request = self.web_client.post(endpoint);

        // Add authentication
        if let Some(auth) = auth {
            request = match auth {
                ApiAuth::Bearer { token } => request.bearer_auth(token),
                ApiAuth::Basic { username, password } => {
                    request.basic_auth(username, Some(password))
                }
                ApiAuth::ApiKey { key, header } => request.header(header, key),
            };
        }

        // Build GraphQL request body
        let mut request_body = serde_json::json!({
            "query": query
        });

        if let Some(vars) = variables {
            request_body["variables"] = vars;
        }

        request = request.json(&request_body);

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;

        let success = status.is_success();

        Ok(ToolResult {
            success,
            output: format!("Status: {}\nResponse: {}", status, body),
            error: if success {
                None
            } else {
                Some(format!("GraphQL error: {}", status))
            },
            metadata: Some(serde_json::json!({
                "endpoint": endpoint,
                "status": status.as_u16(),
                "query_length": query.len()
            })),
        })
    }

    pub async fn sql_query(
        &self,
        connection_string: &str,
        query: &str,
        database_type: DatabaseType,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} SQL query ({:?}): {}",
            "🗃️".cyan(),
            database_type,
            query.yellow()
        );

        // This is a simplified implementation
        // In a real implementation, you would use actual database drivers

        match database_type {
            DatabaseType::SQLite => {
                // For SQLite, we can use the sqlite_query method
                let db_path = connection_string
                    .trim_start_matches("sqlite://")
                    .trim_start_matches("sqlite:");
                self.sqlite_query(db_path, query).await
            }
            // DatabaseType::PostgreSQL => {
            //     // PostgreSQL implementation using psql command line tool
            //
            //     self.execute_sql_command("psql", connection_string, query, "PostgreSQL")
            //         .await
            // }
            // DatabaseType::MySQL => {
            //     // MySQL implementation using mysql command line tool
            //     self.execute_sql_command("mysql", connection_string, query, "MySQL")
            //         .await
            // }
            DatabaseType::MongoDB => {
                // Placeholder for MongoDB implementation
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("MongoDB connections not yet implemented. Use external tools or connection libraries.".to_string()),
                    metadata: Some(serde_json::json!({
                        "database_type": "MongoDB",
                        "connection_string": connection_string,
                        "query": query
                    })),
                })
            }
        }
    }

    pub async fn sqlite_query(
        &self,
        database_path: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} SQLite query in {}: {}",
            "🗄️".cyan(),
            database_path.yellow(),
            query.yellow()
        );

        // Use sqlite3 command line tool
        let output = std::process::Command::new("sqlite3")
            .arg(database_path)
            .arg(query)
            .output()?;

        let success = output.status.success();

        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success {
                None
            } else {
                Some("SQLite query failed".to_string())
            },
            metadata: Some(serde_json::json!({
                "database_path": database_path,
                "query": query,
                "query_type": self.detect_sql_query_type(query)
            })),
        })
    }

    fn detect_sql_query_type(&self, query: &str) -> &'static str {
        let query_upper = query.trim().to_uppercase();

        if query_upper.starts_with("SELECT") {
            "SELECT"
        } else if query_upper.starts_with("INSERT") {
            "INSERT"
        } else if query_upper.starts_with("UPDATE") {
            "UPDATE"
        } else if query_upper.starts_with("DELETE") {
            "DELETE"
        } else if query_upper.starts_with("CREATE") {
            "CREATE"
        } else if query_upper.starts_with("DROP") {
            "DROP"
        } else if query_upper.starts_with("ALTER") {
            "ALTER"
        } else {
            "OTHER"
        }
    }

    // Text processing tools
    pub fn json_format(&self, input: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Formatting JSON", "📝".cyan());

        match serde_json::from_str::<Value>(input) {
            Ok(json) => {
                let formatted = serde_json::to_string_pretty(&json)?;
                let output_length = formatted.len();
                Ok(ToolResult {
                    success: true,
                    output: formatted,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "input_length": input.len(),
                        "output_length": output_length
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid JSON: {}", e)),
                metadata: None,
            }),
        }
    }

    pub fn json_query(
        &self,
        input: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} JSON query: {}", "🔍".cyan(), query.yellow());

        match serde_json::from_str::<Value>(input) {
            Ok(json) => {
                let result = self.execute_json_query(&json, query)?;
                let output = serde_json::to_string_pretty(&result)?;

                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "query": query,
                        "result_type": match result {
                            Value::Array(_) => "array",
                            Value::Object(_) => "object",
                            Value::String(_) => "string",
                            Value::Number(_) => "number",
                            Value::Bool(_) => "boolean",
                            Value::Null => "null",
                        }
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid JSON: {}", e)),
                metadata: None,
            }),
        }
    }

    fn execute_json_query(
        &self,
        json: &Value,
        query: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        // Simple JSONPath-like query implementation
        let mut current = json;

        for part in query.split('.') {
            if part.is_empty() {
                continue;
            }

            if part.starts_with('[') && part.ends_with(']') {
                // Array index
                let index_str = &part[1..part.len() - 1];
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(array) = current.as_array() {
                        if index < array.len() {
                            current = &array[index];
                        } else {
                            return Ok(Value::Null);
                        }
                    } else {
                        return Ok(Value::Null);
                    }
                } else {
                    return Ok(Value::Null);
                }
            } else {
                // Object key
                if let Some(obj) = current.as_object() {
                    if let Some(value) = obj.get(part) {
                        current = value;
                    } else {
                        return Ok(Value::Null);
                    }
                } else {
                    return Ok(Value::Null);
                }
            }
        }

        Ok(current.clone())
    }

    pub fn csv_parse(
        &self,
        input: &str,
        delimiter: Option<char>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let delimiter = delimiter.unwrap_or(',');
        println!(
            "{} Parsing CSV with delimiter: '{}'",
            "📊".cyan(),
            delimiter
        );

        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Empty input".to_string()),
                metadata: None,
            });
        }

        let mut result = Vec::new();
        let mut header = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let fields: Vec<&str> = line.split(delimiter).collect();

            if i == 0 {
                // First line is header
                header = fields.iter().map(|f| f.trim().to_string()).collect();
                result.push(serde_json::json!({
                    "type": "header",
                    "fields": header
                }));
            } else {
                // Data rows
                let mut row = serde_json::Map::new();
                for (j, field) in fields.iter().enumerate() {
                    let key = header.get(j).map(|h| h.as_str()).unwrap_or("unknown");
                    let column_name = if key == "unknown" {
                        format!("column_{}", j)
                    } else {
                        key.to_string()
                    };
                    let value = field.trim();

                    // Try to parse as number
                    if let Ok(num) = value.parse::<f64>() {
                        row.insert(column_name, serde_json::json!(num));
                    } else {
                        row.insert(column_name, serde_json::json!(value));
                    }
                }
                result.push(serde_json::json!(row));
            }
        }

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&result)?,
            error: None,
            metadata: Some(serde_json::json!({
                "rows": lines.len(),
                "columns": header.len(),
                "delimiter": delimiter
            })),
        })
    }

    pub fn regex_match(
        &self,
        pattern: &str,
        text: &str,
        flags: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Regex match: {}", "🔍".cyan(), pattern.yellow());

        let mut regex_builder = regex::RegexBuilder::new(pattern);

        if let Some(flags) = flags {
            if flags.contains('i') {
                regex_builder.case_insensitive(true);
            }
            if flags.contains('m') {
                regex_builder.multi_line(true);
            }
            if flags.contains('s') {
                regex_builder.dot_matches_new_line(true);
            }
        }

        match regex_builder.build() {
            Ok(regex) => {
                let matches: Vec<_> = regex.find_iter(text).collect();

                let mut result = Vec::new();
                for (i, m) in matches.iter().enumerate() {
                    result.push(serde_json::json!({
                        "match": i,
                        "text": m.as_str(),
                        "start": m.start(),
                        "end": m.end()
                    }));
                }

                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&result)?,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "pattern": pattern,
                        "matches_count": matches.len(),
                        "flags": flags
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid regex pattern: {}", e)),
                metadata: None,
            }),
        }
    }

    pub fn text_transform(
        &self,
        input: &str,
        operation: TextOperation,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Text transform: {:?}", "🔄".cyan(), operation);

        let result = match &operation {
            TextOperation::ToUpperCase => input.to_uppercase(),
            TextOperation::ToLowerCase => input.to_lowercase(),
            TextOperation::Trim => input.trim().to_string(),
            TextOperation::Count { pattern } => {
                let count = input.matches(pattern).count();
                count.to_string()
            }
            TextOperation::Replace { old, new } => input.replace(old, new),
            TextOperation::Split { delimiter } => {
                let parts: Vec<&str> = input.split(delimiter).collect();
                serde_json::to_string_pretty(&parts)?
            }
            TextOperation::Join { delimiter } => {
                // Assume input is JSON array of strings
                match serde_json::from_str::<Vec<String>>(input) {
                    Ok(parts) => parts.join(delimiter),
                    Err(_) => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Input must be a JSON array of strings for join operation"
                                    .to_string(),
                            ),
                            metadata: None,
                        });
                    }
                }
            }
        };

        let output_length = result.len();
        Ok(ToolResult {
            success: true,
            output: result,
            error: None,
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "input_length": input.len(),
                "output_length": output_length
            })),
        })
    }
}

