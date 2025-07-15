use super::core::{ToolExecutor, ToolResult};
use colored::Colorize;
use std::fs;
use std::process::Command;

impl ToolExecutor {
    pub async fn system_info(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Gathering system information", "💻".cyan());

        let mut info = Vec::new();

        // Operating System
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        info.push(format!("Operating System: {} ({})", os, arch));

        // Hostname
        if let Ok(hostname) = hostname::get() {
            info.push(format!("Hostname: {}", hostname.to_string_lossy()));
        }

        // CPU Information
        if let Ok(output) = self.get_cpu_info().await {
            info.push(format!("CPU: {}", output));
        }

        // Memory Information
        if let Ok(memory_info) = self.get_memory_info().await {
            info.push(memory_info);
        }

        // Disk Information
        if let Ok(disk_info) = self.get_disk_info().await {
            info.push(disk_info);
        }

        // Uptime
        if let Ok(uptime) = self.get_uptime().await {
            info.push(format!("Uptime: {}", uptime));
        }

        // Load Average (Unix-like systems)
        #[cfg(unix)]
        if let Ok(load_avg) = self.get_load_average().await {
            info.push(format!("Load Average: {}", load_avg));
        }

        Ok(ToolResult {
            success: true,
            output: info.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "os": os,
                "arch": arch,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        })
    }

    async fn get_cpu_info(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
                if let Some(model_line) =
                    cpuinfo.lines().find(|line| line.starts_with("model name"))
                {
                    if let Some(model) = model_line.split(':').nth(1) {
                        return Ok(model.trim().to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()?;
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wmic")
                .args(["cpu", "get", "name", "/value"])
                .output()?;
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = text.lines().find(|line| line.starts_with("Name=")) {
                    return Ok(line.strip_prefix("Name=").unwrap_or("Unknown").to_string());
                }
            }
        }

        Ok("Unknown CPU".to_string())
    }

    async fn get_memory_info(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                let mut total_kb = 0;
                let mut available_kb = 0;

                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        total_kb = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("MemAvailable:") {
                        available_kb = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);
                    }
                }

                let total_gb = total_kb as f64 / 1024.0 / 1024.0;
                let available_gb = available_kb as f64 / 1024.0 / 1024.0;
                let used_gb = total_gb - available_gb;
                let usage_percent = (used_gb / total_gb) * 100.0;

