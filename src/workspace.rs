use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub root_path: PathBuf,
    pub project_type: Option<String>,
    pub included_files: Vec<PathBuf>,
    pub excluded_patterns: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: String,
    pub last_updated: String,
}

impl WorkspaceContext {
    pub fn new(root_path: PathBuf) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            root_path,
            project_type: None,
            included_files: Vec::new(),
            excluded_patterns: vec![
                ".git/**".to_string(),
                "target/**".to_string(),
                "node_modules/**".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
                "*.pyc".to_string(),
                "__pycache__/**".to_string(),
            ],
            metadata: HashMap::new(),
            created_at: now.clone(),
            last_updated: now,
        }
    }

    pub fn detect_project_type(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Multi-language project detection with better priority
        let mut detected_types = Vec::new();
        
        // Rust
        if self.root_path.join("Cargo.toml").exists() {
            detected_types.push(("rust", "cargo"));
        }
        
        // JavaScript/TypeScript
        if self.root_path.join("package.json").exists() {
            detected_types.push(("javascript", "npm"));
            
            // Check for TypeScript
            if self.root_path.join("tsconfig.json").exists() {
                detected_types.push(("typescript", "npm"));
            }
        }
        
        // Python
        if self.root_path.join("pyproject.toml").exists() {
            detected_types.push(("python", "pip"));
        } else if self.root_path.join("setup.py").exists() {
            detected_types.push(("python", "pip"));
        } else if self.root_path.join("requirements.txt").exists() {
            detected_types.push(("python", "pip"));
        }
        
        // Go
        if self.root_path.join("go.mod").exists() {
            detected_types.push(("go", "go"));
        }
        
        // Java
        if self.root_path.join("pom.xml").exists() {
            detected_types.push(("java", "maven"));
        } else if self.root_path.join("build.gradle").exists() || self.root_path.join("build.gradle.kts").exists() {
            detected_types.push(("java", "gradle"));
        }
        
        // C/C++
        if self.root_path.join("CMakeLists.txt").exists() {
            detected_types.push(("cpp", "cmake"));
        } else if self.root_path.join("Makefile").exists() {
            detected_types.push(("c", "make"));
        }
        
        // C#
        if self.root_path.join("*.sln").exists() || self.root_path.join("*.csproj").exists() {
            detected_types.push(("csharp", "dotnet"));
        }
        
        // PHP
        if self.root_path.join("composer.json").exists() {
            detected_types.push(("php", "composer"));
        }
        
        // Ruby
        if self.root_path.join("Gemfile").exists() {
            detected_types.push(("ruby", "bundle"));
        }
        
        // Dart/Flutter
        if self.root_path.join("pubspec.yaml").exists() {
            detected_types.push(("dart", "pub"));
        }
        
        // Swift
        if self.root_path.join("Package.swift").exists() {
            detected_types.push(("swift", "swift"));
        }
        
        // Kotlin
        if self.root_path.join("build.gradle.kts").exists() {
            detected_types.push(("kotlin", "gradle"));
        }
        
        // Select the primary project type (first detected has priority)
        if let Some((project_type, build_tool)) = detected_types.first() {
            self.project_type = Some(project_type.to_string());
            self.metadata.insert("build_tool".to_string(), build_tool.to_string());
            
            // Add all detected types as metadata
            if detected_types.len() > 1 {
                let additional_types: Vec<String> = detected_types.iter()
                    .skip(1)
                    .map(|(t, _)| t.to_string())
                    .collect();
                self.metadata.insert("additional_types".to_string(), additional_types.join(","));
            }
        } else {
            self.project_type = Some("unknown".to_string());
        }
        
        // Detect framework-specific configurations
        self.detect_frameworks()?;
        
        Ok(())
    }
    
    fn detect_frameworks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut frameworks = Vec::new();
        
        // React
        if self.root_path.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(self.root_path.join("package.json")) {
                if content.contains("\"react\"") {
                    frameworks.push("react");
                }
                if content.contains("\"next\"") {
                    frameworks.push("nextjs");
                }
                if content.contains("\"vue\"") {
                    frameworks.push("vue");
                }
                if content.contains("\"angular\"") {
                    frameworks.push("angular");
                }
                if content.contains("\"svelte\"") {
                    frameworks.push("svelte");
                }
            }
        }
        
        // Python frameworks
        if self.root_path.join("requirements.txt").exists() {
            if let Ok(content) = std::fs::read_to_string(self.root_path.join("requirements.txt")) {
                if content.contains("django") {
                    frameworks.push("django");
                }
                if content.contains("flask") {
                    frameworks.push("flask");
                }
                if content.contains("fastapi") {
                    frameworks.push("fastapi");
                }
            }
        }
        
        // Rust frameworks
        if self.root_path.join("Cargo.toml").exists() {
            if let Ok(content) = std::fs::read_to_string(self.root_path.join("Cargo.toml")) {
                if content.contains("actix-web") {
                    frameworks.push("actix-web");
                }
                if content.contains("rocket") {
                    frameworks.push("rocket");
                }
                if content.contains("warp") {
                    frameworks.push("warp");
                }
                if content.contains("axum") {
                    frameworks.push("axum");
                }
            }
        }
        
        if !frameworks.is_empty() {
            self.metadata.insert("frameworks".to_string(), frameworks.join(","));
        }
        
        Ok(())
    }

    pub fn scan_project(&mut self, include_hidden: bool) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Scanning project at: {}", "🔍".cyan(), self.root_path.display());
        
        self.included_files.clear();
        
        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let relative_path = path.strip_prefix(&self.root_path)?;
            
            // Skip hidden files unless specified
            if !include_hidden && self.is_hidden_file(relative_path) {
                continue;
            }

            // Skip excluded patterns
            if self.should_exclude(relative_path) {
                continue;
            }

            // Only include text files
            if self.is_text_file(path) {
                self.included_files.push(relative_path.to_path_buf());
            }
        }

        self.last_updated = chrono::Utc::now().to_rfc3339();
        println!("{} Found {} files", "✅".green(), self.included_files.len());
        
        Ok(())
    }

    pub fn add_files(&mut self, files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for file in files {
            let path = Path::new(file);
            if path.exists() {
                let relative_path = if path.is_absolute() {
                    path.strip_prefix(&self.root_path)?.to_path_buf()
                } else {
                    path.to_path_buf()
                };
                
                if !self.included_files.contains(&relative_path) {
                    self.included_files.push(relative_path);
                }
            }
        }
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn remove_files(&mut self, files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for file in files {
            let path = Path::new(file);
            let relative_path = if path.is_absolute() {
                path.strip_prefix(&self.root_path)?.to_path_buf()
            } else {
                path.to_path_buf()
            };
            
            self.included_files.retain(|p| p != &relative_path);
        }
        self.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn get_file_contents(&self) -> Result<HashMap<PathBuf, String>, Box<dyn std::error::Error>> {
        let mut contents = HashMap::new();
        
        for file_path in &self.included_files {
            let full_path = self.root_path.join(file_path);
            if full_path.exists() {
                match fs::read_to_string(&full_path) {
                    Ok(content) => {
                        contents.insert(file_path.clone(), content);
                    }
                    Err(e) => {
                        eprintln!("{} Failed to read {}: {}", "⚠️".yellow(), file_path.display(), e);
                    }
                }
            }
        }
        
        Ok(contents)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let context: WorkspaceContext = serde_json::from_str(&content)?;
        Ok(context)
    }

    fn is_hidden_file(&self, path: &Path) -> bool {
        path.components().any(|comp| {
            comp.as_os_str().to_string_lossy().starts_with('.')
        })
    }

    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.excluded_patterns.iter().any(|pattern| {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                glob_pattern.matches(&path_str)
            } else {
                path_str.contains(pattern)
            }
        })
    }

    fn is_text_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                matches!(ext_str, 
                    "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "html" | "css" | "scss" | "sass" |
                    "json" | "toml" | "yaml" | "yml" | "md" | "txt" | "csv" | "xml" | "svg" |
                    "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "php" | "rb" | "kt" |
                    "swift" | "dart" | "scala" | "clj" | "sh" | "bash" | "zsh" | "fish" |
                    "sql" | "dockerfile" | "makefile" | "cmake" | "gradle" | "properties" |
                    "config" | "conf" | "ini" | "env" | "log"
                )
            } else {
                false
            }
        } else {
            // Check for files without extension that might be text
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    matches!(filename_str, 
                        "README" | "LICENSE" | "CHANGELOG" | "CONTRIBUTING" | "Makefile" |
                        "Dockerfile" | "docker-compose.yml" | "docker-compose.yaml" |
                        ".gitignore" | ".dockerignore" | ".env" | ".env.example"
                    )
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}

