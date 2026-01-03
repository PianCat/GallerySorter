# Gallery Sorter 相册整理工具

**[English](README.md)** | 中文文档

Gallery Sorter 是一个结合 CLI 与 TUI 的照片/视频整理工具，基于创建时间自动归档。程序会依次尝试 EXIF、视频元数据（FFprobe）、文件名和文件系统时间，并将文件整理为清晰的目录结构。

## 功能亮点

- 多来源时间提取（EXIF → FFprobe → 文件名 → 文件系统时间）
- 使用 xxHash (xxh3) 的高速去重
- 灵活的分类方式：无分类/按年/按年月，月份支持嵌套或组合格式
- 处理模式：增量（默认）、补充、完整
- 并行处理、可配置线程数与试运行模式
- Ratatui 交互向导 + 完整 CLI 自动化
- 中英文双语界面

## 安装

### 从 Releases 下载

从 [GitHub Releases](https://github.com/PianCat/GallerySorter/releases) 下载最新二进制文件。

### 可选：安装 FFprobe（视频元数据）

如需提取视频元数据，请安装 FFprobe（FFmpeg 自带）：

- Windows：从 https://ffmpeg.org/download.html 下载并加入 PATH
- macOS：`brew install ffmpeg`
- Linux：`apt install ffmpeg` 或对应包管理器命令

### 从源码编译

需要 Rust 2024 版本。

```bash
git clone https://github.com/PianCat/GallerySorter.git
cd GallerySorter/GallerySorter_RS
cargo build --release
```

可执行文件位于 `target/release/gallery-sorter`（Windows 为 `gallery-sorter.exe`）。

## 使用方法

### TUI 模式

不带参数运行即可启动 Ratatui 向导：

```bash
gallery-sorter
```

### CLI 模式

```bash
# 基本用法
gallery-sorter -i /path/to/photos -o /path/to/sorted

# 使用配置文件（解析自 Config/Name.toml）
gallery-sorter -C Name

# 完整示例
gallery-sorter \
  -i /path/to/photos \
  -i /path/to/more/photos \
  -o /path/to/sorted \
  -M incremental \
  --classify year-month \
  --month-format nested \
  --operation copy \
  --deduplicate \
  --dry-run
```

### 命令行参数

| 参数 | 简写 | 说明 |
|------|------|------|
| `--config` | `-C` | 配置文件路径或名称（TOML） |
| `--input` | `-i` | 输入目录（可多次指定） |
| `--output` | `-o` | 输出目录 |
| `--mode` | `-M` | `full`、`supplement`、`incremental` |
| `--classify` | `-c` | `none`、`year`、`year-month` |
| `--month-format` | `-m` | `nested`、`combined` |
| `--classify-by-type` |  | 添加 `Photos/Videos/Raw` 子目录 |
| `--operation` | `-O` | `copy`、`move`、`hardlink`、`symlink` |
| `--no-deduplicate` |  | 禁用去重 |
| `--state-file` |  | 增量模式状态文件路径 |
| `--threads` | `-t` | 线程数（0 = 自动） |
| `--large-file-mb` |  | 大文件阈值（MB） |
| `--dry-run` | `-n` | 试运行，仅预览 |
| `--verbose` | `-v` | 详细输出 |
| `--json-log` |  | JSON 日志 |

## 配置文件

配置文件会从可执行文件同级的 `Config/` 目录读取。仓库中的 `Template.toml` 可作为模板，保存为 `Config/Name.toml`。

CLI 参数会覆盖配置文件中的设置。

示例：

```toml
input_dirs = ["D:/Photos", "D:/Videos"]
output_dir = "D:/Sorted"
processing_mode = "incremental"
classification = "year-month"
month_format = "nested"
classify_by_type = false
operation = "copy"
deduplicate = true
dry_run = false
verbose = false
```

运行方式：

```bash
gallery-sorter -C Name
```

## 输出结构

默认（按年月、嵌套）：

```
Output/
├── 2024/
│   └── 01/
│       ├── IMG_20240115_143022.jpg
│       └── VID_20240120_183045.mp4
└── 2023/
    └── 12/
        └── photo.heic
```

启用文件类型分类：

```
Output/
└── 2024/
    └── 01/
        ├── Photos/
        │   ├── IMG_20240115_143022.jpg
        │   └── Raw/
        │       └── DSC_0001.arw
        └── Videos/
            └── VID_20240120_183045.mp4
```

## 日志

日志保存在可执行文件同级的 `Log/` 目录：

- TUI：`Log/Interactive_YYYYMMDD_HHMMSS.log`
- CLI（配置文件）：`Log/ConfigName/ConfigName_YYYYMMDD_HHMMSS.log`
- CLI（无配置）：`Log/CLIRun_YYYYMMDD_HHMMSS.log`

## 许可证

GPL-3.0，详见 `LICENSE`。
