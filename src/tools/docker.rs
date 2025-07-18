use super::core::{DockerResourceType, ToolExecutor, ToolResult};
use colored::Colorize;
use std::collections::HashMap;
use std::process::Command;

impl ToolExecutor {
    pub async fn docker_list(
        &self,
        resource_type: DockerResourceType,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing Docker {:?}", "🐳".cyan(), resource_type);

        let mut cmd = Command::new("docker");
        
        match resource_type {
            DockerResourceType::Containers => {
                cmd.args(&["ps", "-a"]);
            }
            DockerResourceType::Images => {
                cmd.arg("images");
            }
            DockerResourceType::Volumes => {
                cmd.args(&["volume", "ls"]);
            }
            DockerResourceType::Networks => {
                cmd.args(&["network", "ls"]);
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
            error: if success { None } else { Some("Docker list command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "resource_type": format!("{:?}", resource_type)
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
        println!("{} Running Docker container: {}", "🐳".cyan(), image.yellow());

        let mut cmd = Command::new("docker");
        cmd.args(&["run", "-d"]); // Run in detached mode
        
        // Add port mappings
        if let Some(ref ports) = ports {
            for port in ports {
                cmd.args(&["-p", port]);
            }
        }
        
        // Add volume mappings
        if let Some(ref volumes) = volumes {
            for volume in volumes {
                cmd.args(&["-v", volume]);
            }
        }
        
        // Add environment variables
        if let Some(ref env) = environment {
            for (key, value) in env {
                cmd.args(&["-e", &format!("{}={}", key, value)]);
            }
        }
        
        // Add image
        cmd.arg(image);
        
        // Add command if provided
        if let Some(ref run_command) = command {
            cmd.args(run_command.split_whitespace());
        }

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            format!("Container started successfully. ID: {}", container_id)
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };

        Ok(ToolResult {
            success,
            output: output_text,
            error: if success { None } else { Some("Docker run command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "image": image,
                "command": command,
                "ports": ports,
                "volumes": volumes,
                "environment": environment
            })),
        })
    }

    pub async fn docker_stop(
        &self,
        container: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Stopping Docker container: {}", "🐳".cyan(), container.yellow());

        let output = Command::new("docker")
            .args(&["stop", container])
            .output()?;

        let success = output.status.success();
        
        let output_text = if success {
            format!("Container {} stopped successfully", container)
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };

        Ok(ToolResult {
            success,
            output: output_text,
            error: if success { None } else { Some("Docker stop command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "container": container
            })),
        })
    }

    pub async fn docker_logs(
        &self,
        container: &str,
        follow: bool,
        tail: Option<u32>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting Docker logs for: {}", "🐳".cyan(), container.yellow());

        let mut cmd = Command::new("docker");
        cmd.args(&["logs"]);
        
        if follow {
            cmd.arg("-f");
        }
        
        if let Some(tail_lines) = tail {
            cmd.args(&["--tail", &tail_lines.to_string()]);
        }
        
        cmd.arg(container);

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if !stdout.is_empty() && !stderr.is_empty() {
                format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else if !stderr.is_empty() {
                stderr.to_string()
            } else {
                "No logs available".to_string()
            }
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };

        Ok(ToolResult {
            success,
            output: output_text,
            error: if success { None } else { Some("Docker logs command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "container": container,
                "follow": follow,
                "tail": tail
            })),
        })
    }
}