use super::core::{
    CargoOperation, NpmOperation, PackageManagerOperation, PipOperation, ServiceOperation,
    ToolExecutor, ToolResult,
};
use colored::Colorize;
use std::process::Command;

impl ToolExecutor {
    pub async fn cargo_operation(
        &self,
        operation: CargoOperation,
        package: Option<&str>,
        features: Option<Vec<String>>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Cargo operation: {:?}", "🦀".cyan(), operation);

        let mut cmd = Command::new("cargo");
        
        match operation {
            CargoOperation::Build => {
                cmd.arg("build");
                if let Some(ref features) = features {
                    if !features.is_empty() {
                        cmd.arg("--features").arg(features.join(","));
                    }
                }
            }
            CargoOperation::Run => {
                cmd.arg("run");
                if let Some(ref features) = features {
                    if !features.is_empty() {
                        cmd.arg("--features").arg(features.join(","));
                    }
                }
            }
            CargoOperation::Test => {
                cmd.arg("test");
                if let Some(ref features) = features {
                    if !features.is_empty() {
                        cmd.arg("--features").arg(features.join(","));
                    }
                }
            }
            CargoOperation::Check => {
                cmd.arg("check");
                if let Some(ref features) = features {
                    if !features.is_empty() {
                        cmd.arg("--features").arg(features.join(","));
                    }
                }
            }
            CargoOperation::Install => {
                cmd.arg("install");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
            }
            CargoOperation::Add => {
                cmd.arg("add");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
                if let Some(ref features) = features {
                    if !features.is_empty() {
                        cmd.arg("--features").arg(features.join(","));
                    }
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
            web_search_result: None,
                    });
                }
            }
            CargoOperation::Update => {
                cmd.arg("update");
                if let Some(pkg) = package {
                    cmd.arg("-p").arg(pkg);
                }
            }
            CargoOperation::Clean => {
                cmd.arg("clean");
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("Cargo operation failed".to_string()) },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "features": features
            })),
            web_search_result: None,
        })
    }

    pub async fn npm_operation(
        &self,
        operation: NpmOperation,
        package: Option<&str>,
        dev: bool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} NPM operation: {:?}", "📦".cyan(), operation);

        let mut cmd = Command::new("npm");
        
        match operation {
            NpmOperation::Install => {
                cmd.arg("install");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
                if dev {
                    cmd.arg("--save-dev");
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
            web_search_result: None,
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
                if package.is_some() {
                    cmd.arg("fix");
                }
            }
            NpmOperation::Run { ref script } => {
                cmd.arg("run").arg(script);
            }
            NpmOperation::List => {
                cmd.arg("list");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                }
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("NPM operation failed".to_string()) },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "dev": dev
            })),
            web_search_result: None,
        })
    }

    pub async fn pip_operation(
        &self,
        operation: PipOperation,
        package: Option<&str>,
        requirements_file: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Pip operation: {:?}", "🐍".cyan(), operation);

        let mut cmd = Command::new("pip");
        
        match operation {
            PipOperation::Install => {
                cmd.arg("install");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else if let Some(req_file) = requirements_file {
                    cmd.arg("-r").arg(req_file);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name or requirements file required for install operation".to_string()),
                        metadata: None,
            web_search_result: None,
                    });
                }
            }
            PipOperation::Uninstall => {
                cmd.arg("uninstall").arg("-y");
                if let Some(pkg) = package {
                    cmd.arg(pkg);
                } else {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Package name required for uninstall operation".to_string()),
                        metadata: None,
            web_search_result: None,
                    });
                }
            }
            PipOperation::List => {
                cmd.arg("list");
                if let Some(pkg) = package {
                    cmd.arg("--grep").arg(pkg);
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
            web_search_result: None,
                    });
                }
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("Pip operation failed".to_string()) },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "requirements_file": requirements_file
            })),
            web_search_result: None,
        })
    }

    pub async fn system_package_manager(
        &self,
        operation: PackageManagerOperation,
        package: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} System package manager: {:?}", "📦".cyan(), operation);

        // Detect the system package manager
        let package_manager = self.detect_package_manager().await?;
        
        let mut cmd = Command::new(&package_manager.command);
        
        match package_manager.name.as_str() {
            "apt" => {
                match operation {
                    PackageManagerOperation::Install => {
                        cmd.args(&["install", "-y"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Remove => {
                        cmd.args(&["remove", "-y"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Update => {
                        cmd.arg("update");
                    }
                    PackageManagerOperation::Search => {
                        cmd.arg("search");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::List => {
                        cmd.arg("list");
                        if package.is_some() {
                            cmd.arg("--installed");
                        }
                    }
                    PackageManagerOperation::Info => {
                        cmd.arg("show");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::CheckInstalled => {
                        cmd.args(&["list", "--installed"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                }
            }
            "yum" | "dnf" => {
                match operation {
                    PackageManagerOperation::Install => {
                        cmd.args(&["install", "-y"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Remove => {
                        cmd.args(&["remove", "-y"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Update => {
                        cmd.arg("update");
                    }
                    PackageManagerOperation::Search => {
                        cmd.arg("search");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::List => {
                        cmd.arg("list");
                        if package.is_some() {
                            cmd.arg("installed");
                        }
                    }
                    PackageManagerOperation::Info => {
                        cmd.arg("info");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::CheckInstalled => {
                        cmd.args(&["list", "installed"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                }
            }
            "pacman" => {
                match operation {
                    PackageManagerOperation::Install => {
                        cmd.args(&["-S", "--noconfirm"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Remove => {
                        cmd.args(&["-R", "--noconfirm"]);
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Update => {
                        cmd.args(&["-Syu", "--noconfirm"]);
                    }
                    PackageManagerOperation::Search => {
                        cmd.arg("-Ss");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::List => {
                        cmd.arg("-Q");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Info => {
                        cmd.arg("-Si");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::CheckInstalled => {
                        cmd.arg("-Q");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                }
            }
            "brew" => {
                match operation {
                    PackageManagerOperation::Install => {
                        cmd.arg("install");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Remove => {
                        cmd.arg("uninstall");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Update => {
                        cmd.arg("update");
                    }
                    PackageManagerOperation::Search => {
                        cmd.arg("search");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::List => {
                        cmd.arg("list");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::Info => {
                        cmd.arg("info");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                    PackageManagerOperation::CheckInstalled => {
                        cmd.arg("list");
                        if let Some(pkg) = package {
                            cmd.arg(pkg);
                        }
                    }
                }
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported package manager: {}", package_manager.name)),
                    metadata: None,
            web_search_result: None,
                });
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("Package manager operation failed".to_string()) },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "package": package,
                "package_manager": package_manager.name
            })),
            web_search_result: None,
        })
    }

    pub async fn service_manager(
        &self,
        operation: ServiceOperation,
        service_name: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Service manager: {:?} {}", "⚙️".cyan(), operation, service_name.yellow());

        let service_manager = self.detect_service_manager().await?;
        
        let mut cmd = Command::new(&service_manager.command);
        
        match service_manager.name.as_str() {
            "systemctl" => {
                match operation {
                    ServiceOperation::Start => {
                        cmd.args(&["start", service_name]);
                    }
                    ServiceOperation::Stop => {
                        cmd.args(&["stop", service_name]);
                    }
                    ServiceOperation::Restart => {
                        cmd.args(&["restart", service_name]);
                    }
                    ServiceOperation::Status => {
                        cmd.args(&["status", service_name]);
                    }
                    ServiceOperation::Enable => {
                        cmd.args(&["enable", service_name]);
                    }
                    ServiceOperation::Disable => {
                        cmd.args(&["disable", service_name]);
                    }
                    ServiceOperation::List => {
                        cmd.args(&["list-units", "--type=service"]);
                    }
                }
            }
            "service" => {
                match operation {
                    ServiceOperation::Start => {
                        cmd.args(&[service_name, "start"]);
                    }
                    ServiceOperation::Stop => {
                        cmd.args(&[service_name, "stop"]);
                    }
                    ServiceOperation::Restart => {
                        cmd.args(&[service_name, "restart"]);
                    }
                    ServiceOperation::Status => {
                        cmd.args(&[service_name, "status"]);
                    }
                    ServiceOperation::Enable => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Enable/disable not supported with service command".to_string()),
                            metadata: None,
            web_search_result: None,
                        });
                    }
                    ServiceOperation::Disable => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Enable/disable not supported with service command".to_string()),
                            metadata: None,
            web_search_result: None,
                        });
                    }
                    ServiceOperation::List => {
                        cmd.args(&["--status-all"]);
                    }
                }
            }
            "launchctl" => {
                match operation {
                    ServiceOperation::Start => {
                        cmd.args(&["start", service_name]);
                    }
                    ServiceOperation::Stop => {
                        cmd.args(&["stop", service_name]);
                    }
                    ServiceOperation::Restart => {
                        cmd.args(&["stop", service_name]);
                        // Note: This is simplified - real restart would need two commands
                    }
                    ServiceOperation::Status => {
                        cmd.args(&["list", service_name]);
                    }
                    ServiceOperation::Enable => {
                        cmd.args(&["enable", service_name]);
                    }
                    ServiceOperation::Disable => {
                        cmd.args(&["disable", service_name]);
                    }
                    ServiceOperation::List => {
                        cmd.arg("list");
                    }
                }
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unsupported service manager: {}", service_manager.name)),
                    metadata: None,
            web_search_result: None,
                });
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("Service manager operation failed".to_string()) },
            metadata: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "service_name": service_name,
                "service_manager": service_manager.name
            })),
            web_search_result: None,
        })
    }

    pub async fn environment_info(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting environment information", "🌍".cyan());

        let mut info = Vec::new();
        
        // Basic environment info
        info.push(format!("Operating System: {}", std::env::consts::OS));
        info.push(format!("Architecture: {}", std::env::consts::ARCH));
        info.push(format!("Family: {}", std::env::consts::FAMILY));
        
        // Current directory
        if let Ok(current_dir) = std::env::current_dir() {
            info.push(format!("Current Directory: {}", current_dir.display()));
        }
        
        // User information
        if let Ok(user) = std::env::var("USER") {
            info.push(format!("User: {}", user));
        } else if let Ok(username) = std::env::var("USERNAME") {
            info.push(format!("User: {}", username));
        }
        
        // Home directory
        if let Ok(home) = std::env::var("HOME") {
            info.push(format!("Home Directory: {}", home));
        } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
            info.push(format!("Home Directory: {}", userprofile));
        }
        
        // Shell information
        if let Ok(shell) = std::env::var("SHELL") {
            info.push(format!("Shell: {}", shell));
        }
        
        // Path information
        if let Ok(path) = std::env::var("PATH") {
            let path_count = path.split(':').count();
            info.push(format!("PATH entries: {}", path_count));
        }
        
        // Available development tools
        let mut dev_tools = Vec::new();
        let tools_to_check = ["git", "cargo", "npm", "python", "python3", "node", "java", "gcc", "clang"];
        
        for tool in &tools_to_check {
            if Command::new("which").arg(tool).output().map(|o| o.status.success()).unwrap_or(false) {
                dev_tools.push(tool.to_string());
            }
        }
        
        if !dev_tools.is_empty() {
            info.push(format!("Available dev tools: {}", dev_tools.join(", ")));
        }

        Ok(ToolResult {
            success: true,
            output: info.join("\n"),
            error: None,
            metadata: Some(serde_json::json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "family": std::env::consts::FAMILY,
                "available_tools": dev_tools
            })),
            web_search_result: None,
        })
    }

    async fn detect_package_manager(&self) -> Result<PackageManagerInfo, Box<dyn std::error::Error>> {
        let managers = [
            ("apt", "apt-get"),
            ("yum", "yum"),
            ("dnf", "dnf"),
            ("pacman", "pacman"),
            ("brew", "brew"),
            ("zypper", "zypper"),
        ];

        for (name, command) in &managers {
            if Command::new("which").arg(command).output().map(|o| o.status.success()).unwrap_or(false) {
                return Ok(PackageManagerInfo {
                    name: name.to_string(),
                    command: command.to_string(),
                });
            }
        }

        Err("No supported package manager found".into())
    }

    async fn detect_service_manager(&self) -> Result<ServiceManagerInfo, Box<dyn std::error::Error>> {
        let managers = [
            ("systemctl", "systemctl"),
            ("service", "service"),
            ("launchctl", "launchctl"),
        ];

        for (name, command) in &managers {
            if Command::new("which").arg(command).output().map(|o| o.status.success()).unwrap_or(false) {
                return Ok(ServiceManagerInfo {
                    name: name.to_string(),
                    command: command.to_string(),
                });
            }
        }

        Err("No supported service manager found".into())
    }

    pub async fn check_package_managers(&self) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Checking available package managers", "📦".cyan());

        let mut available_managers = Vec::new();
        let managers = [
            ("apt", "apt-get"),
            ("yum", "yum"),
            ("dnf", "dnf"),
            ("pacman", "pacman"),
            ("brew", "brew"),
            ("zypper", "zypper"),
            ("pkg", "pkg"),
            ("portage", "emerge"),
        ];

        for (name, command) in &managers {
            if Command::new("which").arg(command).output().map(|o| o.status.success()).unwrap_or(false) {
                available_managers.push(format!("{} ({})", name, command));
            }
        }

        if available_managers.is_empty() {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("No supported package managers found".to_string()),
                metadata: None,
            web_search_result: None,
            })
        } else {
            Ok(ToolResult {
                success: true,
                output: format!("Available package managers:\n{}", available_managers.join("\n")),
                error: None,
                metadata: Some(serde_json::json!({
                    "available_managers": available_managers
                })),
            web_search_result: None,
            })
        }
    }

    pub async fn search_packages(&self, query: &str) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Searching for packages: {}", "🔍".cyan(), query.yellow());

        let package_manager = self.detect_package_manager().await?;
        
        let mut cmd = Command::new(&package_manager.command);
        
        match package_manager.name.as_str() {
            "apt" => {
                cmd.args(&["search", query]);
            }
            "yum" | "dnf" => {
                cmd.args(&["search", query]);
            }
            "pacman" => {
                cmd.args(&["-Ss", query]);
            }
            "brew" => {
                cmd.args(&["search", query]);
            }
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Search not supported for package manager: {}", package_manager.name)),
                    metadata: None,
            web_search_result: None,
                });
            }
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            String::from_utf8_lossy(&output.stdout)
        } else {
            String::from_utf8_lossy(&output.stderr)
        };

        Ok(ToolResult {
            success,
            output: output_text.to_string(),
            error: if success { None } else { Some("Package search failed".to_string()) },
            metadata: Some(serde_json::json!({
                "query": query,
                "package_manager": package_manager.name
            })),
            web_search_result: None,
        })
    }
}

#[derive(Debug, Clone)]
struct PackageManagerInfo {
    name: String,
    command: String,
}

#[derive(Debug, Clone)]
struct ServiceManagerInfo {
    name: String,
    command: String,
}