use super::core::{DatabaseType, ToolExecutor, ToolResult};
use colored::Colorize;
use std::process::Command;

impl ToolExecutor {
    pub async fn sql_query(
        &self,
        connection_string: &str,
        query: &str,
        database_type: DatabaseType,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Executing SQL query on {:?} database",
            "🗃️".cyan(),
            database_type
        );

        match database_type {
            DatabaseType::PostgreSQL => self.execute_postgres_query(connection_string, query).await,
            DatabaseType::MySQL => self.execute_mysql_query(connection_string, query).await,
            DatabaseType::SQLite => {
                // Extract file path from connection string
                let db_path = connection_string
                    .strip_prefix("sqlite://")
                    .unwrap_or(connection_string);
                self.sqlite_query(db_path, query).await
            }
            DatabaseType::MongoDB => self.execute_mongo_query(connection_string, query).await,
        }
    }

    pub async fn sqlite_query(
        &self,
        database_path: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!(
            "{} Executing SQLite query on: {}",
            "📁".cyan(),
            database_path.yellow()
        );
        println!("  {} Query: {}", "📝".blue(), query.dimmed());

        // Check if database file exists
        if !std::path::Path::new(database_path).exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Database file not found: {}", database_path)),
                metadata: None,
            });
        }

        let output = Command::new("sqlite3")
            .args([database_path, "-header", "-column", query])
            .output()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult {
                success: true,
                output: result.to_string(),
                error: None,
                metadata: Some(serde_json::json!({
                    "database_path": database_path,
                    "database_type": "SQLite",
                    "query": query
                })),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if error.contains("command not found") || error.contains("not recognized") {
                        "SQLite3 command not found. Please install SQLite3.".to_string()
                    } else {
                        error.to_string()
                    },
                ),
                metadata: None,
            })
        }
    }

    async fn execute_postgres_query(
        &self,
        connection_string: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("  {} Using PostgreSQL connection", "🐘".blue());

        let output = Command::new("psql")
            .args([
                connection_string,
                "-c",
                query,
                "--csv", // Output as CSV for better parsing
            ])
            .output()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult {
                success: true,
                output: result.to_string(),
                error: None,
                metadata: Some(serde_json::json!({
                    "database_type": "PostgreSQL",
                    "query": query
                })),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if error.contains("command not found") || error.contains("not recognized") {
                        "psql command not found. Please install PostgreSQL client.".to_string()
                    } else {
                        error.to_string()
                    },
                ),
                metadata: None,
            })
        }
    }

    async fn execute_mysql_query(
        &self,
        connection_string: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("  {} Using MySQL connection", "🐬".blue());

        // Parse connection string to extract components
        // Format: mysql://user:password@host:port/database
        let url = url::Url::parse(connection_string)?;

        let mut cmd = Command::new("mysql");

        if let Some(host) = url.host_str() {
            cmd.args(["-h", host]);
        }

        if let Some(port) = url.port() {
            cmd.args(["-P", &port.to_string()]);
        }

        if !url.username().is_empty() {
            cmd.args(["-u", url.username()]);
        }

        if let Some(password) = url.password() {
            cmd.args(["-p", password]);
        }

        let database = url.path().trim_start_matches('/');
        if !database.is_empty() {
            cmd.arg(database);
        }

        cmd.args(["-e", query]);

        let output = cmd.output()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult {
                success: true,
                output: result.to_string(),
                error: None,
                metadata: Some(serde_json::json!({
                    "database_type": "MySQL",
                    "query": query
                })),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if error.contains("command not found") || error.contains("not recognized") {
                        "mysql command not found. Please install MySQL client.".to_string()
                    } else {
                        error.to_string()
                    },
                ),
                metadata: None,
            })
        }
    }

    async fn execute_mongo_query(
        &self,
        connection_string: &str,
        query: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("  {} Using MongoDB connection", "🍃".blue());

        // MongoDB queries are typically JavaScript
        let output = Command::new("mongosh")
            .args([connection_string, "--eval", query])
            .output()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult {
                success: true,
                output: result.to_string(),
                error: None,
                metadata: Some(serde_json::json!({
                    "database_type": "MongoDB",
                    "query": query
                })),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if error.contains("command not found") || error.contains("not recognized") {
                        "mongosh command not found. Please install MongoDB shell.".to_string()
                    } else {
                        error.to_string()
                    },
                ),
                metadata: None,
            })
        }
    }

    // Database utility functions
    pub async fn list_databases(
        &self,
        database_type: DatabaseType,
        connection_string: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing databases for {:?}", "📋".cyan(), database_type);

        let query = match database_type {
            DatabaseType::PostgreSQL => "\\l",
            DatabaseType::MySQL => "SHOW DATABASES;",
            DatabaseType::SQLite => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "SQLite uses file-based databases. Use 'list directory' to see .db files."
                            .to_string(),
                    ),
                    metadata: None,
                });
            }
            DatabaseType::MongoDB => "show dbs",
        };

        self.sql_query(connection_string, query, database_type)
            .await
    }

    pub async fn list_tables(
        &self,
        database_type: DatabaseType,
        connection_string: &str,
        _database_name: Option<&str>,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Listing tables for {:?}", "📋".cyan(), database_type);

        let query = match database_type {
            DatabaseType::PostgreSQL => "\\dt",
            DatabaseType::MySQL => "SHOW TABLES;",
            DatabaseType::SQLite => "SELECT name FROM sqlite_master WHERE type='table';",
            DatabaseType::MongoDB => "show collections",
        };

        self.sql_query(connection_string, query, database_type)
            .await
    }

    pub async fn describe_table(
        &self,
        database_type: DatabaseType,
        connection_string: &str,
        table_name: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Describing table: {}", "📋".cyan(), table_name.yellow());

        let query = match database_type {
            DatabaseType::PostgreSQL => format!("\\d {}", table_name),
            DatabaseType::MySQL => format!("DESCRIBE {};", table_name),
            DatabaseType::SQLite => format!("PRAGMA table_info({});", table_name),
            DatabaseType::MongoDB => format!("db.{}.findOne()", table_name),
        };

        self.sql_query(connection_string, &query, database_type)
            .await
    }

    // Backup and restore operations
    pub async fn backup_database(
        &self,
        database_type: DatabaseType,
        connection_string: &str,
        output_file: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error>> {
        println!("{} Creating backup: {}", "💾".cyan(), output_file.yellow());

        match database_type {
            DatabaseType::PostgreSQL => {
                let output = Command::new("pg_dump")
                    .args([connection_string, "-f", output_file])
                    .output()?;

                if output.status.success() {
                    Ok(ToolResult {
                        success: true,
                        output: format!("PostgreSQL backup created: {}", output_file),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "backup_file": output_file,
                            "database_type": "PostgreSQL"
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                        metadata: None,
                    })
                }
            }
            DatabaseType::MySQL => {
                let url = url::Url::parse(connection_string)?;
                let database = url.path().trim_start_matches('/');

                let mut cmd = Command::new("mysqldump");

                if let Some(host) = url.host_str() {
                    cmd.args(["-h", host]);
                }

                if !url.username().is_empty() {
                    cmd.args(["-u", url.username()]);
                }

                if let Some(password) = url.password() {
                    cmd.args(["-p", password]);
                }

                cmd.args([database, "--result-file", output_file]);

                let output = cmd.output()?;

                if output.status.success() {
                    Ok(ToolResult {
                        success: true,
                        output: format!("MySQL backup created: {}", output_file),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "backup_file": output_file,
                            "database_type": "MySQL"
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                        metadata: None,
                    })
                }
            }
            DatabaseType::SQLite => {
                let db_path = connection_string
                    .strip_prefix("sqlite://")
                    .unwrap_or(connection_string);

                let output = Command::new("sqlite3")
                    .args([db_path, format!(".backup {}", output_file).as_str()])
                    .output()?;

                if output.status.success() {
                    Ok(ToolResult {
                        success: true,
                        output: format!("SQLite backup created: {}", output_file),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "backup_file": output_file,
                            "database_type": "SQLite"
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                        metadata: None,
                    })
                }
            }
            DatabaseType::MongoDB => {
                let output = Command::new("mongodump")
                    .args(["--uri", connection_string, "--out", output_file])
                    .output()?;

                if output.status.success() {
                    Ok(ToolResult {
                        success: true,
                        output: format!("MongoDB backup created: {}", output_file),
                        error: None,
                        metadata: Some(serde_json::json!({
                            "backup_file": output_file,
                            "database_type": "MongoDB"
                        })),
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                        metadata: None,
                    })
                }
            }
        }
    }
}
