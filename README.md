# Gallery Sorter

**[中文文档](README_zh.md)** | English

A CLI tool for organizing photos and videos by their creation time. Built with Rust for high performance and reliability.

## Features

- **Intelligent Time Extraction**: Extracts creation timestamps from multiple sources with automatic fallback:
  1. EXIF metadata (for images: JPEG, PNG, HEIC, HEIF, AVIF, TIFF, RAW formats)
  2. Video metadata via FFprobe (for videos: MP4, MOV, AVI, MKV, etc.)
  3. Filename patterns (e.g., `IMG_20241005_182840.jpg`)
  4. File system modification time (last resort)

- **Smart Deduplication**: Uses xxHash (xxh3) for fast content-based deduplication. Automatically keeps the file with the cleanest filename when duplicates are found.

- **Flexible Organization**: Organize files by:
  - Year only (`2024/`)
  - Year and month nested (`2024/10/`)
  - Year and month combined (`2024-10/`)
  - No classification (flat structure)

- **Three Processing Modes**:
  - **Incremental** (default): Only processes files newer than the newest file in target directory. Perfect for regular photo imports.
  - **Supplement**: Processes all source files but skips those already existing in target.
  - **Full**: Processes all files, overwrites existing files in target.

- **High Performance**:
  - Parallel processing with configurable thread count
  - Efficient buffered I/O for large files
  - Sampled hashing for large files (configurable threshold)

- **Interactive & CLI Modes**:
  - Interactive TUI wizard with Ratatui for first-time users
  - Full CLI support with configuration files for automation

- **Bilingual Support**: English and Chinese (Simplified) interface

- **Advanced Configuration**:
  - Exclude specific directories from scanning
  - Customizable file extension lists
  - Configurable state file for incremental processing

## Installation

### Download From Releases

