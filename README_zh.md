# Gallery Sorter 相册整理工具

**[English](README.md)** | 中文文档

一个照片和视频整理的命令行工具，基于文件创建时间进行分类。使用 Rust 构建，具有高性能和高可靠性。

## 功能特性

- **智能时间提取**：从多个来源自动提取创建时间，按优先级依次尝试：
  1. EXIF 元数据（适用于图片：JPEG、PNG、HEIC、HEIF、AVIF、TIFF、RAW 格式）
  2. 视频元数据（通过 FFprobe，适用于：MP4、MOV、AVI、MKV 等）
  3. 文件名解析（如 `IMG_20241005_182840.jpg`）
  4. 文件系统修改时间（最后备选）

- **智能去重**：使用 xxHash (xxh3) 进行快速内容哈希去重。发现重复文件时，自动保留文件名最简洁的版本。

- **灵活的分类方式**：
  - 仅按年份分类（`2024/`）
  - 按年月嵌套分类（`2024/10/`）
  - 按年月组合分类（`2024-10/`）
  - 不分类（平铺结构）

- **三种处理模式**：
  - **增量模式**（默认）：只处理比目标目录中最新文件更新的文件。适合日常照片导入。
  - **补充模式**：处理所有源文件，但跳过目标目录中已存在的相同内容文件。
  - **完整模式**：处理所有文件，覆盖目标目录中的现有文件。

- **高性能**：
  - 使用 Rayon 进行并行处理
  - 大文件高效缓冲 I/O
  - 超过 100MB 的文件采用采样哈希

- **交互式和命令行双模式**：
  - 交互式向导，适合首次使用
  - 完整的命令行支持，可配合配置文件实现自动化

- **双语支持**：支持中文和英文界面

## 使用

### 从 Github Releases 下载

