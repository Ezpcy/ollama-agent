use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod client;
mod input;
mod session;
mod tools;
mod workspace;

use client::{
    check_ollama_health, delete_model, fetch_models, list_models_filtered, pull_model,
    select_model, show_model_info, SelectedModel,
};
use session::AssistantSession;
use tools::{ToolConfig, ToolExecutor};
use workspace::WorkspaceManager;

mod api_models {
}

#[derive(Parser)]
#[command(name = "ollama-cli-assistant")]
#[command(about = "AI-powered coding assistant with system tools")]
#[command(version = "0.3.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Skip model selection and use specified model
    #[arg(short, long)]
    model: Option<String>,

    /// Run a single command and exit
    #[arg(short, long)]
    execute: Option<String>,

    /// Enable vim mode for input
    #[arg(long)]
    vim: bool,

    /// Include files in context (similar to Claude CLI)
    #[arg(long)]
    files: Vec<String>,

    /// Set working directory
    #[arg(long)]
    working_dir: Option<String>,

    /// Stream output (default: true)
    #[arg(long)]
    no_stream: bool,

    /// Enable project context scanning
    #[arg(long)]
    project_context: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive chat session (default if no command specified)
    Chat {
        /// Model to use for the session
        #[arg(short, long)]
        model: Option<String>,

        /// Enable vim mode for input
        #[arg(long)]
        vim: bool,

        /// Include files in context
        #[arg(long)]
        files: Vec<String>,

        /// Enable project context
        #[arg(long)]
        project_context: bool,
    },
    /// Ask a question and get a response (non-interactive)
    Ask {
        /// The question to ask
        prompt: String,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,

        /// Include files in context
        #[arg(long)]
        files: Vec<String>,

        /// Enable project context
        #[arg(long)]
        project_context: bool,
    },
    /// Generate code based on description
    Generate {
        /// Code description
        description: String,

        /// Programming language
        #[arg(short, long)]
        language: Option<String>,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Edit files interactively
    Edit {
        /// Files to edit
        files: Vec<String>,

        /// Edit instruction
        #[arg(short, long)]
        instruction: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Review code and provide feedback
    Review {
        /// Files to review
        files: Vec<String>,

        /// Review focus (bugs, style, performance, etc.)
        #[arg(short, long)]
        focus: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Commit changes with AI-generated message
    Commit {
        /// Additional context for commit message
        #[arg(short, long)]
        context: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Initialize project context
    Init {
        /// Project path
        #[arg(short, long)]
        path: Option<String>,

        /// Project type
        #[arg(short, long)]
        project_type: Option<String>,
    },
    /// List available models
    List {
        /// Filter models by name or family
        #[arg(short, long)]
        filter: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Pull a model from the Ollama registry
    Pull {
        /// Model name to pull
        model: String,
    },
    /// Delete a model
    Delete {
        /// Model name to delete
        model: String,
    },
    /// Show model information
    Show {
        /// Model name to show
        model: String,
    },
    /// Check system status and available tools
    Status,
    /// Run system diagnostics
    Diagnostics,
    /// Discover available tools and system capabilities
    Discover,
    /// Configuration management
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
    /// Execute a specific tool directly
    Tool {
        #[command(subcommand)]
        tool_command: ToolCommands,
    },
    /// Manage conversation history
    History {
        #[command(subcommand)]
        history_command: HistoryCommands,
    },
    /// Manage workspace and project context
    Workspace {
        #[command(subcommand)]
        workspace_command: WorkspaceCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
    /// Export configuration
    Export {
        /// Output file path
        path: String,
    },
}

#[derive(Subcommand)]
enum HistoryCommands {
    /// Show conversation history
    Show {
        /// Number of recent entries to show
        #[arg(short, long, default_value = "10")]
        count: usize,
        
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Clear conversation history
    Clear {
        /// Clear all history
        #[arg(short, long)]
        all: bool,
    },
    /// Export conversation history
    Export {
        /// Output file path
        path: String,
        
        /// Export format (json, markdown, text)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Search conversation history
    Search {
        /// Search query
        query: String,
        
        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommands {
    /// Initialize workspace
    Init {
        /// Workspace path
        path: Option<String>,
        
        /// Project type
        #[arg(short, long)]
        project_type: Option<String>,
    },
    /// Show workspace info
    Info,
    /// Scan project for context
    Scan {
        /// Path to scan
        path: Option<String>,
        
        /// Include hidden files
        #[arg(long)]
        include_hidden: bool,
    },
    /// Add files to workspace context
    Add {
        /// Files to add
        files: Vec<String>,
    },
    /// Remove files from workspace context
    Remove {
        /// Files to remove
        files: Vec<String>,
    },
    /// List workspace context
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Clear workspace context
    Clear,
}

#[derive(Subcommand)]
enum ToolCommands {
    /// Git operations
    Git {
        #[command(subcommand)]
        git_command: GitCommands,
    },
    /// System information
    System {
        #[command(subcommand)]
        system_command: SystemCommands,
    },
    /// Docker operations
    Docker {
        #[command(subcommand)]
        docker_command: DockerCommands,
    },
    /// Package management
    Package {
        #[command(subcommand)]
        package_command: PackageCommands,
    },
    /// File operations
    File {
        #[command(subcommand)]
        file_command: FileCommands,
    },
}

#[derive(Subcommand)]
enum GitCommands {
    /// Show git status
    Status,
    /// Add files to git
    Add { files: Vec<String> },
    /// Commit changes
    Commit { message: String },
    /// Push changes
    Push,
    /// Pull changes
    Pull,
    /// Show git log
    Log {
        #[arg(short, long, default_value = "10")]
        count: u32,
    },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Show system information
    Info,
    /// Show memory usage
    Memory,
    /// Show disk usage
    Disk { path: Option<String> },
    /// List processes
    Processes { filter: Option<String> },
    /// Show network information
    Network,
}

#[derive(Subcommand)]
enum DockerCommands {
    /// List docker resources
    List {
        #[arg(value_enum, default_value = "containers")]
        resource: DockerResource,
    },
    /// Run a container
    Run {
        image: String,
        #[arg(short, long)]
        ports: Vec<String>,
        #[arg(short, long)]
        volumes: Vec<String>,
    },
    /// Stop a container
    Stop { container: String },
    /// Show container logs
    Logs {
        container: String,
        #[arg(short, long)]
        tail: Option<u32>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum DockerResource {
    Containers,
    Images,
    Volumes,
    Networks,
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Cargo operations
    Cargo {
        #[command(subcommand)]
        cargo_command: CargoCommands,
    },
    /// NPM operations
    Npm {
        #[command(subcommand)]
        npm_command: NpmCommands,
    },
    /// Check available package managers
    Check,
    /// Search for packages
    Search { query: String },
}

#[derive(Subcommand)]
enum CargoCommands {
    Build,
    Run,
    Test,
    Add { package: String },
    Remove { package: String },
}

#[derive(Subcommand)]
enum NpmCommands {
    Install { package: Option<String> },
    Uninstall { package: String },
    Run { script: String },
    List,
}

#[derive(Subcommand)]
enum FileCommands {
    /// Read a file
    Read { path: String },
    /// Write to a file
    Write { path: String, content: String },
    /// Search for files
    Search {
        pattern: String,
        directory: Option<String>,
    },
    /// List directory contents
    List { path: Option<String> },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging if verbose
    if cli.verbose {
        env_logger::init();
    }

    // Change working directory if specified
    if let Some(working_dir) = cli.working_dir {
        std::env::set_current_dir(&working_dir)?;
    }

    // Check if Ollama is running
    if !check_ollama_health().await? {
        eprintln!("{} Failed to connect to Ollama", "❌".red());
        eprintln!(
            "{} Make sure Ollama is running: ollama serve",
            "💡".yellow()
        );
        process::exit(1);
    }

    match cli.command {
        Some(Commands::Chat { model, vim, files, project_context }) => {
            start_chat_session_with_context(model, cli.config, vim, files, project_context).await?;
        }
        Some(Commands::Ask { prompt, model, files, project_context }) => {
            handle_ask_command(prompt, model, files, project_context).await?;
        }
        Some(Commands::Generate { description, language, output, model }) => {
            handle_generate_command(description, language, output, model).await?;
        }
        Some(Commands::Edit { files, instruction, model }) => {
            handle_edit_command(files, instruction, model).await?;
        }
        Some(Commands::Review { files, focus, model }) => {
            handle_review_command(files, focus, model).await?;
        }
        Some(Commands::Commit { context, model }) => {
            handle_commit_command(context, model).await?;
        }
        Some(Commands::Init { path, project_type }) => {
            handle_init_command(path, project_type).await?;
        }
        Some(Commands::List { filter, detailed }) => {
            list_models_command(filter, detailed).await?;
        }
        Some(Commands::Pull { model }) => {
            pull_model(&model).await?;
        }
        Some(Commands::Delete { model }) => {
            delete_model(&model).await?;
        }
        Some(Commands::Show { model }) => {
            show_model_info(&model).await?;
        }
        Some(Commands::Status) => {
            show_status().await?;
        }
        Some(Commands::Diagnostics) => {
            run_diagnostics().await?;
        }
        Some(Commands::Discover) => {
            run_tool_discovery().await?;
        }
        Some(Commands::Config { config_command }) => {
            handle_config_command(config_command).await?;
        }
        Some(Commands::Tool { tool_command }) => {
            handle_tool_command(tool_command).await?;
        }
        Some(Commands::History { history_command }) => {
            handle_history_command(history_command).await?;
        }
        Some(Commands::Workspace { workspace_command }) => {
            handle_workspace_command(workspace_command).await?;
        }
        None => {
            // No subcommand provided
            if let Some(command) = cli.execute {
                // Execute single command with context
                execute_single_command_with_context(&command, cli.model, cli.vim, cli.files, cli.project_context).await?;
            } else {
                // Default to interactive chat with context
                start_chat_session_with_context(cli.model, cli.config, cli.vim, cli.files, cli.project_context).await?;
            }
        }
    }

    Ok(())
}

async fn start_chat_session(
    model_name: Option<String>,
    _config_path: Option<String>,
    vim_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🚀 Advanced AI Assistant Startup".cyan().bold());
    println!();

    let models = fetch_models().await.map_err(|e| {
        println!("{} Failed to fetch models: {}", "❌".red(), e);
        e
    })?;

    if models.is_empty() {
        println!(
            "{} No models available. Install one with: ollama pull llama2",
            "⚠".yellow()
        );
        return Ok(());
    }

    let selected_model = if let Some(model_name) = model_name {
        // Use specified model
        if let Some(model) = models.iter().find(|m| m.name == model_name) {
            client::SelectedModel::from(model.clone())
        } else {
            println!("{} Model '{}' not found", "❌".red(), model_name);
            return Ok(());
        }
    } else {
        // Interactive selection
        select_model(&models)?
    };

    selected_model.display_info();

    // Initialize tool executor with enhanced configuration
    let tool_config = ToolConfig::default();
    let tool_executor = ToolExecutor::with_config(tool_config);

    // Start enhanced assistant session
    let mut session = AssistantSession::with_vim_mode(selected_model, tool_executor, vim_mode);

    if vim_mode {
        println!("{}", "Vim mode enabled! Use 'ESC' to enter normal mode, 'i' to enter insert mode.".green());
        println!("{}", "Type ':help' in command mode for vim commands.".dimmed());
    }

    session.run().await?;

    Ok(())
}

async fn list_models_command(
    filter: Option<String>,
    detailed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let models = list_models_filtered(filter.as_deref()).await?;

    if models.is_empty() {
        println!("{} No models found", "ℹ️".blue());
        return Ok(());
    }

    println!("{}", "Available Models:".cyan().bold());
    println!();

    for model in models {
        let model_type = if model.name.to_lowercase().contains("code") {
            "📝"
        } else if model.name.to_lowercase().contains("chat") {
            "💬"
        } else {
            "🤖"
        };

        let size_gb = model.size as f64 / 1_000_000_000.0;
        println!("{} {} ({:.1} GB)", model_type, model.name.yellow(), size_gb);

        if detailed {
            if let Some(details) = &model.details {
                if let Some(family) = &details.family {
                    println!("    Family: {}", family.blue());
                }
                if let Some(params) = &details.parameter_size {
                    println!("    Parameters: {}", params.blue());
                }
            }
            println!("    Modified: {}", model.modified_at.dimmed());
            println!();
        }
    }

    Ok(())
}

async fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "System Status".cyan().bold());
    println!();

    // Check Ollama connection
    let ollama_status = if check_ollama_health().await? {
        "✅ Connected".green()
    } else {
        "❌ Disconnected".red()
    };
    println!("Ollama: {}", ollama_status);

    // Check available models
    match fetch_models().await {
        Ok(models) => {
            println!("Models: {} available", models.len().to_string().yellow());
        }
        Err(_) => {
            println!("Models: {}", "Unable to fetch".red());
        }
    }

    // Check tool availability
    let executor = ToolExecutor::new();
    match executor.check_package_managers().await {
        Ok(result) => {
            println!();
            println!("{}", "Package Managers:".cyan().bold());
            for line in result.output.lines() {
                if line.starts_with("✓") {
                    println!("  {}", line.green());
                } else if line.starts_with("✗") {
                    println!("  {}", line.dimmed());
                }
            }
        }
        Err(_) => {
            println!("Package Managers: {}", "Check failed".red());
        }
    }

    // Check system info
    match executor.system_info().await {
        Ok(result) => {
            println!();
            println!("{}", "System Information:".cyan().bold());
            for line in result.output.lines().take(3) {
                println!("  {}", line);
            }
        }
        Err(_) => {
            println!("System Info: {}", "Unable to fetch".red());
        }
    }

    Ok(())
}

async fn run_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Running System Diagnostics...".cyan().bold());
    println!();

    let _executor = ToolExecutor::new();

    // Test different subsystems
    let tests = vec![
        "Ollama Connection",
        "File System Access", 
        "Network Access",
        "Package Managers",
        "System Commands",
    ];

    for test_name in tests {
        print!("Testing {}: ", test_name);
        io::stdout().flush().unwrap();

        let result = match test_name {
            "Ollama Connection" => test_ollama_connection().await,
            "File System Access" => test_file_system().await,
            "Network Access" => test_network_access().await,
            "Package Managers" => test_package_managers().await,
            "System Commands" => test_system_commands().await,
            _ => Ok(()),
        };

        match result {
            Ok(_) => println!("{}", "✅ PASS".green()),
            Err(e) => println!("{} {}", "❌ FAIL".red(), e.to_string().dimmed()),
        }
    }

    println!();
    println!("{}", "Diagnostics complete!".cyan().bold());

    Ok(())
}

async fn test_ollama_connection() -> Result<(), Box<dyn std::error::Error>> {
    check_ollama_health().await?;
    Ok(())
}

async fn test_file_system() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    let test_file = "test_file_access.tmp";
    fs::write(test_file, "test")?;
    fs::remove_file(test_file)?;
    Ok(())
}

async fn test_network_access() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    client
        .get("https://httpbin.org/status/200")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;
    Ok(())
}

async fn test_package_managers() -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();
    executor.check_package_managers().await?;
    Ok(())
}

async fn test_system_commands() -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();
    executor.execute_command("echo test").await?;
    Ok(())
}

async fn run_tool_discovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 Discovering Available Tools...".cyan().bold());
    println!();
    
    let mut discovery = tools::discovery::ToolDiscovery::new();
    let results = discovery.discover_tools().await;
    
    discovery.display_discovery_results(&results);
    
    Ok(())
}

async fn handle_config_command(command: ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();

    match command {
        ConfigCommands::Show => {
            let result = executor.get_config(None).await?;
            println!("{}", result.output);
        }
        ConfigCommands::Set { key, value } => {
            let json_value: serde_json::Value =
                serde_json::from_str(&value).unwrap_or_else(|_| serde_json::Value::String(value));
            let result = executor.set_config(&key, json_value).await?;
            println!("{}", result.output);
        }
        ConfigCommands::Reset => {
            println!("{} Configuration reset to defaults", "✅".green());
        }
        ConfigCommands::Export { path } => {
            let result = executor
                .export_conversation(tools::ExportFormat::Json, &path)
                .await?;
            println!("{}", result.output);
        }
    }

    Ok(())
}

async fn handle_tool_command(command: ToolCommands) -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();

    match command {
        ToolCommands::Git { git_command } => {
            handle_git_command(git_command, &executor).await?;
        }
        ToolCommands::System { system_command } => {
            handle_system_command(system_command, &executor).await?;
        }
        ToolCommands::Docker { docker_command } => {
            handle_docker_command(docker_command, &executor).await?;
        }
        ToolCommands::Package { package_command } => {
            handle_package_command(package_command, &executor).await?;
        }
        ToolCommands::File { file_command } => {
            handle_file_command(file_command, &executor).await?;
        }
    }

    Ok(())
}

async fn handle_git_command(
    command: GitCommands,
    executor: &ToolExecutor,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = match command {
        GitCommands::Status => executor.git_status(None).await?,
        GitCommands::Add { files } => executor.git_add(&files, None).await?,
        GitCommands::Commit { message } => executor.git_commit(&message, None).await?,
        GitCommands::Push => executor.git_push(None, None, None).await?,
        GitCommands::Pull => executor.git_pull(None, None, None).await?,
        GitCommands::Log { count } => executor.git_log(Some(count), true, None).await?,
    };

    if result.success {
        println!("{}", result.output);
    } else {
        eprintln!("{} {}", "Error:".red(), result.error.unwrap_or_default());
    }

    Ok(())
}

async fn handle_system_command(
    command: SystemCommands,
    executor: &ToolExecutor,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = match command {
        SystemCommands::Info => executor.system_info().await?,
        SystemCommands::Memory => executor.memory_usage().await?,
        SystemCommands::Disk { path } => executor.disk_usage(path.as_deref()).await?,
        SystemCommands::Processes { filter } => executor.process_list(filter.as_deref()).await?,
        SystemCommands::Network => executor.network_info().await?,
    };

    if result.success {
        println!("{}", result.output);
    } else {
        eprintln!("{} {}", "Error:".red(), result.error.unwrap_or_default());
    }

    Ok(())
}

async fn handle_docker_command(
    command: DockerCommands,
    executor: &ToolExecutor,
) -> Result<(), Box<dyn std::error::Error>> {
    use tools::DockerResourceType;

    let result = match command {
        DockerCommands::List { resource } => {
            let resource_type = match resource {
                DockerResource::Containers => DockerResourceType::Containers,
                DockerResource::Images => DockerResourceType::Images,
                DockerResource::Volumes => DockerResourceType::Volumes,
                DockerResource::Networks => DockerResourceType::Networks,
            };
            executor.docker_list(resource_type).await?
        }
        DockerCommands::Run {
            image,
            ports,
            volumes,
        } => {
            let port_mappings = if ports.is_empty() { None } else { Some(ports) };
            let volume_mappings = if volumes.is_empty() {
                None
            } else {
                Some(volumes)
            };
            executor
                .docker_run(&image, None, port_mappings, volume_mappings, None)
                .await?
        }
        DockerCommands::Stop { container } => executor.docker_stop(&container).await?,
        DockerCommands::Logs { container, tail } => {
            executor.docker_logs(&container, false, tail).await?
        }
    };

    if result.success {
        println!("{}", result.output);
    } else {
        eprintln!("{} {}", "Error:".red(), result.error.unwrap_or_default());
    }

    Ok(())
}

async fn handle_package_command(
    command: PackageCommands,
    executor: &ToolExecutor,
) -> Result<(), Box<dyn std::error::Error>> {
    use tools::{CargoOperation, NpmOperation};

    let result = match command {
        PackageCommands::Cargo { cargo_command } => {
            let operation = match &cargo_command {
                CargoCommands::Build => CargoOperation::Build,
                CargoCommands::Run => CargoOperation::Run,
                CargoCommands::Test => CargoOperation::Test,
                CargoCommands::Add { package: _ } => CargoOperation::Add,
                CargoCommands::Remove { package: _ } => CargoOperation::Remove,
            };

            let package = match &cargo_command {
                CargoCommands::Add { package } | CargoCommands::Remove { package } => {
                    Some(package.as_str())
                }
                _ => None,
            };

            executor.cargo_operation(operation, package, None).await?
        }
        PackageCommands::Npm { npm_command } => {
            let (operation, package) = match npm_command {
                NpmCommands::Install { package } => (NpmOperation::Install, package),
                NpmCommands::Uninstall { package } => (NpmOperation::Uninstall, Some(package)),
                NpmCommands::Run { script } => (NpmOperation::Run { script }, None),
                NpmCommands::List => (NpmOperation::List, None),
            };

            executor
                .npm_operation(operation, package.as_deref(), false)
                .await?
        }
        PackageCommands::Check => executor.check_package_managers().await?,
        PackageCommands::Search { query } => executor.search_packages(&query).await?,
    };

    if result.success {
        println!("{}", result.output);
    } else {
        eprintln!("{} {}", "Error:".red(), result.error.unwrap_or_default());
    }

    Ok(())
}

async fn handle_file_command(
    command: FileCommands,
    executor: &ToolExecutor,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = match command {
        FileCommands::Read { path } => executor.file_read(&path)?,
        FileCommands::Write { path, content } => executor.file_write(&path, &content)?,
        FileCommands::Search { pattern, directory } => {
            executor.file_search(&pattern, directory.as_deref())?
        }
        FileCommands::List { path } => {
            let list_path = path.unwrap_or_else(|| ".".to_string());
            executor.list_directory(&list_path)?
        }
    };

    if result.success {
        println!("{}", result.output);
    } else {
        eprintln!("{} {}", "Error:".red(), result.error.unwrap_or_default());
    }

    Ok(())
}

async fn execute_single_command(
    command: &str,
    model_name: Option<String>,
    vim_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Executing: {}", "⚡".cyan(), command.yellow());

    // Get model
    let models = fetch_models().await?;
    let selected_model = if let Some(model_name) = model_name {
        models
            .iter()
            .find(|m| m.name == model_name)
            .map(|m| client::SelectedModel::from(m.clone()))
            .ok_or_else(|| format!("Model '{}' not found", model_name))?
    } else {
        client::SelectedModel::from(models[0].clone())
    };

    // Create session and execute command
    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::with_vim_mode(selected_model, tool_executor, vim_mode);

    session.process_single_command(command).await?;

    Ok(())
}

// New Claude CLI-like handlers
async fn start_chat_session_with_context(
    model_name: Option<String>,
    _config_path: Option<String>,
    vim_mode: bool,
    files: Vec<String>,
    project_context: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace_manager = WorkspaceManager::new();
    
    // Load existing workspace context if available
    if let Err(_) = workspace_manager.load_context() {
        // If no context exists and project_context is requested, create one
        if project_context {
            workspace_manager.init_workspace(None, None)?;
            workspace_manager.get_context_mut().unwrap().scan_project(false)?;
        }
    }

    // Add specified files to context
    if !files.is_empty() {
        if let Some(context) = workspace_manager.get_context_mut() {
            context.add_files(&files)?;
        } else {
            // Create a minimal context just for the files
            workspace_manager.init_workspace(None, None)?;
            workspace_manager.get_context_mut().unwrap().add_files(&files)?;
        }
        workspace_manager.save_context()?;
    }

    // Get selected model
    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    // Create session
    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::with_vim_mode(selected_model, tool_executor, vim_mode);

    // Add workspace context to session if available
    if let Some(context) = workspace_manager.get_context() {
        let file_contents = context.get_file_contents()?;
        session.add_workspace_context(context, file_contents)?;
    }

    // Start interactive session
    session.run().await?;

    Ok(())
}

async fn handle_ask_command(
    prompt: String,
    model_name: Option<String>,
    files: Vec<String>,
    project_context: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace_manager = WorkspaceManager::new();
    
    // Load workspace context if requested
    if project_context {
        if let Err(_) = workspace_manager.load_context() {
            workspace_manager.init_workspace(None, None)?;
            workspace_manager.get_context_mut().unwrap().scan_project(false)?;
        }
    }

    // Add specified files to context
    if !files.is_empty() {
        if workspace_manager.get_context().is_none() {
            workspace_manager.init_workspace(None, None)?;
        }
        workspace_manager.get_context_mut().unwrap().add_files(&files)?;
    }

    // Get selected model
    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    // Create session
    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::new(selected_model, tool_executor);

    // Add workspace context to session if available
    if let Some(context) = workspace_manager.get_context() {
        let file_contents = context.get_file_contents()?;
        session.add_workspace_context(context, file_contents)?;
    }

    // Process the prompt
    session.process_single_command(&prompt).await?;

    Ok(())
}

async fn handle_generate_command(
    description: String,
    language: Option<String>,
    output: Option<String>,
    model_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::new(selected_model, tool_executor);

    // Construct the generation prompt
    let mut prompt = format!("Generate code based on this description: {}", description);
    
    if let Some(lang) = language {
        prompt.push_str(&format!(" Use {} programming language.", lang));
    }
    
    if let Some(out) = output {
        prompt.push_str(&format!(" Save the code to file: {}", out));
    }

    session.process_single_command(&prompt).await?;

    Ok(())
}

async fn handle_edit_command(
    files: Vec<String>,
    instruction: Option<String>,
    model_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        eprintln!("{} No files specified for editing", "❌".red());
        return Ok(());
    }

    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::new(selected_model, tool_executor);

    // Load file contents
    let mut file_contents = std::collections::HashMap::new();
    for file in &files {
        if let Ok(content) = std::fs::read_to_string(file) {
            file_contents.insert(file.clone(), content);
        }
    }

    // Construct the editing prompt
    let mut prompt = String::new();
    if let Some(instr) = instruction {
        prompt.push_str(&format!("Edit the following files according to this instruction: {}\n\n", instr));
    } else {
        prompt.push_str("Edit the following files:\n\n");
    }

    for (file, content) in file_contents {
        prompt.push_str(&format!("File: {}\n```\n{}\n```\n\n", file, content));
    }

    session.process_single_command(&prompt).await?;

    Ok(())
}

async fn handle_review_command(
    files: Vec<String>,
    focus: Option<String>,
    model_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        eprintln!("{} No files specified for review", "❌".red());
        return Ok(());
    }

    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::new(selected_model, tool_executor);

    // Load file contents
    let mut file_contents = std::collections::HashMap::new();
    for file in &files {
        if let Ok(content) = std::fs::read_to_string(file) {
            file_contents.insert(file.clone(), content);
        }
    }

    // Construct the review prompt
    let mut prompt = String::new();
    if let Some(focus_area) = focus {
        prompt.push_str(&format!("Review the following files focusing on: {}\n\n", focus_area));
    } else {
        prompt.push_str("Review the following files for code quality, bugs, and improvements:\n\n");
    }

    for (file, content) in file_contents {
        prompt.push_str(&format!("File: {}\n```\n{}\n```\n\n", file, content));
    }

    session.process_single_command(&prompt).await?;

    Ok(())
}

