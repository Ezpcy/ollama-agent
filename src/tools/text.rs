use super::core::{TextOperation, ToolExecutor, ToolResult};
use colored::Colorize;
use regex::Regex;

impl ToolExecutor {
    pub fn json_format(&self, input: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Formatting JSON", "📝".cyan());

        match serde_json::from_str::<serde_json::Value>(input) {
            Ok(parsed) => match serde_json::to_string_pretty(&parsed) {
                Ok(formatted) => Ok(ToolResult {
                    success: true,
                    output: formatted,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "operation": "json_format",
                        "original_length": input.len()
                    })),
                }),
                Err(e) => Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to format JSON: {}", e)),
                    metadata: None,
                }),
            },
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
        println!("{} Querying JSON with: {}", "🔍".cyan(), query.yellow());

        // Parse the JSON
        let parsed: serde_json::Value = match serde_json::from_str(input) {
            Ok(val) => val,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid JSON: {}", e)),
                    metadata: None,
                })
            }
        };

        // Simple JSON path implementation
        let result = self.execute_json_path(&parsed, query);

        match result {
            Ok(value) => {
                let output = match serde_json::to_string_pretty(&value) {
                    Ok(formatted) => formatted,
                    Err(_) => format!("{}", value),
                };

                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "query": query,
                        "result_type": self.get_json_type(&value)
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e),
                metadata: None,
            }),
        }
    }

    fn execute_json_path(
        &self,
        json: &serde_json::Value,
        path: &str,
    ) -> Result<serde_json::Value, String> {
        let mut current = json;
        let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();

        for part in parts {
            if part.starts_with('[') && part.ends_with(']') {
                // Array index
                let index_str = &part[1..part.len() - 1];
                match index_str.parse::<usize>() {
                    Ok(index) => {
                        if let Some(array) = current.as_array() {
                            if index < array.len() {
                                current = &array[index];
                            } else {
                                return Err(format!("Array index {} out of bounds", index));
                            }
                        } else {
                            return Err("Cannot index non-array value".to_string());
                        }
                    }
                    Err(_) => return Err(format!("Invalid array index: {}", index_str)),
                }
            } else {
                // Object key
                if let Some(obj) = current.as_object() {
                    if let Some(value) = obj.get(part) {
                        current = value;
                    } else {
                        return Err(format!("Key '{}' not found", part));
                    }
                } else {
                    return Err(format!("Cannot access key '{}' on non-object value", part));
                }
            }
        }

        Ok(current.clone())
    }

    fn get_json_type(&self, value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }

    pub fn csv_parse(
        &self,
        input: &str,
        delimiter: Option<char>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let delim = delimiter.unwrap_or(',');
        println!("{} Parsing CSV with delimiter: '{}'", "📊".cyan(), delim);

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delim as u8)
            .from_reader(input.as_bytes());

        let mut output = Vec::new();
        let mut records = Vec::new();

        // Get headers
        if let Ok(headers) = reader.headers() {
            let header_row: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
            output.push(format!("Headers: {}", header_row.join(" | ")));
            output.push("=".repeat(50));

            // Read records
            for (index, result) in reader.records().enumerate() {
                match result {
                    Ok(record) => {
                        let row: Vec<String> =
                            record.iter().map(|field| field.to_string()).collect();
                        output.push(format!("Row {}: {}", index + 1, row.join(" | ")));
                        records.push(row);

                        // Limit output for large CSVs
                        if index >= 50 {
                            output.push(format!("... ({} more rows)", reader.records().count()));
                            break;
                        }
                    }
                    Err(e) => {
                        output.push(format!("Error reading row {}: {}", index + 1, e));
                    }
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: output.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "delimiter": delim.to_string(),
                "records_count": records.len(),
                "operation": "csv_parse"
            })),
        })
    }

    pub fn regex_match(
        &self,
        pattern: &str,
        text: &str,
        flags: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Matching regex pattern: {}",
            "🔍".cyan(),
            pattern.yellow()
        );

        // Build regex with flags
        let regex_pattern = if let Some(flag_str) = flags {
            if flag_str.contains('i') {
                format!("(?i){}", pattern)
            } else {
                pattern.to_string()
            }
        } else {
            pattern.to_string()
        };

        match Regex::new(&regex_pattern) {
            Ok(re) => {
                let matches: Vec<String> = re
                    .find_iter(text)
                    .enumerate()
                    .map(|(i, m)| {
                        format!(
                            "Match {}: '{}' at position {}-{}",
                            i + 1,
                            m.as_str(),
                            m.start(),
                            m.end()
                        )
                    })
                    .collect();

                if matches.is_empty() {
                    Ok(ToolResult {
                        success: true,
                        output: "No matches found".to_string(),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "pattern": pattern,
                            "matches_count": 0
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: true,
                        output: matches.join("\n"),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "pattern": pattern,
                            "matches_count": matches.len()
                        })),
                    })
                }
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
        println!(
            "{} Applying text transformation: {:?}",
            "🔄".cyan(),
            operation
        );

        let result = match operation {
            TextOperation::ToUpperCase => input.to_uppercase(),
            TextOperation::ToLowerCase => input.to_lowercase(),
            TextOperation::Trim => input.trim().to_string(),
            TextOperation::Count { ref pattern } => {
                let count = input.matches(pattern).count();
                format!("Pattern '{}' found {} times", pattern, count)
            }
            TextOperation::Replace { ref old, ref new } => input.replace(old, new),
            TextOperation::Split { ref delimiter } => {
                let parts: Vec<&str> = input.split(delimiter.as_str()).collect();
                format!(
                    "Split into {} parts:\n{}",
                    parts.len(),
                    parts
                        .iter()
                        .enumerate()
                        .map(|(i, part)| format!("{}: {}", i + 1, part))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            TextOperation::Join { ref delimiter } => {
                // Assume input is newline-separated for joining
                let lines: Vec<&str> = input.lines().collect();
                lines.join(&delimiter)
            }
        };

        Ok(ToolResult {
            success: true,
            output: result,
            error: None,
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "original_length": input.len()
            })),
        })
    }

    // Word count and text statistics
    pub fn text_statistics(&self, input: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Calculating text statistics", "📊".cyan());

        let lines = input.lines().count();
        let words = input.split_whitespace().count();
        let chars = input.chars().count();
        let chars_no_spaces = input.chars().filter(|c| !c.is_whitespace()).count();
        let paragraphs = input.split("\n\n").filter(|p| !p.trim().is_empty()).count();

        // Most common words (simple implementation)
        let mut word_counts = std::collections::HashMap::new();
        for word in input
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.len() > 2)
        // Skip very short words
        {
            *word_counts.entry(word.to_string()).or_insert(0) += 1;
        }

        let mut common_words: Vec<_> = word_counts.into_iter().collect();
        common_words.sort_by(|a, b| b.1.cmp(&a.1));

        let top_words = common_words
            .iter()
            .take(5)
            .map(|(word, count)| format!("'{}': {}", word, count))
            .collect::<Vec<_>>()
            .join(", ");

        let stats = format!(
            "Text Statistics:\n\
            Lines: {}\n\
            Words: {}\n\
            Characters: {}\n\
            Characters (no spaces): {}\n\
            Paragraphs: {}\n\
            Average words per line: {:.1}\n\
            Top words: {}",
            lines,
            words,
            chars,
            chars_no_spaces,
            paragraphs,
            if lines > 0 {
                words as f64 / lines as f64
            } else {
                0.0
            },
            if top_words.is_empty() {
                "None".to_string()
            } else {
                top_words
            }
        );

        Ok(ToolResult {
            success: true,
            output: stats,
            error: None,
            metadata: Some(serde_json::json!({
                "lines": lines,
                "words": words,
                "characters": chars,
                "characters_no_spaces": chars_no_spaces,
                "paragraphs": paragraphs
            })),
        })
    }

    // Hash generation
    pub fn generate_hash(
        &self,
        input: &str,
        algorithm: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Generating {} hash", "🔐".cyan(), algorithm.yellow());

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hash_result = match algorithm.to_lowercase().as_str() {
            "md5" => {
                // Simple MD5 implementation would require additional crate
                format!("MD5 hashing requires additional dependencies")
            }
            "sha256" => {
                // Simple SHA256 implementation would require additional crate
                format!("SHA256 hashing requires additional dependencies")
            }
            "simple" | "default" => {
                let mut hasher = DefaultHasher::new();
                input.hash(&mut hasher);
                format!("{:x}", hasher.finish())
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported hash algorithm: {}", algorithm)),
                    metadata: None,
                });
            }
        };

        Ok(ToolResult {
            success: true,
            output: format!("{} hash: {}", algorithm.to_uppercase(), hash_result),
            error: None,
            metadata: Some(serde_json::json!({
                "algorithm": algorithm,
                "input_length": input.len()
            })),
        })
    }

    // Base64 encoding/decoding
    pub fn base64_encode(&self, input: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Encoding to Base64", "🔄".cyan());

        // Simple base64 implementation (in production, use base64 crate)
        let encoded = base64_simple::encode(input.as_bytes());

        Ok(ToolResult {
            success: true,
            output: encoded,
            error: None,
            metadata: Some(serde_json::json!({
                "operation": "base64_encode",
                "input_length": input.len()
            })),
        })
    }

    pub fn base64_decode(&self, input: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Decoding from Base64", "🔄".cyan());

        match base64_simple::decode(input) {
            Ok(decoded_bytes) => match String::from_utf8(decoded_bytes) {
                Ok(decoded_string) => Ok(ToolResult {
                    success: true,
                    output: decoded_string,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "operation": "base64_decode"
                    })),
                }),
                Err(_) => Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Decoded data is not valid UTF-8".to_string()),
                    metadata: None,
                }),
            },
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid Base64: {}", e)),
                metadata: None,
            }),
        }
    }
}

