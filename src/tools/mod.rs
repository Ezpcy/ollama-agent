pub mod core;
pub mod executor;
pub mod parser;
pub mod permissions;

// New modules
pub mod api;
pub mod config;
pub mod database;
pub mod docker;
pub mod git;
pub mod model_config;
pub mod package;
pub mod system;
pub mod text;

// Re-export everything from core for convenience
pub use core::*;

// Re-export commonly used items
pub use config::{AppConfig, ConversationEntry};
pub use executor::*;
pub use model_config::{create_enhanced_request, get_current_model_config, ModelConfig};
pub use parser::*;
pub use permissions::*;