从 [GitHub Releases](https://github.com/PianCat/GallerySorter/releases) 页面下载最新版本。选择适合您操作系统的二进制文件（Windows、macOS、Linux）。

### 可选：安装 FFprobe 以支持视频元数据

如需提取视频元数据，请安装 FFprobe：

- **Windows**：从 [FFmpeg 官网](https://ffmpeg.org/download.html) 下载，添加到 PATH
- **macOS**：`brew install ffmpeg`
- **Linux**：`apt install ffmpeg` 或相应包管理器命令

### 进阶：从源码编译

需要 Rust 2024 版本或更高版本。

```bash
git clone https://github.com/yourusername/gallery-sorter.git
cd gallery-sorter
cargo build --release
```

编译后的程序位于 `target/release/gallery-sorter`（Windows 上为 `gallery-sorter.exe`）。

## 使用方法

### 交互式模式

直接运行程序（不带参数）：

```bash
./gallery-sorter
```

这将启动交互式配置向导。

### 命令行模式

```bash
# 基本用法
./gallery-sorter -i /path/to/photos -o /path/to/sorted

# 使用配置文件
./gallery-sorter -c MyConfig

# 完整参数示例
./gallery-sorter \
  -i /path/to/photos \
  -i /path/to/more/photos \
  -o /path/to/sorted \
  -m incremental \
  --classify year-month \
  --month-format nested \
  --operation copy \
  --deduplicate \
  --dry-run
```

### 命令行参数

| 参数 | 简写 | 说明 |
|------|------|------|
| `--input` | `-i` | 输入目录（可指定多个） |
| `--output` | `-o` | 输出目录 |
| `--config` | `-c` | 配置文件路径或名称 |
| `--mode` | `-m` | 处理模式：`full`、`supplement`、`incremental` |
| `--classify` | | 分类规则：`none`、`year`、`year-month` |
| `--month-format` | | 月份格式：`nested`、`combined` |
| `--operation` | | 文件操作：`copy`、`move`、`hardlink`、`symlink` |
| `--deduplicate` | | 启用去重 |
| `--no-deduplicate` | | 禁用去重 |
| `--dry-run` | | 预览模式，显示操作但不执行 |
| `--verbose` | `-v` | 详细输出 |
| `--json-log` | | 以 JSON 格式输出日志 |

## 配置文件

将配置文件放在程序同目录下的 `Config/` 文件夹中，使用 `.toml` 格式。

### 示例：Config/Template.toml

```toml
# 要扫描的输入目录
input_dirs = [
    "D:/DCIM",
    "D:/Camera",
]

# 输出目录
output_dir = "D:/Photos/Sorted"

# 处理模式："full"、"supplement"、"incremental"
processing_mode = "incremental"

# 分类规则："none"、"year"、"year-month"
classification = "year-month"

# 月份格式："nested"（YYYY/MM/）或 "combined"（YYYY-MM/）
month_format = "nested"

# 文件操作："copy"、"move"、"hardlink"、"symlink"
operation = "copy"

# 启用基于内容的去重
deduplicate = true

# 预览模式（不执行实际操作）
dry_run = false
```

使用方法：`./gallery-sorter -c Template`

## 处理模式详解

### 增量模式（默认）

适用于日常从相机/手机导入照片：

1. 扫描输出目录，找到最新文件的时间戳（水位线）
2. 只处理时间戳比水位线新的源文件
3. 跳过目标目录中已存在相同内容的文件
4. 对于同名但内容不同的文件，添加数字后缀（`_1`、`_2`）

### 补充模式

适用于合并照片集合：

1. 处理所有源文件
2. 将内容哈希与目标目录中的现有文件进行比较
3. 跳过已存在相同内容的文件
4. 对于同名但内容不同的文件，添加数字后缀

### 完整模式

适用于首次整理或重新整理：

1. 处理所有源文件
2. 覆盖目标目录中的现有文件
3. 请谨慎使用 - 可能会覆盖文件！

## 重复文件处理

当检测到重复文件（相同内容哈希）时：

1. **源文件之间**：保留文件名最简洁的文件（最短，不含 `_1`、` - 副本`、`(1)` 等复制标记）
2. **与目标文件比较**：行为取决于处理模式（见上文）

识别的重复标记：
- 数字后缀：`_1`、`_2`、` 1`、` 2`
- 括号：`(1)`、`(2)`
- 复制关键词：`- copy`、`- 副本`

## 支持的格式

### 图片（支持 EXIF 提取）
`jpg`、`jpeg`、`png`、`webp`、`heic`、`heif`、`avif`、`tiff`、`tif`、`gif`、`bmp`

### RAW 格式
`raw`、`arw`、`cr2`、`cr3`、`nef`、`orf`、`rw2`、`dng`、`raf`、`srw`、`pef`

### 视频（通过 FFprobe 提取元数据）
`mp4`、`mov`、`avi`、`mkv`、`wmv`、`flv`、`m4v`、`3gp`

## 输出目录结构

```
输出目录/
├── .gallery_sorter_increment_metadata.toml  # 水位线文件（增量模式）
├── .gallery_sorter_state.json               # 处理状态文件（增量模式）
├── 2024/
│   ├── 01/
│   │   ├── IMG_20240115_143022.jpg
│   │   └── VID_20240120_183045.mp4
│   └── 02/
│       └── DSC_0001.jpg
└── 2023/
    └── 12/
        └── photo.heic
```

## 日志文件

日志文件保存在程序同目录下的 `Log/` 文件夹中：
- 交互式模式：`Interactive_YYYYMMDD_HHMMSS.log`
- 使用配置文件的命令行模式：`配置名称.log`
- 不使用配置文件的命令行模式：`CLIRun_YYYYMMDD_HHMMSS.log`

## 性能优化建议

1. **使用增量模式** 进行日常导入 - 它会跳过对旧文件的时间戳比较
2. **使用硬链接操作** 在同一文件系统上节省磁盘空间
3. **禁用去重** 如果确定没有重复文件（可加快处理速度）
4. **先使用预览模式** 在实际处理前预览更改

## 许可证

MIT 许可证 - 详见 LICENSE 文件。

## 参与贡献

欢迎贡献！请随时提交 Issue 和 Pull Request。
