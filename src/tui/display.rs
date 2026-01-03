//! 汇总显示模块
//!
//! 提供处理完成后的摘要显示。

use crate::process::{FileResult, ProcessingStats, ProcessingStatus};
use rust_i18n::t;
use std::sync::atomic::Ordering;

/// 显示处理摘要
pub fn display_summary(stats: &ProcessingStats, results: &[FileResult], dry_run: bool) {
    println!("\n{}", "═".repeat(60));
    println!("{:^60}", t!("processing_complete"));
    println!("{}", "═".repeat(60));

    let total = stats.total_files.load(Ordering::Relaxed);
    let processed = stats.processed.load(Ordering::Relaxed);
    let skipped = stats.skipped.load(Ordering::Relaxed);
    let duplicates = stats.duplicates.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    println!("\n  {}", t!("statistics"));
    println!("  {}", "─".repeat(40));
    println!("    > {}: {}", t!("stat_total"), total);
    println!("    ✓ {}: {}", t!("stat_processed"), processed);
    println!("    ○ {}: {}", t!("stat_skipped"), skipped);
    println!("    ◎ {}: {}", t!("stat_duplicates"), duplicates);
    println!("    ✗ {}: {}", t!("stat_failed"), failed);

    let failed_files: Vec<_> = results
        .iter()
        .filter(|r| r.status == ProcessingStatus::Failed)
        .collect();

    let unknown_error = t!("unknown_error");

    if !failed_files.is_empty() {
        println!("\n  {}", t!("failed_files"));
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
                result.error.as_deref().unwrap_or(unknown_error.as_ref())
            );
        }
        if failed_files.len() > 5 {
            println!(
                "    {}",
                t!("failed_files_more", count = failed_files.len() - 5)
            );
        }
    }

    if dry_run {
        println!("\n  {}", t!("dry_run_notice"));
    }

    println!("\n{}", "═".repeat(60));
}

/// 是否运行交互模式（无参数时启用）
pub fn should_run_interactive() -> bool {
    let args: Vec<String> = std::env::args().collect();
    args.len() == 1
}
