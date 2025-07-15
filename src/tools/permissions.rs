use colored::Colorize;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::collections::HashMap;

use super::core::AvailableTool;

pub struct PermissionManager {
    auto_approve_safe: bool,
    session_approvals: HashMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq)]
enum RiskLevel {
    Safe,
    Moderate,
    Dangerous,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            auto_approve_safe: true,
            session_approvals: HashMap::new(),
        }
    }

    pub fn request_permission(
        &mut self,
        tool: &AvailableTool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let (action_desc, risk_level) = self.describe_action(tool);

        if self.auto_approve_safe && risk_level == RiskLevel::Safe {
            println!("{} {}", "✓".green(), action_desc.dimmed());
            return Ok(true);
        }

        let action_key = self.get_action_key(tool);
        if let Some(&approved) = self.session_approvals.get(&action_key) {
            if approved {
                println!(
                    "{} {} {}",
                    "✓".green(),
                    action_desc.dimmed(),
                    "(previously approved)".dimmed()
                );
                return Ok(true);
            } else {
                println!(
                    "{} {} {}",
                    "✗".red(),
                    action_desc.dimmed(),
                    "(previously denied)".dimmed()
                );
                return Ok(false);
            }
        }

        self.show_action_preview(tool);

        let prompt = match risk_level {
            RiskLevel::Safe => format!("{} Execute this action?", "🔵".blue()),
            RiskLevel::Moderate => format!("{} Execute this action?", "🟡".yellow()),
            RiskLevel::Dangerous => {
                format!("{} Execute this POTENTIALLY DANGEROUS action?", "🔴".red())
            }
        };

        let approved = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(risk_level == RiskLevel::Safe)
            .interact()?;

        self.session_approvals.insert(action_key, approved);

        if approved {
            println!("{} Action approved", "✓".green());
        } else {
            println!("{} Action denied", "✗".red());
        }

        Ok(approved)
    }

    fn describe_action(&self, tool: &AvailableTool) -> (String, RiskLevel) {
        match tool {
            AvailableTool::WebSearch { query } => {
                (format!("Search web for: '{}'", query), RiskLevel::Safe)
            }
            AvailableTool::WebScrape { url } => {
                (format!("Scrape content from: {}", url), RiskLevel::Safe)
            }
            AvailableTool::FileSearch { pattern, directory } => (
                format!(
                    "Search for files matching '{}' in {}",
                    pattern,
                    directory.as_deref().unwrap_or("current directory")
                ),
                RiskLevel::Safe,
            ),
            AvailableTool::FileRead { path } => (format!("Read file: {}", path), RiskLevel::Safe),
            AvailableTool::FileWrite { path, content: _ } => {
                (format!("Write to file: {}", path), RiskLevel::Moderate)
            }
            AvailableTool::FileEdit { path, operation: _ } => {
                (format!("Edit file: {}", path), RiskLevel::Moderate)
            }
            AvailableTool::ContentSearch { pattern, directory } => (
                format!(
                    "Search for content '{}' in {}",
                    pattern,
                    directory.as_deref().unwrap_or("current directory")
                ),
                RiskLevel::Safe,
            ),
            AvailableTool::CreateProject {
                name,
                project_type,
                path,
            } => (
                format!(
                    "Create {} project '{}' in {}",
                    project_type,
                    name,
                    path.as_deref().unwrap_or("current directory")
                ),
                RiskLevel::Moderate,
            ),
            AvailableTool::ExecuteCommand { command } => (
                format!("Execute command: {}", command),
                RiskLevel::Dangerous,
            ),
            AvailableTool::ListDirectory { path } => {
                (format!("List directory: {}", path), RiskLevel::Safe)
            }
        }
    }

    fn show_action_preview(&self, tool: &AvailableTool) {
        println!();
        println!("{}", "📋 Action Details:".cyan().bold());

        match tool {
            AvailableTool::WebSearch { query } => {
                println!("  {} {}", "Type:".blue(), "Web Search");
                println!("  {} {}", "Query:".blue(), query.yellow());
                println!("  {} Search engines for information", "Effect:".blue());
            }
            AvailableTool::FileWrite { path, content } => {
                println!("  {} {}", "Type:".blue(), "File Write");
                println!("  {} {}", "Path:".blue(), path.yellow());
                println!("  {} {} bytes", "Size:".blue(), content.len());
                println!("  {} Create or overwrite file", "Effect:".blue());

                if content.len() < 200 {
                    println!(
                        "  {} {}",
                        "Preview:".blue(),
                        content
                            .lines()
                            .take(3)
                            .collect::<Vec<_>>()
                            .join("\\n")
                            .dimmed()
                    );
                }
            }
            AvailableTool::ExecuteCommand { command } => {
                println!("  {} {}", "Type:".blue(), "System Command".red());
                println!("  {} {}", "Command:".blue(), command.yellow());
                println!(
                    "  {} Execute system command with full privileges",
                    "Effect:".blue()
                );
                println!(
                    "  {} This could modify files, install software, or affect system",
                    "Warning:".red().bold()
                );
            }
            _ => {
                println!("  {} {:?}", "Type:".blue(), tool);
            }
        }
        println!();
    }

    fn get_action_key(&self, tool: &AvailableTool) -> String {
        match tool {
            AvailableTool::WebSearch { .. } => "web_search".to_string(),
            AvailableTool::WebScrape { url } => format!("web_scrape:{}", url),
            AvailableTool::FileWrite { path, .. } => format!("file_write:{}", path),
            AvailableTool::ExecuteCommand { command } => format!("execute_command:{}", command),
            _ => format!("{:?}", std::mem::discriminant(tool)),
        }
    }
}
