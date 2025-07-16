# 🤖 Advanced Ollama AI Assistant

A powerful, extensible AI assistant that integrates with Ollama to provide intelligent system automation, development tools, and project management across multiple programming languages.

## ✨ Features

### 🧠 Core AI Capabilities
- **LLM-Powered Tool Selection**: Natural language requests automatically mapped to appropriate tools
- **Advanced Model Configuration**: Real-time parameter adjustment (temperature, tokens, etc.)
- **Multiple Model Support**: Code models, chat models, and custom configurations
- **Dynamic Model Switching**: Switch between models during conversation with live updates
- **Conversation Management**: Export/import, history tracking, session statistics
- **Vim Mode Input**: Full vim-style text editing with normal, insert, and command modes
- **Response Control**: Stop response generation anytime with Ctrl+C without exiting

### 🛠️ System Tools
- **Git Operations**: Status, commit, push, pull, branching, logs, diffs
- **File Management**: Read, write, edit, search, watch for changes
- **System Monitoring**: CPU, memory, disk usage, process management
- **Package Management**: Cargo, NPM, Pip, Maven, Gradle, Go modules with dependency management
- **Docker Integration**: Container management, logs, image operations

### 🌐 Web & API Tools
- **Web Scraping**: Extract content from websites
- **HTTP Requests**: GET, POST, PUT, DELETE with authentication
- **REST API Calls**: Structured API interactions
- **GraphQL Support**: Query GraphQL endpoints

### 📊 Data Processing
- **Database Operations**: SQLite, PostgreSQL, MySQL, MongoDB support
- **Text Processing**: JSON formatting, CSV parsing, regex matching
- **Data Analysis**: Statistics, transformations, pattern matching

### ⚙️ Advanced Features
- **Task Scheduling**: Cron-like task automation
- **Configuration Management**: Persistent settings and preferences
- **Permission System**: Safe execution with user approval
- **File Watching**: Monitor files for changes
- **Multi-format Export**: Markdown, HTML, JSON conversation exports

## 🚀 Quick Start

### Prerequisites
- Rust (latest stable) - for building the assistant
- Ollama running locally
- At least one Ollama model installed

### Installation

```bash
# Clone the repository
git clone https://github.com/Ezpcy/ollama-cli-assistant
cd ollama-cli-assistant

# Build the project
cargo build --release

# Run the assistant
cargo run
```

### First Run

```bash
# Start interactive session
cargo run

# Or use a specific model
cargo run -- chat --model codellama

# Enable vim mode for enhanced text editing
cargo run -- --vim

# Execute a single command
cargo run -- --execute "show system info"
```

## 📚 Usage Examples

### Interactive Mode

```bash
# Start the assistant
cargo run

# Example interactions:
🤖 How can I help you?
> set temperature to 0.8
> switch to codellama          # Switch model during conversation
> git status
> analyze my python project
> create a new java application
> search for programming tutorials
> show memory usage
> list docker containers
> format this json: {"name":"test"}

# During AI responses, press Ctrl+C to stop generation
🤖 Generating response...
Press Ctrl+C to stop response generation...
[AI response text...]
^C
Response generation stopped by user
```

### Command Line Interface

```bash
# Git operations
cargo run -- tool git status
cargo run -- tool git commit "fix: update dependencies"
cargo run -- tool git push

# System information
cargo run -- tool system info
cargo run -- tool system memory
cargo run -- tool system disk /home

# Docker operations
cargo run -- tool docker list containers
cargo run -- tool docker run nginx --ports 80:80

# Package management
cargo run -- tool package cargo build
cargo run -- tool package npm install express
cargo run -- tool package mvn clean install
cargo run -- tool package go mod tidy

# File operations
cargo run -- tool file read package.json
cargo run -- tool file search "*.py" src/
cargo run -- tool file read pom.xml
```

### Model Management

```bash
# List available models
cargo run -- list

# Pull a new model
cargo run -- pull codellama:7b

# Show model information
cargo run -- show llama2

# Delete a model
cargo run -- delete old-model:latest
```

### Configuration

```bash
# View current configuration
cargo run -- config show

# Set configuration values
cargo run -- config set auto_approve_safe true
cargo run -- config set default_timeout 60
cargo run -- config set theme dark

# Export configuration
cargo run -- config export my-config.json
```

## 🔧 Advanced Configuration

### Model Parameters

The assistant supports real-time model parameter adjustment:

```bash
# In interactive mode:
> set temperature to 0.7
> set max tokens to 4096
> set top p to 0.9
> show model config              # Shows current model and parameters
> switch to codellama            # Dynamic model switching (live updates)
> use model llama2               # Alternative syntax
> change model to qwen           # Another way to switch

# Features:
# - Live model switching during conversation
# - Global configuration sync
# - Automatic model validation
# - Visual feedback and confirmation
```

### Vim Mode

Enable vim-style text editing for enhanced input control:

```bash
# Start with vim mode enabled
cargo run -- --vim

# Or enable/disable during runtime
cargo run -- chat --vim

# Vim Mode Commands:
# Normal Mode:
#   i - Enter insert mode
#   I - Insert at beginning of line
#   a - Insert after cursor
#   A - Insert at end of line
#   o - Open new line below
#   h,j,k,l - Move cursor
#   0,$ - Move to beginning/end of line
#   w,b - Move by word
#   x,X - Delete character
#   dd - Delete line
#   u - Undo
#   :q - Quit
#   :help - Show help
#   ESC - Return to normal mode
#
# Visual Features:
#   - Highlighted cursor in normal mode (blue background, terminal cursor hidden)
#   - Proper terminal cursor positioning in insert mode
#   - Mode indicator shows current mode ([NORMAL], [INSERT], [COMMAND])
#   - Cursor position always visible and accurately aligned
#   - Clean cursor behavior: visual highlight in normal mode, terminal cursor in insert mode
#   - Unicode and emoji support in prompts
```

