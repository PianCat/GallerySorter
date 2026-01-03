//! TUI 状态模块

pub mod app;
pub mod input;
pub mod menu;
pub mod progress;
pub mod selection;
pub mod summary;
pub mod wizard;

pub use app::{AppState, TuiResult, reset_to_main_menu};
pub use input::InputState;
pub use menu::{MenuItem, MenuState, Screen};
pub use progress::ProgressState;
pub use selection::{Selectable, SelectionState};
pub use summary::SummaryState;
pub use wizard::{
    BoolSelection, ConfigFormState, ConfigStep, ConfigWizardState, EnumSelection, FormField,
};
