use super::core::{DockerResourceType, ToolExecutor, ToolResult};
use colored::Colorize;
use std::collections::HashMap;
use std::process::Command;

impl ToolExecutor {
    pub async fn docker_list(
        &self,
        resource_type: DockerResourceType,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let (cmd_arg, description) = match resource_type {
            DockerResourceType::Containers => ("ps", "containers"),
            DockerResourceType::Images => ("images", "images"),
            DockerResourceType::Volumes => ("volume", "volumes"),
            DockerResourceType::Networks => ("network", "networks"),
        };

        println!("{} Listing Docker {}", "🐳".cyan(), description.yellow());

        let mut cmd = Command::new("docker");

        match resource_type {
            DockerResourceType::Containers => {
                cmd.args([
                    "ps",
                    "-a",
                    "--format",
                    "table {{.ID}}\\t{{.Image}}\\t{{.Command}}\\t{{.Status}}\\t{{.Names}}",
                ]);
            }
            DockerResourceType::Images => {
                cmd.args([
                    "images",
                    "--format",
                    "table {{.Repository}}\\t{{.Tag}}\\t{{.ID}}\\t{{.Size}}\\t{{.CreatedAt}}",
                ]);
            }
            DockerResourceType::Volumes => {
                cmd.args(["volume", "ls", "--format", "table {{.Driver}}\\t{{.Name}}"]);
            }
            DockerResourceType::Networks => {
                cmd.args([
                    "network",
                    "ls",
                    "--format",
                    "table {{.ID}}\\t{{.Name}}\\t{{.Driver}}\\t{{.Scope}}",
                ]);
            }
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if error_msg.contains("command not found")
                        || error_msg.contains("not recognized")
                    {
                        "Docker is not installed or not in PATH".to_string()
                    } else {
                        error_msg.to_string()
                    },
                ),
                metadata: None,
            });
        }

        let result = String::from_utf8_lossy(&output.stdout);

        Ok(ToolResult {
            success: true,
            output: if result.trim().is_empty() {
                format!("No Docker {} found", description)
            } else {
                result.to_string()
            },
            error: None,
            metadata: Some(serde_json::json!({
                "resource_type": format!("{:?}", resource_type),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        })
    }

    pub async fn docker_run(
        &self,
        image: &str,
        command: Option<String>,
        ports: Option<Vec<String>>,
        volumes: Option<Vec<String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Running Docker container: {}",
            "🚀".cyan(),
            image.yellow()
        );

        let mut cmd = Command::new("docker");
        cmd.args(["run", "-d"]); // Run in detached mode

        // Add port mappings
        if let Some(ports) = ports {
            for port in ports {
                cmd.args(["-p", &port]);
                println!("  {} Port mapping: {}", "🔌".blue(), port);
            }
        }

        // Add volume mappings
        if let Some(volumes) = volumes {
            for volume in volumes {
                cmd.args(["-v", &volume]);
                println!("  {} Volume mapping: {}", "📁".blue(), volume);
            }
        }

        // Add environment variables
        if let Some(env_vars) = environment {
            for (key, value) in env_vars {
                cmd.args(["-e", &format!("{}={}", key, value)]);
                println!("  {} Environment: {}={}", "🌍".blue(), key, value);
            }
        }

        // Add image
        cmd.arg(image);

        // Add command if specified
        if let Some(container_command) = command {
            cmd.args(container_command.split_whitespace());
        }

        let output = cmd.output()?;

        if output.status.success() {
            let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(ToolResult {
                success: true,
                output: format!(
                    "Container started successfully\nContainer ID: {}",
                    container_id
                ),
                error: None,
                metadata: Some(serde_json::json!({
                    "container_id": container_id,
                    "image": image,
                    "type": "container_start"
                })),
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            })
        }
    }

    pub async fn docker_stop(
        &self,
        container: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Stopping Docker container: {}",
            "⏹️".cyan(),
            container.yellow()
        );

        let output = Command::new("docker").args(["stop", container]).output()?;

        if output.status.success() {
            Ok(ToolResult {
                success: true,
                output: format!("Container '{}' stopped successfully", container),
                error: None,
                metadata: Some(serde_json::json!({
                    "container": container,
                    "type": "container_stop"
                })),
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            })
        }
    }

    pub async fn docker_logs(
        &self,
        container: &str,
        follow: bool,
        tail: Option<u32>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Getting Docker logs for: {}",
            "📋".cyan(),
            container.yellow()
        );

        let mut cmd = Command::new("docker");
        cmd.args(["logs"]);

        if let Some(tail_lines) = tail {
            cmd.args(["--tail", &tail_lines.to_string()]);
        }

        if follow {
            cmd.arg("--follow");
            println!("  {} Following logs (this may take a while)", "⏳".yellow());
        }

        cmd.arg(container);

        let output = cmd.output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let logs = if !stderr.is_empty() {
                format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)
            } else {
                stdout.to_string()
            };

            Ok(ToolResult {
                success: true,
                output: if logs.trim().is_empty() {
                    "No logs available for this container".to_string()
                } else {
                    logs
                },
                error: None,
                metadata: Some(serde_json::json!({
                    "container": container,
                    "follow": follow,
                    "tail": tail,
                    "type": "container_logs"
                })),
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            })
        }
    }

    pub async fn docker_inspect(
        &self,
        container: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Inspecting Docker container: {}",
            "🔍".cyan(),
            container.yellow()
        );

        let output = Command::new("docker")
            .args(["inspect", container])
            .output()?;

        if output.status.success() {
            let inspect_data = String::from_utf8_lossy(&output.stdout);

            // Parse JSON to extract key information
            if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&inspect_data) {
                if let Some(container_info) = json_data.as_array().and_then(|arr| arr.first()) {
                    let mut summary = Vec::new();

                    if let Some(id) = container_info.get("Id").and_then(|v| v.as_str()) {
                        summary.push(format!("ID: {}", &id[..12]));
                    }

                    if let Some(name) = container_info.get("Name").and_then(|v| v.as_str()) {
                        summary.push(format!("Name: {}", name));
                    }

                    if let Some(state) = container_info.get("State") {
                        if let Some(status) = state.get("Status").and_then(|v| v.as_str()) {
                            summary.push(format!("Status: {}", status));
                        }
                        if let Some(running) = state.get("Running").and_then(|v| v.as_bool()) {
                            summary.push(format!("Running: {}", running));
                        }
                    }

                    if let Some(config) = container_info.get("Config") {
                        if let Some(image) = config.get("Image").and_then(|v| v.as_str()) {
                            summary.push(format!("Image: {}", image));
                        }
                    }

                    return Ok(ToolResult {
                        success: true,
                        output: format!(
                            "Container Summary:\n{}\n\nFull JSON:\n{}",
                            summary.join("\n"),
                            inspect_data
                        ),
                        error: None,
                        metadata: Some(container_info.clone()),
                    });
                }
            }

            Ok(ToolResult {
                success: true,
                output: inspect_data.to_string(),
                error: None,
                metadata: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            })
        }
    }

    pub async fn docker_exec(
        &self,
        container: &str,
        command: &str,
        interactive: bool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Executing command in container {}: {}",
            "⚡".cyan(),
            container.yellow(),
            command.blue()
        );

        let mut cmd = Command::new("docker");
        cmd.args(["exec"]);

        if interactive {
            cmd.args(["-it"]);
        }

        cmd.arg(container);
        cmd.args(command.split_whitespace());

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
                "container": container,
                "command": command,
                "interactive": interactive,
                "exit_code": output.status.code()
            })),
        })
    }

    pub async fn docker_build(
        &self,
        context_path: &str,
        tag: Option<String>,
        dockerfile: Option<String>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let tag_clone = tag.clone();
        let tag_name = tag_clone.as_deref().unwrap_or("latest");
        println!(
            "{} Building Docker image: {}",
            "🔨".cyan(),
            tag_name.yellow()
        );

        let mut cmd = Command::new("docker");
        cmd.args(["build"]);

        if let Some(ref dockerfile_path) = dockerfile {
            cmd.args(["-f", &dockerfile_path]);
        }

        if let Some(tag_value) = tag {
            cmd.args(["-t", &tag_value]);
        }

        cmd.arg(context_path);

        let output = cmd.output()?;

        if output.status.success() {
            Ok(ToolResult {
                success: true,
                output: format!(
                    "Docker image built successfully\nTag: {}\nContext: {}",
                    tag_name, context_path
                ),
                error: None,
                metadata: Some(serde_json::json!({
                    "tag": tag_name,
                    "context_path": context_path,
                    "dockerfile": dockerfile,
                    "type": "image_build"
                })),
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::from_utf8_lossy(&output.stdout).to_string(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            })
        }
    }

    pub async fn docker_compose_up(
        &self,
        compose_file: Option<String>,
        detached: bool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Starting Docker Compose services", "🐙".cyan());

        let mut cmd = Command::new("docker-compose");

        if let Some(ref file) = compose_file {
            cmd.args(["-f", &file]);
        }

        cmd.arg("up");

        if detached {
            cmd.arg("-d");
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
                "compose_file": compose_file,
                "detached": detached,
                "type": "compose_up"
            })),
        })
    }
}
