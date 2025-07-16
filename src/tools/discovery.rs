use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: String,
    pub path: Option<PathBuf>,
    pub available: bool,
    pub required_for: Vec<String>,
    pub install_command: Option<String>,
    pub check_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryResult {
    pub available_tools: HashMap<String, ToolInfo>,
    pub missing_tools: Vec<String>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub shell: Option<String>,
    pub package_managers: Vec<String>,
}

pub struct ToolDiscovery {
    tool_definitions: HashMap<String, ToolInfo>,
}

impl ToolDiscovery {
    pub fn new() -> Self {
        let mut tool_definitions = HashMap::new();
        
        // Core system tools
        tool_definitions.insert("git".to_string(), ToolInfo {
            name: "git".to_string(),
            version: None,
            description: "Git version control system".to_string(),
            path: None,
            available: false,
            required_for: vec!["git_operations".to_string()],
            install_command: Self::get_install_command("git"),
            check_command: "git --version".to_string(),
        });
        
        tool_definitions.insert("docker".to_string(), ToolInfo {
            name: "docker".to_string(),
            version: None,
            description: "Docker containerization platform".to_string(),
            path: None,
            available: false,
            required_for: vec!["docker_operations".to_string()],
            install_command: Self::get_install_command("docker"),
            check_command: "docker --version".to_string(),
        });
        
        tool_definitions.insert("curl".to_string(), ToolInfo {
            name: "curl".to_string(),
            version: None,
            description: "Command-line tool for transferring data".to_string(),
            path: None,
            available: false,
            required_for: vec!["web_operations".to_string(), "api_calls".to_string()],
            install_command: Self::get_install_command("curl"),
            check_command: "curl --version".to_string(),
        });
        
        tool_definitions.insert("jq".to_string(), ToolInfo {
            name: "jq".to_string(),
            version: None,
            description: "Command-line JSON processor".to_string(),
            path: None,
            available: false,
            required_for: vec!["json_processing".to_string()],
            install_command: Self::get_install_command("jq"),
            check_command: "jq --version".to_string(),
        });
        
        // Language-specific tools
        tool_definitions.insert("cargo".to_string(), ToolInfo {
            name: "cargo".to_string(),
            version: None,
            description: "Rust package manager and build tool".to_string(),
            path: None,
            available: false,
            required_for: vec!["rust_development".to_string()],
            install_command: Some("Visit https://rustup.rs/ to install Rust".to_string()),
            check_command: "cargo --version".to_string(),
        });
        
        tool_definitions.insert("npm".to_string(), ToolInfo {
            name: "npm".to_string(),
            version: None,
            description: "Node.js package manager".to_string(),
            path: None,
            available: false,
            required_for: vec!["javascript_development".to_string()],
            install_command: Some("Visit https://nodejs.org/ to install Node.js".to_string()),
            check_command: "npm --version".to_string(),
        });
        
        tool_definitions.insert("python".to_string(), ToolInfo {
            name: "python".to_string(),
            version: None,
            description: "Python programming language".to_string(),
            path: None,
            available: false,
            required_for: vec!["python_development".to_string()],
            install_command: Self::get_install_command("python3"),
            check_command: "python --version".to_string(),
        });
        
        tool_definitions.insert("go".to_string(), ToolInfo {
            name: "go".to_string(),
            version: None,
            description: "Go programming language".to_string(),
            path: None,
            available: false,
            required_for: vec!["go_development".to_string()],
            install_command: Some("Visit https://golang.org/dl/ to install Go".to_string()),
            check_command: "go version".to_string(),
        });
        
        Self { tool_definitions }
    }
    
    pub async fn discover_tools(&mut self) -> ToolDiscoveryResult {
        let mut available_tools = HashMap::new();
        let mut missing_tools = Vec::new();
        
        // Clone the tool definitions to avoid borrow checker issues
        let tool_definitions_clone = self.tool_definitions.clone();
        
        for (name, tool_info) in tool_definitions_clone {
            println!("{} Checking tool: {}", "🔍".cyan(), name.yellow());
            
            let mut updated_tool = tool_info.clone();
            
            match self.check_tool_availability(&tool_info.check_command).await {
                Ok((available, version, path)) => {
                    updated_tool.available = available;
                    updated_tool.version = version;
                    updated_tool.path = path;
                    
                    if available {
                        println!("  {} Found: {}", "✅".green(), name);
                        if let Some(version) = &updated_tool.version {
                            println!("    Version: {}", version.blue());
                        }
                        available_tools.insert(name.clone(), updated_tool);
                    } else {
                        println!("  {} Not found: {}", "❌".red(), name);
                        missing_tools.push(name.clone());
                    }
                }
                Err(e) => {
                    println!("  {} Error checking {}: {}", "⚠️".yellow(), name, e);
                    missing_tools.push(name.clone());
                }
            }
        }
        
        let system_info = self.get_system_info().await;
        
        ToolDiscoveryResult {
            available_tools,
            missing_tools,
            system_info,
        }
    }
    
    async fn check_tool_availability(&self, check_command: &str) -> Result<(bool, Option<String>, Option<PathBuf>), String> {
        let parts: Vec<&str> = check_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Invalid check command".to_string());
        }
        
        let command = parts[0];
        let args = &parts[1..];
        
