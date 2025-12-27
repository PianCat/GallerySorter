//! 显示摘要模块
//!
//! 提供处理完成后的摘要显示功能。

use crate::process::{FileResult, ProcessingStats, ProcessingStatus};
use std::sync::atomic::Ordering;

/// 显示处理摘要
pub fn display_summary(stats: &ProcessingStats, results: &[FileResult], dry_run: bool) {
    println!("\n{}", "═".repeat(60));
    println!("{:^60}", "处理完成");
    println!("{}", "═".repeat(60));

    let total = stats.total_files.load(Ordering::Relaxed);
    let processed = stats.processed.load(Ordering::Relaxed);
    let skipped = stats.skipped.load(Ordering::Relaxed);
    let duplicates = stats.duplicates.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    println!("\n  # 统计信息");
    println!("  {}", "─".repeat(40));
    println!("    > 总文件数       {}", total);
    println!("    ✓ 已处理         {}", processed);
    println!("    ○ 跳过           {}", skipped);
    println!("    ◎ 重复           {}", duplicates);
    println!("    ✗ 失败           {}", failed);

    // 显示失败的文件
    let failed_files: Vec<_> = results
        .iter()
        .filter(|r| r.status == ProcessingStatus::Failed)
        .collect();

    if !failed_files.is_empty() {
        println!("\n  ! 失败的文件");
        println!("  {}", "─".repeat(40));
        for (i, result) in failed_files.iter().take(5).enumerate() {
            println!(
                "    {}. {} - {}",
                i + 1,
                result
                    .source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                result.error.as_deref().unwrap_or("未知错误")
            );
        }
        if failed_files.len() > 5 {
            println!("    以及其他 {} 个文件", failed_files.len() - 5);
        }
    }

    if dry_run {
        println!("\n  * 试运行模式（无实际文件操作）");
    }

    println!("\n{}", "═".repeat(60));
}

/// 检查是否应该运行交互模式（没有提供参数时）
pub fn should_run_interactive() -> bool {
    let args: Vec<String> = std::env::args().collect();
    // 只有程序名称，没有其他参数
    args.len() == 1
}
