//! Ratatui Terminal UI module
//!
//! Provides modern terminal user interface based on ratatui.

pub mod app;
pub mod display;
pub mod event;
pub mod state;
pub mod theme;
pub mod ui;

// Re-export main types
pub use app::TuiApp;
pub use display::{display_summary, should_run_interactive};
pub use event::{EventPoll, TuiEvent};
pub use state::{
    ConfigStep, ConfigWizardState, InputState, MenuItem, MenuState, ProgressState, Screen,
    SelectionState, SummaryState,
};
pub use theme::{Theme, theme};
pub use ui::{AppState, TuiResult, render, run_processing};