async fn handle_commit_command(
    context: Option<String>,
    model_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::new(selected_model, tool_executor);

    // Get git diff
    let git_tool_executor = ToolExecutor::new();
    let diff_result = git_tool_executor.git_diff(None, false, None).await?;
    if !diff_result.success {
        eprintln!("{} Failed to get git diff", "❌".red());
        return Ok(());
    }

    // Construct commit message generation prompt
    let mut prompt = "Generate a concise and descriptive commit message based on the following git diff:\n\n".to_string();
    prompt.push_str(&diff_result.output);
    
    if let Some(ctx) = context {
        prompt.push_str(&format!("\n\nAdditional context: {}", ctx));
    }

    session.process_single_command(&prompt).await?;

    Ok(())
}

async fn handle_init_command(
    path: Option<String>,
    project_type: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace_manager = WorkspaceManager::new();
    
    workspace_manager.init_workspace(path, project_type)?;
    
    if let Some(context) = workspace_manager.get_context_mut() {
        context.scan_project(false)?;
        workspace_manager.save_context()?;
    }
    
    Ok(())
}

async fn handle_history_command(
    command: HistoryCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut history_manager = tools::history::HistoryManager::new();
    
    match command {
        HistoryCommands::Show { count, detailed } => {
            let entries = history_manager.get_recent(count);
            history_manager.show_entries(&entries, detailed);
        }
        HistoryCommands::Clear { all: _ } => {
            history_manager.clear();
            println!("{} Conversation history cleared", "🧹".cyan());
        }
        HistoryCommands::Export { path, format } => {
            history_manager.export(&path, &format)?;
            println!("{} Conversation history exported to: {} (format: {})", "📤".cyan(), path, format);
        }
        HistoryCommands::Search { query, limit } => {
            let entries = history_manager.search(&query, limit);
            println!("{} Search results for '{}':", "🔍".cyan(), query);
            history_manager.show_entries(&entries, true);
        }
    }
    
    Ok(())
}

