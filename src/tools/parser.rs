use super::core::{AvailableTool, EditOperation};
use regex::Regex;

pub struct NaturalLanguageParser {
    web_patterns: Vec<Regex>,
    file_patterns: Vec<Regex>,
    project_patterns: Vec<Regex>,
    command_patterns: Vec<Regex>,
}

impl NaturalLanguageParser {
    pub fn new() -> Self {
        Self {
            web_patterns: vec![
                Regex::new(r"(?i)search\s+(?:for\s+|about\s+)?(.+?)(?:\s+on\s+(?:web|internet|google))?$").unwrap(),
                Regex::new(r"(?i)(?:find|get|tell me about|what is|who is)\s+(.+?)(?:\s+(?:from|on)\s+(?:web|internet|wiki))?").unwrap(),
                Regex::new(r"(?i)(?:information|info)\s+about\s+(.+?)(?:\s+from\s+(?:web|internet))?").unwrap(),
                Regex::new(r"(?i)scrape\s+(?:the\s+)?(?:website\s+|url\s+|page\s+)?(.+)").unwrap(),
            ],
            file_patterns: vec![
                Regex::new(r"(?i)(?:find|search|look for)\s+(?:files?\s+)?(?:named\s+|called\s+)?(.+?)(?:\s+in\s+(.+?))?$").unwrap(),
                Regex::new(r"(?i)(?:read|show|open|cat)\s+(?:the\s+)?(?:file\s+)?(.+)").unwrap(),
                Regex::new(r"(?i)(?:write|create|make)\s+(?:a\s+)?(?:file\s+)?(.+?)(?:\s+with\s+content\s+(.+))?").unwrap(),
                Regex::new(r"(?i)(?:edit|modify|change)\s+(?:the\s+)?(?:file\s+)?(.+)").unwrap(),
                Regex::new(r#"(?i)(?:search|find|grep)\s+(?:for\s+)?["'](.+?)["'](?:\s+in\s+(.+?))?"#).unwrap(),
                Regex::new(r"(?i)(?:list|show)\s+(?:the\s+)?(?:contents?\s+of\s+)?(?:directory\s+)?(.+)").unwrap(),
            ],
            project_patterns: vec![
                Regex::new(r"(?i)(?:create|make|generate|start)\s+(?:a\s+)?(?:new\s+)?(.+?)\s+project\s+(?:named\s+|called\s+)?(.+?)(?:\s+in\s+(.+?))?").unwrap(),
                Regex::new(r"(?i)(?:initialize|init)\s+(?:a\s+)?(.+?)\s+project\s+(.+?)").unwrap(),
            ],
            command_patterns: vec![
                Regex::new(r"(?i)(?:run|execute|cmd)\s+(.+)").unwrap(),
                Regex::new(r"(?i)(?:install|update|upgrade)\s+(.+)").unwrap(),
                Regex::new(r"(?i)(?:git\s+|npm\s+|cargo\s+|pip\s+)(.+)").unwrap(),
            ],
        }
    }

    pub fn parse_request(&self, input: &str) -> Vec<AvailableTool> {
        let mut tools = Vec::new();

        // Try web search patterns
        for pattern in &self.web_patterns {
            if let Some(captures) = pattern.captures(input) {
                let query = captures.get(1).unwrap().as_str().trim();

                if query.starts_with("http") || input.to_lowercase().contains("scrape") {
                    tools.push(AvailableTool::WebScrape {
                        url: query.to_string(),
                    });
                } else {
                    tools.push(AvailableTool::WebSearch {
                        query: query.to_string(),
                    });
                }
                break;
            }
        }

        // Try file operation patterns
        for pattern in &self.file_patterns {
            if let Some(captures) = pattern.captures(input) {
                let main_param = captures.get(1).unwrap().as_str().trim();
                let optional_param = captures.get(2).map(|m| m.as_str().trim());

                if input.to_lowercase().contains("read")
                    || input.to_lowercase().contains("show")
                    || input.to_lowercase().contains("cat")
                {
                    tools.push(AvailableTool::FileRead {
                        path: main_param.to_string(),
                    });
                } else if input.to_lowercase().contains("write")
                    || input.to_lowercase().contains("create")
                {
                    let content = optional_param.unwrap_or("").to_string();
                    tools.push(AvailableTool::FileWrite {
                        path: main_param.to_string(),
                        content,
                    });
                } else if input.to_lowercase().contains("edit")
                    || input.to_lowercase().contains("modify")
                    || input.to_lowercase().contains("change")
                {
                    let operation = self.parse_edit_operation(input, main_param);
                    tools.push(AvailableTool::FileEdit {
                        path: main_param.to_string(),
                        operation,
                    });
                } else if input.to_lowercase().contains("search")
                    && (input.contains("\"") || input.contains("'"))
                {
                    tools.push(AvailableTool::ContentSearch {
                        pattern: main_param.to_string(),
                        directory: optional_param.map(|s| s.to_string()),
                    });
                } else if input.to_lowercase().contains("list") {
                    tools.push(AvailableTool::ListDirectory {
                        path: main_param.to_string(),
                    });
                } else {
                    tools.push(AvailableTool::FileSearch {
                        pattern: main_param.to_string(),
                        directory: optional_param.map(|s| s.to_string()),
                    });
                }
                break;
            }
        }

        // Try project creation patterns
        for pattern in &self.project_patterns {
            if let Some(captures) = pattern.captures(input) {
                let project_type = captures.get(1).unwrap().as_str().trim();
                let name = captures.get(2).unwrap().as_str().trim();
                let path = captures.get(3).map(|m| m.as_str().trim().to_string());

                tools.push(AvailableTool::CreateProject {
                    name: name.to_string(),
                    project_type: project_type.to_string(),
                    path,
                });
                break;
            }
        }

        // Try command execution patterns
        for pattern in &self.command_patterns {
            if let Some(captures) = pattern.captures(input) {
                let command = if input.to_lowercase().starts_with("run ")
                    || input.to_lowercase().starts_with("execute ")
                {
                    captures.get(1).unwrap().as_str().trim().to_string()
                } else {
                    input.trim().to_string()
                };

                tools.push(AvailableTool::ExecuteCommand { command });
                break;
            }
        }

        tools
    }

    fn parse_edit_operation(&self, input: &str, _file_path: &str) -> EditOperation {
        if let Ok(replace_regex) =
            Regex::new(r#"(?i)replace\s+["'](.+?)["']\s+with\s+["'](.+?)["']"#)
        {
            if let Some(captures) = replace_regex.captures(input) {
                let old = captures.get(1).unwrap().as_str().to_string();
                let new = captures.get(2).unwrap().as_str().to_string();
                return EditOperation::Replace { old, new };
            }
        }

        if let Ok(insert_regex) = Regex::new(r"(?i)insert\s+(.+?)\s+at\s+line\s+(\d+)") {
            if let Some(captures) = insert_regex.captures(input) {
                let content = captures.get(1).unwrap().as_str().to_string();
                let line = captures
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap_or(1);
                return EditOperation::Insert { line, content };
            }
        }

        if input.to_lowercase().contains("append") || input.to_lowercase().contains("add to end") {
            if let Ok(append_regex) = Regex::new(r#"(?i)(?:append|add)\s+["'](.+?)["']"#) {
                if let Some(captures) = append_regex.captures(input) {
                    let content = captures.get(1).unwrap().as_str().to_string();
                    return EditOperation::Append { content };
                }
            }
        }

        EditOperation::Append {
            content: "# Edit operation could not be parsed".to_string(),
        }
    }

    pub fn suggest_clarification(&self, input: &str) -> Option<String> {
        let lower_input = input.to_lowercase();

        if lower_input.contains("file") {
            Some("I can help with file operations. Try: 'read file.txt', 'search for *.rs files', or 'create file.txt with content Hello'".to_string())
        } else if lower_input.contains("search") {
            Some("I can search the web or files. Try: 'search for rust programming' or 'search for \"function main\" in src/'".to_string())
        } else if lower_input.contains("project") {
            Some("I can create projects. Try: 'create a rust project called my-app' or 'make a python project named calculator'".to_string())
        } else {
            None
        }
    }
}