Download the latest release from the [GitHub Releases](https://github.com/PianCat/GallerySorter/releases) page. Choose the appropriate binary for your OS (Windows, macOS, Linux).

### Optional: FFprobe for Video Metadata

For video metadata extraction, install FFprobe:

- **Windows**: Download from [FFmpeg](https://ffmpeg.org/download.html), add to PATH
- **macOS**: `brew install ffmpeg`
- **Linux**: `apt install ffmpeg` or equivalent

### Building from Source

Requires Rust 2024 edition or later.

```bash
git clone https://github.com/PianCat/GallerySorter.git
cd GallerySorter/GallerySorter_RS
cargo build --release
```

The binary will be at `target/release/gallery-sorter` (or `gallery-sorter.exe` on Windows).

## Usage

### Interactive Mode

Simply run the program without arguments:

```bash
./gallery-sorter
```

This launches an interactive TUI wizard to configure and run the sorter.

### CLI Mode

```bash
# Basic usage
./gallery-sorter -i /path/to/photos -o /path/to/sorted

# With configuration file
./gallery-sorter -C MyConfig

# Full options
./gallery-sorter \
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

### Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--config` | `-C` | Path to configuration file (TOML format) |
| `--input` | `-i` | Input directories to scan (can specify multiple) |
| `--output` | `-o` | Output directory for organized files |
| `--mode` | `-M` | Processing mode: `full`, `supplement`, `incremental` |
| `--classify` | `-c` | Classification rule: `none`, `year`, `year-month` |
| `--month-format` | `-m` | Month format: `nested`, `combined` |
| `--classify-by-type` | | Classify by file type (adds Photos/Videos/RAW subdirectory) |
| `--operation` | `-O` | File operation: `copy`, `move`, `hardlink`, `symlink` |
| `--no-deduplicate` | | Disable file deduplication |
| `--state-file` | | State file path for tracking processed files |
| `--threads` | `-t` | Number of threads for parallel processing (0 = auto) |
| `--large-file-mb` | | Large file threshold in MB (files larger use sampled hashing) |
| `--dry-run` | `-n` | Show what would be done without doing it |
| `--verbose` | `-v` | Verbose output |
| `--json-log` | | Output logs in JSON format |

## Configuration Files

Place configuration files in the `Config/` directory next to the executable. Use `.toml` format.

### Example: Config/Template.toml

```toml
# Gallery Sorter Configuration File

# Input directories to scan for media files
input_dirs = [
    "D:/Photos",
    "D:/Videos",
]

# Output directory for organized files
output_dir = "D:/Sorted"

# Directories to exclude from scanning
# Can be absolute paths or folder names (will match any folder with that name)
exclude_dirs = [
    ".sync",
    ".thumbnails",
    "@eaDir",
]

# Processing mode: "full", "supplement", or "incremental"
processing_mode = "incremental"

# Classification rule: "none", "year", or "year-month"
classification = "year-month"

# Month format: "nested" or "combined"
month_format = "nested"

# Classify by file type (adds Photos/Videos subdirectory, RAW files nested under Photos/Raw)
classify_by_type = false

# File operation: "copy", "move", "symlink", or "hardlink"
operation = "copy"

# Enable file deduplication
deduplicate = true

# State file path for incremental processing
# state_file = ".gallery_sorter_state.json"

# Number of threads for parallel processing (0 = auto-detect)
threads = 0

# Large file threshold in bytes (files larger use sampled hashing)
# Default: 100MB = 104857600 bytes
large_file_threshold = 104857600

# Dry run mode
dry_run = false

# Verbose output
verbose = false

# Supported file extensions (customize as needed)
image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "avif", "tiff", "tif"]
video_extensions = ["mp4", "mov", "avi", "mkv", "wmv", "flv", "m4v", "3gp"]
raw_extensions = ["raw", "arw", "cr2", "cr3", "nef", "orf", "rw2", "dng", "raf", "srw", "pef"]
```

Use with: `./gallery-sorter -C Template`

## Processing Modes Explained

### Incremental Mode (Default)

Best for regular photo imports from camera/phone:

1. Scans output directory to find the newest file's timestamp (watermark)
2. Only processes source files with timestamps newer than the watermark
3. Skips files that already exist with identical content
4. Adds numeric suffix (`_1`, `_2`) for different files with same name

### Supplement Mode

Best for merging photo collections:

1. Processes all source files
2. Compares content hash with existing files in target
3. Skips files that already exist with identical content
4. Adds numeric suffix for different files with same name

### Full Mode

Best for initial organization or re-organizing:

1. Processes all source files
2. Overwrites existing files in target
3. Use with caution - may overwrite files!

## Duplicate Handling

When duplicates are detected (same content hash):

1. **Within source files**: Keeps the file with the cleanest filename (shortest, without copy indicators like `_1`, ` - copy`, `(1)`)
2. **Against target files**: Behavior depends on processing mode (see above)

Duplicate indicators recognized:
- Numeric suffixes: `_1`, `_2`, ` 1`, ` 2`
- Parenthetical: `(1)`, `(2)`
- Copy keywords: `- copy`, `- 副本` (Chinese)

## Supported Formats

### Images (EXIF extraction supported)
`jpg`, `jpeg`, `png`, `webp`, `heic`, `heif`, `avif`, `tiff`, `tif`, `gif`, `bmp`

### RAW Formats
`raw`, `arw`, `cr2`, `cr3`, `nef`, `orf`, `rw2`, `dng`, `raf`, `srw`, `pef`

### Videos (FFprobe metadata)
`mp4`, `mov`, `avi`, `mkv`, `wmv`, `flv`, `m4v`, `3gp`

## Output Structure

Default structure (classify_by_type = false):
```
Output Directory/
├── .gallery_sorter_increment_metadata.toml  # Watermark file (incremental mode)
├── .gallery_sorter_state.json               # State file (incremental mode)
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

With file type classification (classify_by_type = true):
```
Output Directory/
├── 2024/
│   └── 01/
│       ├── Photos/
│       │   ├── IMG_20240115_143022.jpg
│       │   └── Raw/
│       │       └── DSC_0001.arw
│       └── Videos/
│           └── VID_20240120_183045.mp4
```

## Logs

Log files are saved in the `Log/` directory next to the executable:
- Interactive mode: `Log/Interactive_YYYYMMDD_HHMMSS.log`
- CLI with config: `Log/ConfigName/ConfigName_YYYYMMDD_HHMMSS.log`
- CLI without config: `Log/CLIRun_YYYYMMDD_HHMMSS.log`

## Performance Tips

1. **Use incremental mode** for regular imports - it skips timestamp comparison for older files
2. **Use hardlink operation** on the same filesystem to save disk space
3. **Disable deduplication** if you're sure there are no duplicates
4. **Use dry-run first** to preview changes before actual processing
5. **Adjust thread count** based on your CPU cores for optimal performance
6. **Increase large file threshold** for SSDs, decrease for HDDs

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
