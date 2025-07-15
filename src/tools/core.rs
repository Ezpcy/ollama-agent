use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Tool definition system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub command: Option<String>,
    pub requires_permission: bool,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

// Available tools enum
#[derive(Debug, Clone)]
pub enum AvailableTool {
    WebSearch {
        query: String,
    },
    WebScrape {
        url: String,
    },
    FileSearch {
        pattern: String,
        directory: Option<String>,
    },
    FileRead {
        path: String,
    },
    FileWrite {
        path: String,
        content: String,
    },
    FileEdit {
        path: String,
        operation: EditOperation,
    },
    ContentSearch {
        pattern: String,
        directory: Option<String>,
    },
    CreateProject {
        name: String,
        project_type: String,
        path: Option<String>,
    },
    ExecuteCommand {
        command: String,
    },
    ListDirectory {
        path: String,
    },
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    Replace {
        old: String,
        new: String,
    },
    Insert {
        line: usize,
        content: String,
    },
    Append {
        content: String,
    },
    Delete {
        line_start: usize,
        line_end: Option<usize>,
    },
}

// Tool executor
pub struct ToolExecutor {
    pub web_client: reqwest::Client,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            web_client: reqwest::Client::new(),
        }
    }

    pub async fn execute_tool(
        &self,
        tool: AvailableTool,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        match tool {
            AvailableTool::WebSearch { query } => self.web_search(&query).await,
            AvailableTool::WebScrape { url } => self.web_scrape(&url).await,
            AvailableTool::FileSearch { pattern, directory } => {
                self.file_search(&pattern, directory.as_deref())
            }
            AvailableTool::FileRead { path } => self.file_read(&path),
            AvailableTool::FileWrite { path, content } => self.file_write(&path, &content),
            AvailableTool::FileEdit { path, operation } => self.file_edit(&path, operation),
            AvailableTool::ContentSearch { pattern, directory } => {
                self.content_search(&pattern, directory.as_deref())
            }
            AvailableTool::CreateProject {
                name,
                project_type,
                path,
            } => self.create_project(&name, &project_type, path.as_deref()),
            AvailableTool::ExecuteCommand { command } => self.execute_command(&command).await,
            AvailableTool::ListDirectory { path } => self.list_directory(&path),
        }
    }
}
