pub mod core;
pub mod executor;
pub mod parser;
pub mod permissions;

// New modules
pub mod api;
pub mod async_executor;
pub mod config;
pub mod database;
pub mod discovery;
pub mod docker;
pub mod enhanced_errors;
pub mod errors;
pub mod git;
pub mod history;
pub mod logging;
pub mod model_config;
pub mod package;
pub mod search;
pub mod system;
pub mod text;

// Re-export everything from core for convenience
pub use core::*;

// Re-export commonly used items
pub use async_executor::*;
pub use config::ConversationEntry;
pub use parser::*;
pub use permissions::*;