async fn handle_workspace_command(
    command: WorkspaceCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace_manager = WorkspaceManager::new();
    
    match command {
        WorkspaceCommands::Init { path, project_type } => {
            workspace_manager.init_workspace(path, project_type)?;
            if let Some(context) = workspace_manager.get_context_mut() {
                context.scan_project(false)?;
            }
            workspace_manager.save_context()?;
        }
        WorkspaceCommands::Info => {
            workspace_manager.load_context()?;
            if let Some(context) = workspace_manager.get_context() {
                println!("{} Workspace Information", "📁".cyan());
                println!("Root path: {}", context.root_path.display());
                println!("Project type: {}", context.project_type.as_deref().unwrap_or("unknown"));
                println!("Files in context: {}", context.included_files.len());
                println!("Created: {}", context.created_at);
                println!("Last updated: {}", context.last_updated);
            } else {
                println!("{} No workspace context found", "❌".red());
            }
        }
        WorkspaceCommands::Scan { path, include_hidden } => {
            if workspace_manager.get_context().is_none() {
                workspace_manager.init_workspace(path, None)?;
            }
            if let Some(context) = workspace_manager.get_context_mut() {
                context.scan_project(include_hidden)?;
                workspace_manager.save_context()?;
            }
        }
        WorkspaceCommands::Add { files } => {
            workspace_manager.load_context()?;
            if let Some(context) = workspace_manager.get_context_mut() {
                context.add_files(&files)?;
                workspace_manager.save_context()?;
                println!("{} Added {} files to workspace context", "✅".green(), files.len());
            } else {
                println!("{} No workspace context found. Run 'init' first.", "❌".red());
            }
        }
        WorkspaceCommands::Remove { files } => {
            workspace_manager.load_context()?;
            if let Some(context) = workspace_manager.get_context_mut() {
                context.remove_files(&files)?;
                workspace_manager.save_context()?;
                println!("{} Removed {} files from workspace context", "✅".green(), files.len());
            } else {
                println!("{} No workspace context found", "❌".red());
            }
        }
        WorkspaceCommands::List { detailed } => {
            workspace_manager.load_context()?;
            if let Some(context) = workspace_manager.get_context() {
                println!("{} Workspace files ({})", "📁".cyan(), context.included_files.len());
                for file in &context.included_files {
                    if detailed {
                        let full_path = context.root_path.join(file);
                        if let Ok(metadata) = std::fs::metadata(&full_path) {
                            println!("  {} ({} bytes)", file.display(), metadata.len());
                        } else {
                            println!("  {} (not found)", file.display());
                        }
                    } else {
                        println!("  {}", file.display());
                    }
                }
            } else {
                println!("{} No workspace context found", "❌".red());
            }
        }
        WorkspaceCommands::Clear => {
            workspace_manager.clear_context()?;
            println!("{} Workspace context cleared", "🧹".cyan());
        }
    }
    
    Ok(())
}