                return Ok(format!(
                    "Memory: {:.1} GB used / {:.1} GB total ({:.1}% used)",
                    used_gb, total_gb, usage_percent
                ));
            }
        }

        #[cfg(target_os = "macos")]
        {
            let vm_stat = Command::new("vm_stat").output()?;
            if vm_stat.status.success() {
                // Parse vm_stat output (simplified)
                return Ok("Memory information available via vm_stat".to_string());
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wmic")
                .args([
                    "OS",
                    "get",
                    "TotalVisibleMemorySize,FreePhysicalMemory",
                    "/value",
                ])
                .output()?;
            if output.status.success() {
                // Parse Windows memory info
                return Ok("Memory information from Windows WMI".to_string());
            }
        }

        Ok("Memory information not available".to_string())
    }

    async fn get_disk_info(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(unix)]
        {
            let output = Command::new("df").args(["-h", "/"]).output()?;
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = text.lines().nth(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        return Ok(format!(
                            "Root Disk: {} used / {} total ({}% used)",
                            parts[2], parts[1], parts[4]
                        ));
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wmic")
                .args([
                    "logicaldisk",
                    "where",
                    "size>0",
                    "get",
                    "size,freespace,caption",
                    "/value",
                ])
                .output()?;
            if output.status.success() {
                return Ok("Disk information from Windows WMI".to_string());
            }
        }

        Ok("Disk information not available".to_string())
    }

    async fn get_uptime(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
                if let Some(uptime_seconds_str) = uptime_str.split_whitespace().next() {
                    if let Ok(uptime_seconds) = uptime_seconds_str.parse::<f64>() {
                        let days = (uptime_seconds / 86400.0) as u64;
                        let hours = ((uptime_seconds % 86400.0) / 3600.0) as u64;
                        let minutes = ((uptime_seconds % 3600.0) / 60.0) as u64;

                        return Ok(format!(
                            "{} days, {} hours, {} minutes",
                            days, hours, minutes
                        ));
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("uptime").output()?;
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("wmic")
                .args(["os", "get", "lastbootuptime", "/value"])
                .output()?;
            if output.status.success() {
                return Ok("Uptime information from Windows WMI".to_string());
            }
        }

        Ok("Uptime information not available".to_string())
    }

    #[cfg(unix)]
    async fn get_load_average(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
                let parts: Vec<&str> = loadavg.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Ok(format!(
                        "{} {} {} (1m 5m 15m)",
                        parts[0], parts[1], parts[2]
                    ));
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("uptime").output()?;
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(load_part) = text.split("load averages:").nth(1) {
                    return Ok(format!("load averages:{}", load_part.trim()));
                }
            }
        }

        Ok("Load average not available".to_string())
    }

    pub async fn memory_usage(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Checking memory usage", "🧠".cyan());

        let memory_info = self.get_memory_info().await?;

        Ok(ToolResult {
            success: true,
            output: memory_info,
            error: None,
            metadata: Some(serde_json::json!({
                "type": "memory_usage",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        })
    }

    pub async fn disk_usage(
        &self,
        path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let check_path = path.unwrap_or(".");
        println!(
            "{} Checking disk usage for: {}",
            "💾".cyan(),
            check_path.yellow()
        );

        #[cfg(unix)]
        {
            let output = Command::new("du").args(["-sh", check_path]).output()?;

            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout);
                return Ok(ToolResult {
                    success: true,
                    output: result.trim().to_string(),
                    error: None,
                    metadata: Some(serde_json::json!({
                        "path": check_path,
                        "type": "disk_usage"
                    })),
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("dir").args([check_path, "/-c"]).output()?;

            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout);
                return Ok(ToolResult {
                    success: true,
                    output: result,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "path": check_path,
                        "type": "disk_usage"
                    })),
                });
            }
        }

        Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some("Unable to check disk usage on this system".to_string()),
            metadata: None,
        })
    }

    pub async fn process_list(
        &self,
        filter: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing processes", "⚙️".cyan());

        #[cfg(unix)]
        {
            let mut cmd = Command::new("ps");
            cmd.args(["aux"]);

            let output = cmd.output()?;

            if output.status.success() {
                let mut result = String::from_utf8_lossy(&output.stdout).to_string();

                if let Some(filter_term) = filter {
                    let filtered_lines: Vec<&str> = result
                        .lines()
                        .filter(|line| line.to_lowercase().contains(&filter_term.to_lowercase()))
                        .collect();
                    result = filtered_lines.join("\n");
                }

                return Ok(ToolResult {
                    success: true,
                    output: result,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "filter": filter,
                        "type": "process_list"
                    })),
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("tasklist").args(["/fo", "table"]).output()?;

            if output.status.success() {
                let mut result = String::from_utf8_lossy(&output.stdout).to_string();

                if let Some(filter_term) = filter {
                    let filtered_lines: Vec<&str> = result
                        .lines()
                        .filter(|line| line.to_lowercase().contains(&filter_term.to_lowercase()))
                        .collect();
                    result = filtered_lines.join("\n");
                }

                return Ok(ToolResult {
                    success: true,
                    output: result,
                    error: None,
                    metadata: Some(serde_json::json!({
                        "filter": filter,
                        "type": "process_list"
                    })),
                });
            }
        }

        Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some("Unable to list processes on this system".to_string()),
            metadata: None,
        })
    }

    pub async fn network_info(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting network information", "🌐".cyan());

        let mut info = Vec::new();

        #[cfg(unix)]
        {
            // Get network interfaces
            if let Ok(output) = Command::new("ifconfig").output() {
                if output.status.success() {
                    info.push("Network Interfaces:".to_string());
                    info.push(String::from_utf8_lossy(&output.stdout).to_string());
                }
            } else if let Ok(output) = Command::new("ip").args(["addr", "show"]).output() {
                if output.status.success() {
                    info.push("Network Interfaces:".to_string());
                    info.push(String::from_utf8_lossy(&output.stdout).to_string());
                }
            }

            // Get routing table
            if let Ok(output) = Command::new("netstat").args(["-rn"]).output() {
                if output.status.success() {
                    info.push("\nRouting Table:".to_string());
                    info.push(String::from_utf8_lossy(&output.stdout).to_string());
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("ipconfig").args(["/all"]).output() {
                if output.status.success() {
                    info.push("Network Configuration:".to_string());
                    info.push(String::from_utf8_lossy(&output.stdout).to_string());
                }
            }
        }

        if info.is_empty() {
            info.push("Network information not available".to_string());
        }

        Ok(ToolResult {
            success: true,
            output: info.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "type": "network_info",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        })
    }

    // File watching capability
    pub async fn file_watch(
        &self,
        path: &str,
        duration_seconds: Option<u64>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Watching file: {} for changes",
            "👁️".cyan(),
            path.yellow()
        );

        let duration = duration_seconds.unwrap_or(30);
        let mut changes = Vec::new();

        // Get initial file state
        let initial_metadata = match std::fs::metadata(path) {
            Ok(meta) => Some(meta),
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("File not found: {}", path)),
                    metadata: None,
                });
            }
        };

        let start_time = std::time::Instant::now();
        let mut last_modified = initial_metadata.as_ref().and_then(|m| m.modified().ok());

        while start_time.elapsed().as_secs() < duration {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if let Some(last_mod) = last_modified {
                        if modified > last_mod {
                            changes.push(format!("File modified at: {:?}", modified));
                            last_modified = Some(modified);
                        }
                    }
                }
            } else {
                changes.push("File was deleted or became inaccessible".to_string());
                break;
            }
        }

        let output = if changes.is_empty() {
            format!(
                "No changes detected in {} during {} seconds",
                path, duration
            )
        } else {
            format!("Changes detected in {}:\n{}", path, changes.join("\n"))
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
            metadata: Some(serde_json::json!({
                "file": path,
                "duration_seconds": duration,
                "changes_count": changes.len()
            })),
        })
    }
}

// Add these dependencies to Cargo.toml if not already present:
// hostname = "0.3"
// chrono = { version = "0.4", features = ["serde"] }
