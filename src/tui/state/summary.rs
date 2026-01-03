//! 摘要状态

use crate::process::{FileResult, ProcessingStats};
use std::path::PathBuf;

/// 结果摘要状态
#[derive(Debug)]
pub struct SummaryState {
    /// 处理统计
    pub stats: ProcessingStats,
    /// 处理结果
    pub results: Vec<FileResult>,
    /// 是否试运行
    pub dry_run: bool,
    /// 日志路径
    pub log_path: Option<PathBuf>,
}

impl SummaryState {
    /// 创建摘要状态
    pub fn new(
        stats: ProcessingStats,
        results: Vec<FileResult>,
        dry_run: bool,
        log_path: Option<PathBuf>,
    ) -> Self {
        Self {
            stats,
            results,
            dry_run,
            log_path,
        }
    }
}