async fn execute_single_command_with_context(
    command: &str,
    model_name: Option<String>,
    vim_mode: bool,
    files: Vec<String>,
    project_context: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace_manager = WorkspaceManager::new();
    
    // Load workspace context if requested
    if project_context {
        if let Err(_) = workspace_manager.load_context() {
            workspace_manager.init_workspace(None, None)?;
            workspace_manager.get_context_mut().unwrap().scan_project(false)?;
        }
    }

    // Add specified files to context
    if !files.is_empty() {
        if workspace_manager.get_context().is_none() {
            workspace_manager.init_workspace(None, None)?;
        }
        workspace_manager.get_context_mut().unwrap().add_files(&files)?;
    }

    // Get selected model
    let selected_model = if let Some(model) = model_name {
        // Try to find the specified model
        let available_models = fetch_models().await?;
        let matching_models: Vec<_> = available_models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&model.to_lowercase()))
            .collect();
        
        if matching_models.is_empty() {
            return Err(format!("Model '{}' not found", model).into());
        } else if matching_models.len() == 1 {
            SelectedModel::from(matching_models[0].clone())
        } else {
            return Err(format!("Multiple models match '{}', please be more specific", model).into());
        }
    } else {
        let available_models = fetch_models().await?;
        select_model(&available_models)?
    };

    // Create session
    let tool_executor = ToolExecutor::new();
    let mut session = AssistantSession::with_vim_mode(selected_model, tool_executor, vim_mode);

    // Add workspace context to session if available
    if let Some(context) = workspace_manager.get_context() {
        let file_contents = context.get_file_contents()?;
        session.add_workspace_context(context, file_contents)?;
    }

    session.process_single_command(command).await?;

    Ok(())
}

use std::io::{self, Write};
