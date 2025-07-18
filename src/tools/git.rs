use super::core::{GitBranchOperation, ToolExecutor, ToolResult};
use colored::Colorize;
use std::process::Command;

impl ToolExecutor {
    pub async fn git_status(
        &self,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting git status", "📊".cyan());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.arg("status");

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
            error: if success { None } else { Some("Git status command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "command": "status"
            })),
        })
    }

    pub async fn git_add(
        &self,
        files: &[String],
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Adding files to git: {:?}", "➕".cyan(), files);

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.arg("add");
        cmd.args(files);

        let output = cmd.output()?;
        let success = output.status.success();
        
        let output_text = if success {
            format!("Successfully added {} files to git", files.len())
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };

        Ok(ToolResult {
            success,
            output: output_text,
            error: if success { None } else { Some("Git add command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "files": files,
                "command": "add"
            })),
        })
    }

    pub async fn git_commit(
        &self,
        message: &str,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Committing with message: {}", "💾".cyan(), message.yellow());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.args(&["commit", "-m", message]);

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
            error: if success { None } else { Some("Git commit command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "message": message,
                "command": "commit"
            })),
        })
    }

    pub async fn git_push(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let remote_name = remote.unwrap_or("origin");
        println!("{} Pushing to remote: {}", "🚀".cyan(), remote_name.yellow());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.args(&["push", remote_name]);
        
        if let Some(branch_name) = branch {
            cmd.arg(branch_name);
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
            error: if success { None } else { Some("Git push command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "remote": remote_name,
                "branch": branch,
                "command": "push"
            })),
        })
    }

    pub async fn git_pull(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        let remote_name = remote.unwrap_or("origin");
        println!("{} Pulling from remote: {}", "⬇️".cyan(), remote_name.yellow());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.args(&["pull", remote_name]);
        
        if let Some(branch_name) = branch {
            cmd.arg(branch_name);
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
            error: if success { None } else { Some("Git pull command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "remote": remote_name,
                "branch": branch,
                "command": "pull"
            })),
        })
    }

    pub async fn git_branch(
        &self,
        operation: GitBranchOperation,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Git branch operation: {:?}", "🌿".cyan(), operation);

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        match operation {
            GitBranchOperation::List => {
                cmd.args(&["branch", "-a"]);
            }
            GitBranchOperation::Create { ref name } => {
                cmd.args(&["branch", name]);
            }
            GitBranchOperation::Switch { ref name } => {
                cmd.args(&["checkout", name]);
            }
            GitBranchOperation::Delete { ref name } => {
                cmd.args(&["branch", "-d", name]);
            }
            GitBranchOperation::Merge { ref from } => {
                cmd.args(&["merge", from]);
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
            error: if success { None } else { Some("Git branch command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "operation": format!("{:?}", operation),
                "command": "branch"
            })),
        })
    }

    pub async fn git_log(
        &self,
        count: Option<u32>,
        oneline: bool,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting git log", "📋".cyan());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.arg("log");
        
        if oneline {
            cmd.arg("--oneline");
        }
        
        if let Some(n) = count {
            cmd.arg(format!("-{}", n));
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
            error: if success { None } else { Some("Git log command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "count": count,
                "oneline": oneline,
                "command": "log"
            })),
        })
    }

    pub async fn git_diff(
        &self,
        file: Option<&str>,
        cached: bool,
        repository_path: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Getting git diff", "🔍".cyan());

        let mut cmd = Command::new("git");
        
        if let Some(repo_path) = repository_path {
            cmd.args(&["-C", repo_path]);
        }
        
        cmd.arg("diff");
        
        if cached {
            cmd.arg("--cached");
        }
        
        if let Some(file_path) = file {
            cmd.arg(file_path);
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
            error: if success { None } else { Some("Git diff command failed".to_string()) },
            metadata: Some(serde_json::json!({
                "repository_path": repository_path,
                "file": file,
                "cached": cached,
                "command": "diff"
            })),
        })
    }
}