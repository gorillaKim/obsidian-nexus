pub mod config;
pub mod db;
pub mod embedding;
pub mod llm;
pub mod error;
pub mod index_engine;
pub mod indexer;
pub mod onboard;
pub mod project;
pub mod search;
pub mod status;
pub mod watcher;

#[cfg(test)]
pub mod test_helpers;

pub use config::Config;
pub use error::{NexusError, Result};