// Simple base64 implementation module
mod base64_simple {
    pub fn encode(input: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();

        for chunk in input.chunks(3) {
            let mut buf = [0u8; 3];
            for (i, &b) in chunk.iter().enumerate() {
                buf[i] = b;
            }

            let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);

            result.push(CHARS[((b >> 18) & 63) as usize] as char);
            result.push(CHARS[((b >> 12) & 63) as usize] as char);

            if chunk.len() > 1 {
                result.push(CHARS[((b >> 6) & 63) as usize] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(CHARS[(b & 63) as usize] as char);
            } else {
                result.push('=');
            }
        }

        result
    }

    pub fn decode(input: &str) -> Result<Vec<u8>, String> {
        let input = input.trim_end_matches('=');
        let mut result = Vec::new();

        for chunk in input.as_bytes().chunks(4) {
            let mut buf = [0u8; 4];
            for (i, &b) in chunk.iter().enumerate() {
                buf[i] = match b {
                    b'A'..=b'Z' => b - b'A',
                    b'a'..=b'z' => b - b'a' + 26,
                    b'0'..=b'9' => b - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    _ => return Err("Invalid Base64 character".to_string()),
                };
            }

            let combined = ((buf[0] as u32) << 18)
                | ((buf[1] as u32) << 12)
                | ((buf[2] as u32) << 6)
                | (buf[3] as u32);

            result.push((combined >> 16) as u8);
            if chunk.len() > 2 {
                result.push((combined >> 8) as u8);
            }
            if chunk.len() > 3 {
                result.push(combined as u8);
            }
        }

        Ok(result)
    }
}
