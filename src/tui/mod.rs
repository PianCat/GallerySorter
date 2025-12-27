//! Ratatui Terminal UI模块
//!
//! 提供基于ratatui的现代终端用户界面。

pub mod app;
pub mod display;
pub mod event;
pub mod state;
pub mod theme;
pub mod ui;

// 重新导出主要类型
pub use app::TuiApp;
pub use display::{display_summary, should_run_interactive};
pub use event::{EventPoll, TuiEvent};
pub use state::{
    ConfigStep, ConfigWizardState, InputState, MenuItem, MenuState, ProgressState, Screen,
    SelectionState, SummaryState,
};
pub use theme::{Theme, theme};
pub use ui::{AppState, TuiResult, render, run_processing};
