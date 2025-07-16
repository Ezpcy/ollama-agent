use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod client;
mod input;
mod session;
mod tools;

use client::{
    check_ollama_health, delete_model, fetch_models, list_models_filtered, pull_model,
    select_model, show_model_info,
};
use session::AssistantSession;
use tools::{ToolConfig, ToolExecutor};

#[derive(Parser)]
#[command(name = "ollama-agent")]
#[command(about = "Advanced AI Assistant with System Tools")]
#[command(version = "0.2.0")]
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
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive chat session
    Chat {
        /// Model to use for the session
        #[arg(short, long)]
        model: Option<String>,

        /// Enable vim mode for input
        #[arg(long)]
        vim: bool,
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
        Some(Commands::Chat { model, vim }) => {
            start_chat_session(model, cli.config, vim).await?;
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
        Some(Commands::Config { config_command }) => {
            handle_config_command(config_command).await?;
        }
        Some(Commands::Tool { tool_command }) => {
            handle_tool_command(tool_command).await?;
        }
        None => {
            // No subcommand provided
            if let Some(command) = cli.execute {
                // Execute single command
                execute_single_command(&command, cli.model, cli.vim).await?;
            } else {
                // Default to interactive chat
                start_chat_session(cli.model, cli.config, cli.vim).await?;
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

    let executor = ToolExecutor::new();

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

use std::io::{self, Write};
