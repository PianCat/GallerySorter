//! 事件处理模块
//!
//! 使用crossterm进行终端事件处理。

use crossterm::{
    ExecutableCommand,
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
};
use std::time::Duration;

/// 事件轮询间隔（毫秒）
const TICK_RATE: u64 = 50;

/// 事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiEvent {
    /// 键按下
    Key(KeyCode),
    /// Enter键
    Enter,
    /// Escape键
    Escape,
    /// 上箭头
    Up,
    /// 下箭头
    Down,
    /// 左箭头
    Left,
    /// 右箭头
    Right,
    /// Tab键
    Tab,
    /// Backspace键
    Backspace,
    /// Delete键
    Delete,
    /// 字符输入
    Char(char),
    /// Ctrl+C退出
    CtrlC,
    /// 窗口调整
    Resize(u16, u16),
    /// Home键
    Home,
    /// End键
    End,
    /// 无事件（超时）
    None,
}

impl From<Event> for TuiEvent {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key_event) => key_event.into(),
            Event::Resize(width, height) => TuiEvent::Resize(width, height),
            Event::Mouse(_) => TuiEvent::None,
            Event::FocusGained => TuiEvent::None,
            Event::FocusLost => TuiEvent::None,
            Event::Paste(_) => TuiEvent::None,
        }
    }
}

impl From<KeyEvent> for TuiEvent {
    fn from(key: KeyEvent) -> Self {
        // 忽略非按下事件
        if key.kind != KeyEventKind::Press {
            return TuiEvent::None;
        }

        // 处理Ctrl+C和Ctrl+D退出
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
        {
            return TuiEvent::CtrlC;
        }

        match key.code {
            KeyCode::Esc => TuiEvent::Escape,
            KeyCode::Enter => TuiEvent::Enter,
            KeyCode::Up => TuiEvent::Up,
            KeyCode::Down => TuiEvent::Down,
            KeyCode::Left => TuiEvent::Left,
            KeyCode::Right => TuiEvent::Right,
            KeyCode::Tab => TuiEvent::Tab,
            KeyCode::Backspace => TuiEvent::Backspace,
            KeyCode::Delete => TuiEvent::Delete,
            KeyCode::Char(c) => TuiEvent::Char(c),
            KeyCode::F(_) => TuiEvent::None,
            KeyCode::Null => TuiEvent::None,
            KeyCode::CapsLock => TuiEvent::None,
            KeyCode::NumLock => TuiEvent::None,
            KeyCode::ScrollLock => TuiEvent::None,
            KeyCode::Home => TuiEvent::Home,
            KeyCode::End => TuiEvent::End,
            KeyCode::PageUp => TuiEvent::None,
            KeyCode::PageDown => TuiEvent::None,
            KeyCode::Insert => TuiEvent::None,
            _ => TuiEvent::None,
        }
    }
}

/// 事件轮询器
#[derive(Debug)]
pub struct EventPoll {
    tick_rate: Duration,
}

impl EventPoll {
    /// 创建新的事件轮询器
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// 创建默认的事件轮询器
    pub fn default() -> Self {
        Self::new(Duration::from_millis(TICK_RATE))
    }

    /// 轮询下一个事件
    pub fn next(&self) -> TuiEvent {
        if event::poll(self.tick_rate).unwrap_or(false) {
            event::read()
                .unwrap_or_else(|_| Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::empty())))
                .into()
        } else {
            TuiEvent::None
        }
    }

    /// 尝试立即获取事件（非阻塞）
    pub fn try_next(&self) -> Option<TuiEvent> {
        if event::poll(Duration::ZERO).unwrap_or(false) {
            Some(
                event::read()
                    .unwrap_or_else(|_| {
                        Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::empty()))
                    })
                    .into(),
            )
        } else {
            None
        }
    }
}

impl Default for EventPoll {
    fn default() -> Self {
        Self::new(Duration::from_millis(TICK_RATE))
    }
}

/// 启用bracketed paste模式
pub fn enable_bracketed_paste() -> std::io::Result<()> {
    std::io::stdout().execute(EnableBracketedPaste)?;
    Ok(())
}

/// 禁用bracketed paste模式
pub fn disable_bracketed_paste() -> std::io::Result<()> {
    std::io::stdout().execute(DisableBracketedPaste)?;
    Ok(())
}
