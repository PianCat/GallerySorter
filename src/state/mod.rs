//! Incremental processing state management
//!
//! Provides two mechanisms for incremental processing:
//! 1. Full state tracking (JSON) - tracks every processed file
//! 2. Water level (TOML) - tracks only the newest file for fast comparison
//!
//! The water level approach is more efficient for append-only workflows like
//! photo imports, as it only needs to compare timestamps rather than
//! computing metadata hashes for all files.

mod full;
mod waterlevel;

pub use full::{ProcessedFile, ProcessingState};
pub use waterlevel::IncrementalWaterLevel;
