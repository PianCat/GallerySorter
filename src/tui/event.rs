//! Event handling module
//!
//! Uses crossterm for terminal event handling.

use crossterm::{
    ExecutableCommand,
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
};
use std::time::Duration;

/// Event poll interval (milliseconds)
const TICK_RATE: u64 = 50;

/// Event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiEvent {
    /// Key press
    Key(KeyCode),
    /// Enter key
    Enter,
    /// Escape key
    Escape,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Tab key
    Tab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Character input
    Char(char),
    /// Ctrl+C exit
    CtrlC,
    /// Window resize
    Resize(u16, u16),
    /// Home key
    Home,
    /// End key
    End,
    /// No event (timeout)
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
        // Ignore non-press events
        if key.kind != KeyEventKind::Press {
            return TuiEvent::None;
        }

        // Handle Ctrl+C and Ctrl+D exit
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

/// Event poller
#[derive(Debug)]
pub struct EventPoll {
    tick_rate: Duration,
}

impl EventPoll {
    /// Create new event poller
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Create default event poller
    pub fn default() -> Self {
        Self::new(Duration::from_millis(TICK_RATE))
    }

    /// Poll next event
    pub fn next(&self) -> TuiEvent {
        if event::poll(self.tick_rate).unwrap_or(false) {
            event::read()
                .unwrap_or_else(|_| Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::empty())))
                .into()
        } else {
            TuiEvent::None
        }
    }

    /// Try to get event immediately (non-blocking)
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

/// Enable bracketed paste mode
pub fn enable_bracketed_paste() -> std::io::Result<()> {
    std::io::stdout().execute(EnableBracketedPaste)?;
    Ok(())
}

/// Disable bracketed paste mode
pub fn disable_bracketed_paste() -> std::io::Result<()> {
    std::io::stdout().execute(DisableBracketedPaste)?;
    Ok(())
}
