//! 主题模块
//!
//! 提供统一的主题定义，使用Stylize trait实现简洁的样式设置。

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

/// 主题颜色配置
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// 背景色（深色主题）
    pub bg: Color,
    /// 前景色（白色）
    pub fg: Color,
    /// 强调色（青色）
    pub accent: Color,
    /// 选中项背景色
    pub selected_bg: Color,
    /// 选中项前景色
    pub selected_fg: Color,
    /// 成功色（绿色）
    pub success: Color,
    /// 警告色（黄色）
    pub warning: Color,
    /// 错误色（红色）
    pub error: Color,
    /// 提示/次要文字色（灰色）
    pub hint: Color,
    /// 边框色
    pub border: Color,
    /// 进度条颜色
    pub progress: Color,
    /// 标题颜色
    pub title: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Black,
            fg: Color::White,
            accent: Color::Cyan,
            selected_bg: Color::Cyan,
            selected_fg: Color::Black,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            hint: Color::Gray,
            border: Color::Cyan,
            progress: Color::Cyan,
            title: Color::Cyan,
        }
    }
}

impl Theme {
    /// 创建新主题
    pub fn new() -> Self {
        Self::default()
    }

    /// 普通文本样式
    pub fn normal(&self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }

    /// 标题样式 - 使用Stylize trait
    pub fn title(&self) -> Style {
        Style::new()
            .fg(self.title)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    /// 选中项样式
    pub fn selected(&self) -> Style {
        Style::new()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// 边框样式
    pub fn border(&self) -> Style {
        Style::new().fg(self.border).bg(self.bg)
    }

    /// 提示文本样式
    pub fn hint(&self) -> Style {
        Style::new().fg(self.hint).bg(self.bg)
    }

    /// 成功样式
    pub fn success(&self) -> Style {
        Style::new().fg(self.success).bg(self.bg)
    }

    /// 警告样式
    pub fn warning(&self) -> Style {
        Style::new().fg(self.warning).bg(self.bg)
    }

    /// 错误样式
    pub fn error(&self) -> Style {
        Style::new().fg(self.error).bg(self.bg)
    }

    /// 进度条样式
    pub fn progress(&self) -> Style {
        Style::new().fg(self.progress).bg(self.bg)
    }

    /// 创建居中的标题行
    pub fn centered_title(&self, text: String) -> Line<'static> {
        Line::from(text).centered().style(self.title())
    }

    /// 创建带样式的行
    pub fn styled_line<'a>(&self, text: String, style: Style) -> Line<'a> {
        Line::from(text).style(style)
    }
}

/// 全局主题实例
pub static THEME: Theme = Theme {
    bg: Color::Black,
    fg: Color::White,
    accent: Color::Cyan,
    selected_bg: Color::Cyan,
    selected_fg: Color::Black,
    success: Color::Green,
    warning: Color::Yellow,
    error: Color::Red,
    hint: Color::Gray,
    border: Color::Cyan,
    progress: Color::Cyan,
    title: Color::Cyan,
};

/// 获取全局主题引用
pub fn theme() -> &'static Theme {
    &THEME
}
