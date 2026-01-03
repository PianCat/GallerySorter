//! 进度状态

use crate::process::ProcessingStats;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// 进度状态
#[derive(Debug)]
pub struct ProgressState {
    /// 处理统计
    pub stats: Arc<ProcessingStats>,
    /// 总文件数
    pub total_files: usize,
    /// 当前文件
    pub current_file: String,
}

impl ProgressState {
    /// 创建进度状态
    pub fn new(stats: Arc<ProcessingStats>, total_files: usize) -> Self {
        Self {
            stats,
            total_files,
            current_file: String::new(),
        }
    }

    /// 设置当前处理文件
    pub fn set_current_file(&mut self, file: &str) {
        let char_count = file.chars().count();
        self.current_file = if char_count > 50 {
            let tail: String = file
                .chars()
                .rev()
                .take(47)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!("...{}", tail)
        } else {
            file.to_string()
        };
    }

    /// 进度比例
    pub fn progress_ratio(&self) -> f64 {
        let processed = self.stats.processed.load(Ordering::Relaxed);
        let total = self.total_files;
        if total == 0 {
            0.0
        } else {
            processed as f64 / total as f64
        }
    }

    /// 已处理数
    pub fn processed(&self) -> usize {
        self.stats.processed.load(Ordering::Relaxed)
    }

    /// 已跳过数
    pub fn skipped(&self) -> usize {
        self.stats.skipped.load(Ordering::Relaxed)
    }

    /// 重复数
    pub fn duplicates(&self) -> usize {
        self.stats.duplicates.load(Ordering::Relaxed)
    }

    /// 失败数
    pub fn failed(&self) -> usize {
        self.stats.failed.load(Ordering::Relaxed)
    }
}
