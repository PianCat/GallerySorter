//! 输入状态

use unicode_width::UnicodeWidthStr;

/// 文本输入状态
#[derive(Debug, Default, Clone)]
pub struct InputState {
    buffer: String,
    cursor: usize,
}

impl InputState {
    /// 新建输入状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用初始值创建
    pub fn with_value(value: &str) -> Self {
        Self {
            buffer: value.to_string(),
            cursor: value.len(),
        }
    }

    /// 清空输入
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// 插入字符
    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// 删除光标前字符
    pub fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            let prev_char_len = self.buffer[..self.cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor -= prev_char_len;
            self.buffer.remove(self.cursor);
        }
    }

    /// 删除光标后字符
    pub fn delete_after_cursor(&mut self) {
        if self.cursor < self.buffer.len() {
            let next_char_len = self.buffer[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.buffer.drain(self.cursor..self.cursor + next_char_len);
        }
    }

    /// 光标左移
    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= self.buffer[..self.cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// 光标右移
    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += self.buffer[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// 移动到行首
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// 移动到行尾
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// 光标可视位置
    pub fn visual_cursor_position(&self) -> usize {
        self.buffer[..self.cursor].width()
    }

    /// 获取当前值
    pub fn value(&self) -> &str {
        &self.buffer
    }

    /// 获取光标位置
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// 设置缓冲区与光标位置
    pub fn set_buffer(&mut self, buffer: String, cursor: usize) {
        let mut safe_cursor = cursor.min(buffer.len());
        while !buffer.is_char_boundary(safe_cursor) {
            safe_cursor = safe_cursor.saturating_sub(1);
        }
        self.buffer = buffer;
        self.cursor = safe_cursor;
    }
}