        // First check if the tool is available in PATH
        match Command::new("which").arg(command).output() {
            Ok(output) if output.status.success() => {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let path_str = path_str.trim();
                let path = if !path_str.is_empty() {
                    Some(PathBuf::from(path_str))
                } else {
                    None
                };
                
                // Try to get version
                match Command::new(command).args(args).output() {
                    Ok(version_output) if version_output.status.success() => {
                        let version_str = String::from_utf8_lossy(&version_output.stdout);
                        let version = if !version_str.is_empty() {
                            Some(version_str.trim().to_string())
                        } else {
                            None
                        };
                        Ok((true, version, path))
                    }
                    Ok(_) => Ok((true, None, path)),
                    Err(e) => Err(format!("Error getting version: {}", e)),
                }
            }
            Ok(_) => Ok((false, None, None)),
            Err(e) => Err(format!("Error checking tool availability: {}", e)),
        }
    }
    
    async fn get_system_info(&self) -> SystemInfo {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        
        let shell = std::env::var("SHELL").ok()
            .and_then(|s| {
                Path::new(&s).file_name()
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_string())
            });
        
        let mut package_managers = Vec::new();
        
        // Check for common package managers
        let pm_checks = vec![
            ("apt", "apt --version"),
            ("yum", "yum --version"),
            ("dnf", "dnf --version"),
            ("pacman", "pacman --version"),
            ("brew", "brew --version"),
            ("chocolatey", "choco --version"),
            ("scoop", "scoop --version"),
        ];
        
        for (pm_name, check_cmd) in pm_checks {
            if let Ok((available, _, _)) = self.check_tool_availability(check_cmd).await {
                if available {
                    package_managers.push(pm_name.to_string());
                }
            }
        }
        
        SystemInfo {
            os,
            arch,
            shell,
            package_managers,
        }
    }
    
    fn get_install_command(tool_name: &str) -> Option<String> {
        let os = std::env::consts::OS;
        
        match (os, tool_name) {
            ("linux", "git") => Some("sudo apt update && sudo apt install git".to_string()),
            ("linux", "docker") => Some("sudo apt update && sudo apt install docker.io".to_string()),
            ("linux", "curl") => Some("sudo apt update && sudo apt install curl".to_string()),
            ("linux", "jq") => Some("sudo apt update && sudo apt install jq".to_string()),
            ("linux", "python3") => Some("sudo apt update && sudo apt install python3".to_string()),
            
            ("macos", "git") => Some("brew install git".to_string()),
            ("macos", "docker") => Some("brew install docker".to_string()),
            ("macos", "curl") => Some("brew install curl".to_string()),
            ("macos", "jq") => Some("brew install jq".to_string()),
            ("macos", "python3") => Some("brew install python3".to_string()),
            
            ("windows", "git") => Some("winget install Git.Git".to_string()),
            ("windows", "docker") => Some("winget install Docker.DockerDesktop".to_string()),
            ("windows", "curl") => Some("winget install curl.curl".to_string()),
            ("windows", "jq") => Some("winget install jq.jq".to_string()),
            ("windows", "python3") => Some("winget install Python.Python.3".to_string()),
            
            _ => None,
        }
    }
    
    pub fn display_discovery_results(&self, results: &ToolDiscoveryResult) {
        println!();
        println!("{}", "🔍 Tool Discovery Results".cyan().bold());
        println!();
        
        // System info
        println!("{}", "System Information:".blue().bold());
        println!("  OS: {}", results.system_info.os);
        println!("  Architecture: {}", results.system_info.arch);
        if let Some(shell) = &results.system_info.shell {
            println!("  Shell: {}", shell);
        }
        if !results.system_info.package_managers.is_empty() {
            println!("  Package Managers: {}", results.system_info.package_managers.join(", "));
        }
        println!();
        
        // Available tools
        if !results.available_tools.is_empty() {
            println!("{}", "✅ Available Tools:".green().bold());
            for (name, tool) in &results.available_tools {
                println!("  {} {}", "•".green(), name);
                if let Some(version) = &tool.version {
                    println!("    Version: {}", version.blue());
                }
                if let Some(path) = &tool.path {
                    println!("    Path: {}", path.display().to_string().dimmed());
                }
                if !tool.required_for.is_empty() {
                    println!("    Required for: {}", tool.required_for.join(", ").yellow());
                }
            }
            println!();
        }
        
        // Missing tools
        if !results.missing_tools.is_empty() {
            println!("{}", "❌ Missing Tools:".red().bold());
            for tool_name in &results.missing_tools {
                if let Some(tool) = self.tool_definitions.get(tool_name) {
                    println!("  {} {}", "•".red(), tool_name);
                    println!("    Description: {}", tool.description);
                    if let Some(install_cmd) = &tool.install_command {
                        println!("    Install: {}", install_cmd.yellow());
                    }
                    if !tool.required_for.is_empty() {
                        println!("    Required for: {}", tool.required_for.join(", ").yellow());
                    }
                }
            }
        }
        
        // Summary
        println!();
        println!("{}", "📊 Summary:".cyan().bold());
        println!("  Available: {}", results.available_tools.len().to_string().green());
        println!("  Missing: {}", results.missing_tools.len().to_string().red());
        
        let total_tools = results.available_tools.len() + results.missing_tools.len();
        if total_tools > 0 {
            let percentage = (results.available_tools.len() as f64 / total_tools as f64) * 100.0;
            println!("  Coverage: {:.1}%", percentage.to_string().blue());
        }
    }
    
    pub fn get_tools_for_feature(&self, feature: &str) -> Vec<&ToolInfo> {
        self.tool_definitions.values()
            .filter(|tool| tool.required_for.contains(&feature.to_string()))
            .collect()
    }
    
    pub fn get_missing_tools_for_feature(&self, feature: &str, results: &ToolDiscoveryResult) -> Vec<String> {
        self.get_tools_for_feature(feature)
            .into_iter()
            .filter(|tool| results.missing_tools.contains(&tool.name))
            .map(|tool| tool.name.clone())
            .collect()
    }
}

impl Default for ToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}