//! 通用选择状态

use ratatui::widgets::ListState;

/// 可选择列表通用行为
pub trait Selectable {
    /// 总选项数
    fn count(&self) -> usize;
    /// 获取列表状态引用
    fn list_state(&self) -> &ListState;
    /// 获取列表状态可变引用
    fn list_state_mut(&mut self) -> &mut ListState;

    /// 选择下一个
    fn next(&mut self) {
        let count = self.count();
        if count == 0 {
            return;
        }
        if let Some(i) = self.list_state().selected() {
            self.list_state_mut().select(Some((i + 1) % count));
        }
    }

    /// 选择上一个
    fn prev(&mut self) {
        let count = self.count();
        if count == 0 {
            return;
        }
        if let Some(i) = self.list_state().selected() {
            let prev = if i == 0 {
                count.saturating_sub(1)
            } else {
                i - 1
            };
            self.list_state_mut().select(Some(prev));
        }
    }

    /// 指定索引
    fn select(&mut self, index: usize) {
        let count = self.count();
        if count == 0 {
            self.list_state_mut().select(None);
            return;
        }
        self.list_state_mut().select(Some(index % count));
    }

    /// 获取当前选中索引（有默认）
    fn selected_or_default(&self) -> usize {
        self.list_state().selected().unwrap_or(0)
    }

    /// 获取当前选中索引
    fn selected(&self) -> Option<usize> {
        self.list_state().selected()
    }
}

/// 列表选择状态
#[derive(Debug, Default)]
pub struct SelectionState {
    /// Ratatui 列表状态
    pub list_state: ListState,
    /// 总选项数
    pub count: usize,
}

impl SelectionState {
    /// 创建选择状态
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
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// 设置选中索引
    pub fn select(&mut self, index: usize) {
        if self.count == 0 {
            self.list_state.select(None);
            return;
        }
        self.list_state.select(Some(index % self.count));
    }
}

impl Selectable for SelectionState {
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
