//! 应用状态

use crate::process::ProcessingStats;
use crate::tui::state::{ConfigWizardState, MenuState, ProgressState, Screen, SummaryState};
use crate::tui::theme::config::MENU_ITEM_COUNT;
use ratatui::widgets::ListState;
use std::sync::Arc;

/// TUI 运行结果
#[derive(Debug)]
pub struct TuiResult {
    /// 配置
    pub config: crate::config::Config,
    /// 配置名称
    pub config_name: Option<String>,
    /// 是否在 TUI 内执行处理
    pub run_processing: bool,
}

/// 应用状态（包含 UI 状态）
#[derive(Debug)]
pub struct AppState {
    /// 当前屏幕
    pub current_screen: Screen,
    /// 菜单状态
    pub menu_state: MenuState,
    /// 配置向导状态
    pub config_wizard: ConfigWizardState,
    /// 进度状态
    pub progress_state: ProgressState,
    /// 摘要状态
    pub summary_state: SummaryState,
    /// 是否退出
    pub should_exit: bool,
    /// TUI 运行结果
    pub result: Option<TuiResult>,
    /// 配置选择列表状态
    pub select_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screen: Screen::MainMenu,
            menu_state: MenuState::with_count(MENU_ITEM_COUNT),
            config_wizard: ConfigWizardState::new(),
            progress_state: ProgressState::new(Arc::new(ProcessingStats::new()), 0),
            summary_state: SummaryState::new(ProcessingStats::new(), Vec::new(), false, None),
            should_exit: false,
            result: None,
            select_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
        }
    }
}

/// 重置到主菜单
pub fn reset_to_main_menu(state: &mut AppState) {
    state.current_screen = Screen::MainMenu;
    state.menu_state = MenuState::with_count(MENU_ITEM_COUNT);
    state.progress_state = ProgressState::new(Arc::new(ProcessingStats::new()), 0);
    state.summary_state = SummaryState::new(ProcessingStats::new(), Vec::new(), false, None);
    state.config_wizard = ConfigWizardState::new();
    state.should_exit = false;
    state.result = None;
    state.select_state.select(Some(0));
}
