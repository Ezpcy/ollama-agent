use super::core::{GitBranchOperation, ToolExecutor, ToolResult};
use colored::Colorize;
use std::process::Command;

impl ToolExecutor {
    pub async fn git_status(
        &self,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Checking git status", "📋".cyan());

        let mut cmd = Command::new("git");
        cmd.arg("status").arg("--porcelain");

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
        }

        let output = cmd.output()?;

        if !output.status.success() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                metadata: None,
            });
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let formatted_output = if status_output.trim().is_empty() {
            "Working tree clean".to_string()
        } else {
            format!("Modified files:\n{}", status_output)
        };

        Ok(ToolResult {
            success: true,
            output: formatted_output,
            error: None,
            metadata: None,
        })
    }

    pub async fn git_add(
        &self,
        files: &[String],
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Adding files to git: {}",
            "➕".cyan(),
            files.join(", ").yellow()
        );

        let mut cmd = Command::new("git");
        cmd.arg("add");

        for file in files {
            cmd.arg(file);
        }

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
        }

        let output = cmd.output()?;

        Ok(ToolResult {
            success: output.status.success(),
            output: if output.status.success() {
                format!("Successfully added {} file(s)", files.len())
            } else {
                String::new()
            },
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
            metadata: None,
        })
    }

    pub async fn git_commit(
        &self,
        message: &str,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Creating commit: {}", "💾".cyan(), message.yellow());

        let mut cmd = Command::new("git");
        cmd.arg("commit").arg("-m").arg(message);

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
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
            metadata: None,
        })
    }

    pub async fn git_push(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let remote = remote.unwrap_or(&self.config.git_default_remote);
        let branch = branch.unwrap_or("HEAD");

        println!(
            "{} Pushing to {}/{}",
            "⬆️".cyan(),
            remote.yellow(),
            branch.yellow()
        );

        let mut cmd = Command::new("git");
        cmd.arg("push").arg(remote).arg(branch);

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
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
            metadata: None,
        })
    }

    pub async fn git_pull(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let remote = remote.unwrap_or(&self.config.git_default_remote);

        println!("{} Pulling from {}", "⬇️".cyan(), remote.yellow());

        let mut cmd = Command::new("git");
        cmd.arg("pull").arg(remote);

        if let Some(branch) = branch {
            cmd.arg(branch);
        }

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
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
            metadata: None,
        })
    }

    pub async fn git_branch(
        &self,
        operation: GitBranchOperation,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("git");

        match operation {
            GitBranchOperation::List => {
                println!("{} Listing branches", "🌿".cyan());
                cmd.arg("branch").arg("-a");
            }
            GitBranchOperation::Create { ref name } => {
                println!("{} Creating branch: {}", "🌱".cyan(), name.yellow());
                cmd.arg("checkout").arg("-b").arg(name);
            }
            GitBranchOperation::Switch { ref name } => {
                println!("{} Switching to branch: {}", "🔄".cyan(), name.yellow());
                cmd.arg("checkout").arg(name);
            }
            GitBranchOperation::Delete { ref name } => {
                println!("{} Deleting branch: {}", "🗑️".cyan(), name.yellow());
                cmd.arg("branch").arg("-d").arg(name);
            }
            GitBranchOperation::Merge { ref from } => {
                println!("{} Merging from branch: {}", "🔀".cyan(), from.yellow());
                cmd.arg("merge").arg(from);
            }
        }

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
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
            metadata: None,
        })
    }

    pub async fn git_log(
        &self,
        count: Option<u32>,
        oneline: bool,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting git log", "📜".cyan());

        let mut cmd = Command::new("git");
        cmd.arg("log");

        if let Some(count) = count {
            cmd.arg(format!("-{}", count));
        }

        if oneline {
            cmd.arg("--oneline");
        } else {
            cmd.arg("--pretty=format:%h - %an, %ar : %s");
        }

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
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
            metadata: None,
        })
    }

    pub async fn git_diff(
        &self,
        file: Option<&str>,
        cached: bool,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting git diff", "📊".cyan());

        let mut cmd = Command::new("git");
        cmd.arg("diff");

        if cached {
            cmd.arg("--cached");
        }

        if let Some(file) = file {
            cmd.arg(file);
        }

        // Validate repository_path before using it
        if let Some(path) = repository_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                cmd.current_dir(path);
            }
            // If path is invalid, just use current directory (no current_dir call)
        }

        let output = cmd.output()?;

        Ok(ToolResult {
            success: output.status.success(),
            output: if output.stdout.is_empty() {
                "No differences found".to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).to_string()
            },
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
            metadata: None,
        })
    }
}
