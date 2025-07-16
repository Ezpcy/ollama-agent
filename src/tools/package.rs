use super::core::{CargoOperation, NpmOperation, PipOperation, ToolExecutor, ToolResult};
use colored::Colorize;
use std::process::Command;

impl ToolExecutor {
    // Cargo (Rust) operations
    pub async fn cargo_operation(
        &self,
        operation: CargoOperation,
        package: Option<&str>,
        features: Option<Vec<String>>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let operation_name = format!("{:?}", operation).to_lowercase();
        println!("{} Running cargo {}", "📦".cyan(), operation_name.yellow());

        let mut cmd = Command::new("cargo");

        match operation {
            CargoOperation::Build => {
                cmd.arg("build");
                if let Some(ref features_list) = features {
                    cmd.arg("--features").arg(features_list.join(","));
                }
            }
            CargoOperation::Run => {
                cmd.arg("run");
                if let Some(ref features_list) = features {
                    cmd.arg("--features").arg(features_list.join(","));
                }
            }
            CargoOperation::Test => {
                cmd.arg("test");
                if let Some(ref features_list) = features {
                    cmd.arg("--features").arg(features_list.join(","));
                }
            }
            CargoOperation::Check => {
                cmd.arg("check");
            }
            CargoOperation::Install => {
                cmd.arg("install");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for install operation".to_string()),
                        metadata: None,
                    });
                }
            }
            CargoOperation::Add => {
                cmd.arg("add");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                    if let Some(ref features_list) = features {
                        cmd.arg("--features").arg(features_list.join(","));
                    }
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for add operation".to_string()),
                        metadata: None,
                    });
                }
            }
            CargoOperation::Remove => {
                cmd.arg("remove");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for remove operation".to_string()),
                        metadata: None,
                    });
                }
            }
            CargoOperation::Update => {
                cmd.arg("update");
                if let Some(pkg) = package {
                    cmd.arg("--package").arg(pkg);
                }
            }
            CargoOperation::Clean => {
                cmd.arg("clean");
            }
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr for cargo output
        let combined_output = if !stdout.is_empty() && !stderr.is_empty() {
            format!("{}\n{}", stdout, stderr)
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };

        Ok(ToolResult {
            success: output.status.success(),
            output: combined_output,
            error: if output.status.success() {
                None
            } else {
                Some("Cargo operation failed".to_string())
            },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "features": features,
                "exit_code": output.status.code()
            })),
        })
    }

    // NPM (Node.js) operations
    pub async fn npm_operation(
        &self,
        operation: NpmOperation,
        package: Option<&str>,
        dev: bool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let operation_name = match operation {
            NpmOperation::Run { ref script } => format!("run {}", script),
            _ => format!("{:?}", operation).to_lowercase(),
        };

        println!("{} Running npm {}", "📦".cyan(), operation_name.yellow());

        let mut cmd = Command::new("npm");

        match operation {
            NpmOperation::Install => {
                cmd.arg("install");
                if let Some(pkg) = package {
                    if dev {
                        cmd.arg("--save-dev");
                    }
                    cmd.arg(pkg);
                }
            }
            NpmOperation::Uninstall => {
                cmd.arg("uninstall");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for uninstall operation".to_string()),
                        metadata: None,
                    });
                }
            }
            NpmOperation::Update => {
                cmd.arg("update");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
            }
            NpmOperation::Audit => {
                cmd.arg("audit");
            }
            NpmOperation::Run { ref script } => {
                cmd.args(["run", &script]);
            }
            NpmOperation::List => {
                cmd.arg("list");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
            }
        }

        let output = cmd.output()?;

        Ok(ToolResult {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "dev": dev,
                "exit_code": output.status.code()
            })),
        })
    }

    // Pip (Python) operations
    pub async fn pip_operation(
        &self,
        operation: PipOperation,
        package: Option<&str>,
        requirements_file: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let operation_name = format!("{:?}", operation).to_lowercase();
        println!("{} Running pip {}", "🐍".cyan(), operation_name.yellow());

        let mut cmd = Command::new("pip");

        match operation {
            PipOperation::Install => {
                cmd.arg("install");
                if let Some(req_file) = requirements_file {
                    cmd.args(["-r", req_file]);
                } else if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(
                            "Package name or requirements file required for install".to_string(),
                        ),
                        metadata: None,
                    });
                }
            }
            PipOperation::Uninstall => {
                cmd.arg("uninstall");
                cmd.arg("-y"); // Assume yes for automation
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for uninstall operation".to_string()),
                        metadata: None,
                    });
                }
            }
            PipOperation::List => {
                cmd.arg("list");
                if let Some(_pkg) = package {
                    // Use grep-like functionality to filter
                    cmd.arg("--format").arg("columns");
                }
            }
            PipOperation::Freeze => {
                cmd.arg("freeze");
            }
            PipOperation::Show => {
                cmd.arg("show");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for show operation".to_string()),
                        metadata: None,
                    });
                }
            }
        }

        let output = cmd.output()?;

        let mut result_output = String::from_utf8_lossy(&output.stdout).to_string();

        // Filter output if showing specific package in list
        if let (PipOperation::List, Some(_pkg)) = (&operation, package) {
            let filtered_lines: Vec<&str> = result_output
                .lines()
                .filter(|line| line.to_lowercase().contains(&_pkg.to_lowercase()))
                .collect();
            if !filtered_lines.is_empty() {
                result_output = filtered_lines.join("\n");
            } else {
                result_output = format!("Package '{}' not found in installed packages", _pkg);
            }
        }

        Ok(ToolResult {
            success: output.status.success(),
            output: result_output,
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "requirements_file": requirements_file,
                "exit_code": output.status.code()
            })),
        })
    }

    // Check if package managers are available
    pub async fn check_package_managers(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Checking available package managers", "🔍".cyan());

        let mut available = Vec::new();
        let mut unavailable = Vec::new();

        // Check Cargo
        match Command::new("cargo").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ Cargo: {}", version));
            }
            _ => unavailable.push("✗ Cargo: Not available"),
        }

        // Check NPM
        match Command::new("npm").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ NPM: {}", version));
            }
            _ => unavailable.push("✗ NPM: Not available"),
        }

        // Check Yarn
        match Command::new("yarn").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ Yarn: {}", version));
            }
            _ => unavailable.push("✗ Yarn: Not available"),
        }

        // Check Pip
        match Command::new("pip").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ Pip: {}", version));
            }
            _ => unavailable.push("✗ Pip: Not available"),
        }

        // Check Python
        match Command::new("python").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ Python: {}", version));
            }
            _ => {
                // Try python3
                match Command::new("python3").arg("--version").output() {
                    Ok(output) if output.status.success() => {
                        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        available.push(format!("✓ Python3: {}", version));
                    }
                    _ => unavailable.push("✗ Python: Not available"),
                }
            }
        }

        // Check Node.js
        match Command::new("node").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                available.push(format!("✓ Node.js: {}", version));
            }
            _ => unavailable.push("✗ Node.js: Not available"),
        }

        let mut result = Vec::new();
        result.push("Available Package Managers:".to_string());
        result.extend(available);

        if !unavailable.is_empty() {
            result.push(String::new());
            result.push("Unavailable Package Managers:".to_string());
            result.extend(unavailable.into_iter().map(|s| s.to_string()));
        }

        Ok(ToolResult {
            success: true,
            output: result.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "type": "package_manager_check",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        })
    }

    // Universal package search across multiple managers
    pub async fn search_packages(
        &self,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Searching for packages: {}", "🔍".cyan(), query.yellow());

        let mut results = Vec::new();

        // Search Cargo
        if let Ok(output) = Command::new("cargo").args(["search", query]).output() {
            if output.status.success() {
                let cargo_results = String::from_utf8_lossy(&output.stdout);
                if !cargo_results.trim().is_empty() {
                    results.push(format!("📦 Cargo Results:\n{}", cargo_results));
                }
            }
        }

        // Search NPM
        if let Ok(output) = Command::new("npm").args(["search", query]).output() {
            if output.status.success() {
                let npm_results = String::from_utf8_lossy(&output.stdout);
                if !npm_results.trim().is_empty() {
                    results.push(format!("📦 NPM Results:\n{}", npm_results));
                }
            }
        }

        // Search Pip (using pip search alternative)
        if let Ok(output) = Command::new("pip")
            .args(["index", "versions", query])
            .output()
        {
            if output.status.success() {
                let pip_results = String::from_utf8_lossy(&output.stdout);
                if !pip_results.trim().is_empty() {
                    results.push(format!("🐍 Pip Results:\n{}", pip_results));
                }
            }
        }

        Ok(ToolResult {
            success: true,
            output: if results.is_empty() {
                format!("No packages found for query: '{}'", query)
            } else {
                results.join("\n\n")
            },
            error: None,
            metadata: Some(serde_json::json!({
                "query": query,
                "results_count": results.len()
            })),
        })
    }
}
