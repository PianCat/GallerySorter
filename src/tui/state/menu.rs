//! 菜单相关状态

use crate::tui::state::selection::Selectable;
use ratatui::widgets::ListState;

/// 屏幕枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// 主菜单
    #[default]
    MainMenu,
    /// 配置向导
    ConfigWizard,
    /// 处理进度
    Progress,
    /// 结果摘要
    Summary,
    /// 退出确认
    Exit,
}

/// 菜单项
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    /// 直接运行
    RunDirect,
    /// 使用配置运行
    RunConfig,
    /// 创建配置
    CreateConfig,
    /// 退出
    Exit,
}

const MENU_ITEMS: [MenuItem; 4] = [
    MenuItem::RunDirect,
    MenuItem::RunConfig,
    MenuItem::CreateConfig,
    MenuItem::Exit,
];

impl MenuItem {
    /// 获取显示文本
    pub fn label(&self) -> String {
        match self {
            MenuItem::RunDirect => rust_i18n::t!("menu_option_run_direct").to_string(),
            MenuItem::RunConfig => rust_i18n::t!("menu_option_run_config").to_string(),
            MenuItem::CreateConfig => rust_i18n::t!("menu_option_create_config").to_string(),
            MenuItem::Exit => rust_i18n::t!("menu_option_exit").to_string(),
        }
    }

    /// 迭代所有菜单项
    pub fn iter() -> std::array::IntoIter<MenuItem, 4> {
        MENU_ITEMS.into_iter()
    }
}

/// 菜单状态
#[derive(Debug, Default)]
pub struct MenuState {
    /// List 组件状态
    pub list_state: ListState,
    /// 菜单项数量
    pub count: usize,
}

impl MenuState {
    /// 创建菜单状态
    pub fn with_count(count: usize) -> Self {
        Self {
            list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            count,
        }
    }

    /// 获取当前选中索引
    pub fn selected(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }
}

impl Selectable for MenuState {
    fn count(&self) -> usize {
        self.count
    }

    fn list_state(&self) -> &ListState {
        &self.list_state
    }

    fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list_state
    }
}
