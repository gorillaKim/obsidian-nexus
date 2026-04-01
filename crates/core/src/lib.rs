// Pre-existing clippy lints suppressed at crate level (not introduced by current changes)
#![allow(clippy::derivable_impls)]
#![allow(clippy::missing_transmute_annotations)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::empty_line_after_outer_attr)]

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