### Response Control

Control AI response generation with improved interrupt handling:

```bash
# During any AI response generation:
Press Ctrl+C to stop response generation...
[AI response text...]
^C
Response generation stopped by user

# The application continues running - no exit
🤖 How can I help you?
> continue with next question

# Features:
# - Responsive Ctrl+C handling (checks every 50ms)
# - Immediate interruption during token generation
# - Preserves partial responses
# - Session continues after interruption
```

### Tool Configuration

Configure tool behavior through the configuration system:

```json
# ~/.ollama_agent/config.json
{
  "auto_approve_safe": true,
  "max_file_size": 10485760,
  "default_timeout": 30,
  "git_default_remote": "origin",
  "theme": "default",
  "editor": "nano",
  "log_level": "info",
  "backup_enabled": true
}
```

### Database Connections

Configure database connections for SQL operations:

```bash
# In interactive mode:
> query sqlite database.db "SELECT * FROM users"
> backup database postgres://user:pass@localhost/mydb to backup.sql
```

## 🎯 Use Cases

### Development Workflow

```bash
# Complete development workflow
> git status
> run tests                   # Works with any project type
> git add .
> git commit "feat: add new feature"
> git push origin main
> create documentation for the changes
```

### System Administration

```bash
# System monitoring and maintenance
> system info
> memory usage
> disk usage /var/log
> list processes python
> docker list containers
> check package managers
```

### Data Analysis

```bash
# Process and analyze data
> read data.csv
> parse csv with delimiter ;
> find pattern "\d+\.\d+" in text
> format json data
> calculate text statistics for document
```

### API Development

```bash
# API testing and development
> GET request to https://api.github.com/users/octocat
> scrape https://example.com
> test GraphQL endpoint with query
> create REST API documentation
```

## 🔌 Tool Integration

### Git Integration

Full Git workflow support with intelligent command recognition:

- **Status and Inspection**: `git status`, `git log`, `git diff`
- **Branching**: `create branch feature-x`, `switch to main`, `merge feature`
- **Remote Operations**: `push to origin`, `pull latest changes`
- **Advanced**: Repository-specific operations, multi-repo support

### Docker Integration

Complete Docker ecosystem management:

- **Container Management**: List, run, stop, inspect containers
- **Image Operations**: Build, pull, tag, remove images
- **Logs and Monitoring**: Real-time logs, resource usage
- **Compose Support**: Multi-container applications

### Package Managers

Universal package management across ecosystems:

- **Rust (Cargo)**: Build, test, add/remove dependencies
- **Node.js (NPM)**: Install, run scripts, audit packages
- **Python (Pip)**: Install, list, search packages
- **Java (Maven/Gradle)**: Build, test, dependency management
- **Go (Go modules)**: Module management, dependency resolution
- **Cross-Platform**: Search packages across all managers

## 🛡️ Security Features

### Permission System

- **Safe Operations**: Automatically approved (file reads, listings)
- **Moderate Risk**: User confirmation required (file writes, git operations)
- **High Risk**: Explicit approval with warnings (system commands, deletions)

### Session Management

- **Isolated Sessions**: Each session is independent
- **Audit Trail**: Complete history of commands and tools used
- **Configuration Backup**: Automatic backup of important settings

## 📊 Performance & Monitoring

### Session Statistics

Track your assistant usage with detailed statistics:

```bash
# View session stats
> stats

# Example output:
Session Statistics:
  Commands processed: 15
  Tools executed: 8
  Success rate: 93.3%
  Average response time: 1.2s
```

### System Diagnostics

Built-in system health checking:

```bash
cargo run -- diagnostics

# Tests:
# ✅ Ollama Connection
# ✅ File System Access
# ✅ Network Access
# ✅ Package Managers
# ✅ System Commands
```

## 🔮 Advanced Features

### File Watching

Monitor files for changes and trigger actions:

```bash
> watch config.json for changes for 60 seconds
> set up file monitoring for src/ directory
```

### Task Scheduling

Schedule recurring tasks with cron-like syntax:

```bash
> schedule task "cargo test" daily at 9am
> list scheduled tasks
> cancel task daily-tests
```

### Multi-Format Export

Export conversations in various formats:

```bash
> export conversation to markdown report.md
> export session to html documentation.html
> export data to json backup.json
```

### Custom Commands

Define custom command shortcuts:

```bash
# In configuration:
{
  "custom_commands": {
    "daily": "git status && cargo test && git log --oneline -5",
    "deploy": "cargo build --release && docker build -t myapp .",
    "backup": "git add . && git commit -m 'backup' && git push"
  }
}
```

## 🚨 Troubleshooting

### Common Issues

1. **Ollama Not Running**
   ```bash
   # Start Ollama service
   ollama serve
   
   # Check status
   cargo run -- status
   ```

2. **Model Not Found**
   ```bash
   # List available models
   cargo run -- list
   
   # Pull required model
   cargo run -- pull llama2
   ```

3. **Permission Denied**
   ```bash
   # Check file permissions
   ls -la file.txt
   
   # Update tool configuration
   cargo run -- config set auto_approve_safe false
   ```

4. **Tool Not Available**
   ```bash
   # Check tool availability
   cargo run -- status
   
   # Install missing tools (example)
   # For git: Install git
   # For docker: Install Docker
   # For package managers: Install npm, cargo, pip
   ```

## 🙏 Acknowledgments

- [Ollama](https://ollama.ai) for the excellent LLM runtime
- The Rust community for amazing crates and tools
- Contributors and users providing feedback and improvements

---

> Note that this application gives a LLM access to system tools and should be used with caution, I'm not responsible for any damage that might be caused.