pub struct WorkspaceManager {
    context: Option<WorkspaceContext>,
    context_file: PathBuf,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let context_file = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ollama-cli-assistant")
            .join("workspace.json");
        
        Self {
            context: None,
            context_file,
        }
    }

    pub fn init_workspace(&mut self, path: Option<String>, project_type: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_path = if let Some(path) = path {
            PathBuf::from(path)
        } else {
            std::env::current_dir()?
        };

        if !workspace_path.exists() {
            return Err("Workspace path does not exist".into());
        }

        let mut context = WorkspaceContext::new(workspace_path);
        
        if let Some(proj_type) = project_type {
            context.project_type = Some(proj_type);
        } else {
            context.detect_project_type()?;
        }

        // Create config directory if it doesn't exist
        if let Some(parent) = self.context_file.parent() {
            fs::create_dir_all(parent)?;
        }

        context.save_to_file(&self.context_file)?;
        self.context = Some(context);

        println!("{} Workspace initialized at: {}", "✅".green(), self.context.as_ref().unwrap().root_path.display());
        
        Ok(())
    }

    pub fn load_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.context_file.exists() {
            self.context = Some(WorkspaceContext::load_from_file(&self.context_file)?);
        }
        Ok(())
    }

    pub fn get_context(&self) -> Option<&WorkspaceContext> {
        self.context.as_ref()
    }

    pub fn get_context_mut(&mut self) -> Option<&mut WorkspaceContext> {
        self.context.as_mut()
    }

    pub fn save_context(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(context) = &self.context {
            context.save_to_file(&self.context_file)?;
        }
        Ok(())
    }

    pub fn clear_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.context = None;
        if self.context_file.exists() {
            fs::remove_file(&self.context_file)?;
        }
        Ok(())
    }
}