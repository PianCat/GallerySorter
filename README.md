# Gallery Sorter

**[中文文档](README_zh.md)** | English

A professional command-line tool for organizing photos and videos by their creation time. Built with Rust for high performance and reliability.

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
  - Parallel processing with Rayon
  - Efficient buffered I/O for large files
  - Sampled hashing for files over 100MB

- **Interactive & CLI Modes**:
  - Interactive wizard for first-time users
  - Full CLI support with configuration files for automation

- **Bilingual Support**: English and Chinese (Simplified) interface

## Installation

### Download From Releases

Download the latest release from the [GitHub Releases](https://github.com/PianCat/GallerySorter/releases) page. Choose the appropriate binary for your OS (Windows, macOS, Linux).

### Optional: FFprobe for Video Metadata

For video metadata extraction, install FFprobe:

- **Windows**: Download from [FFmpeg](https://ffmpeg.org/download.html), add to PATH
- **macOS**: `brew install ffmpeg`
- **Linux**: `apt install ffmpeg` or equivalent

### Advanced: Building from Source

Requires Rust 2024 edition or later.

```bash
git clone https://github.com/yourusername/gallery-sorter.git
cd gallery-sorter
cargo build --release
```

The binary will be at `target/release/gallery-sorter` (or `gallery-sorter.exe` on Windows).

## Usage

### Interactive Mode

Simply run the program without arguments:

```bash
./gallery-sorter
```

This launches an interactive wizard to configure and run the sorter.

### CLI Mode

```bash
# Basic usage
./gallery-sorter -i /path/to/photos -o /path/to/sorted

# With configuration file
./gallery-sorter -c MyConfig

# Full options
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

### Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--input` | `-i` | Input directory (can specify multiple) |
| `--output` | `-o` | Output directory |
| `--config` | `-c` | Configuration file path or name |
| `--mode` | `-m` | Processing mode: `full`, `supplement`, `incremental` |
| `--classify` | | Classification: `none`, `year`, `year-month` |
| `--month-format` | | Month format: `nested`, `combined` |
| `--operation` | | File operation: `copy`, `move`, `hardlink`, `symlink` |
| `--deduplicate` | | Enable deduplication |
| `--no-deduplicate` | | Disable deduplication |
| `--dry-run` | | Show what would be done without doing it |
| `--verbose` | `-v` | Verbose output |
| `--json-log` | | Output logs in JSON format |

## Configuration Files

Place configuration files in the `Config/` directory next to the executable. Use `.toml` format.

### Example: Config/Template.toml

```toml
# Input directories to scan
input_dirs = [
    "D:/DCIM",
    "D:/Camera",
]

# Output directory
output_dir = "D:/Photos/Sorted"

# Processing mode: "full", "supplement", "incremental"
processing_mode = "incremental"

# Classification: "none", "year", "year-month"
classification = "year-month"

# Month format: "nested" (YYYY/MM/) or "combined" (YYYY-MM/)
month_format = "nested"

# File operation: "copy", "move", "hardlink", "symlink"
operation = "copy"

# Enable content-based deduplication
deduplicate = true

# Preview mode (no actual file operations)
dry_run = false
```

Use with: `./gallery-sorter -c Template`

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

```
Output Directory/
├── .gallery_sorter_increment_metadata.toml  # Watermark file (incremental mode)
├── .gallery_sorter_state.json               # Processing state (incremental mode)
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

## Logs

Log files are saved in the `Log/` directory next to the executable:
- Interactive mode: `Interactive_YYYYMMDD_HHMMSS.log`
- CLI with config: `ConfigName.log`
- CLI without config: `CLIRun_YYYYMMDD_HHMMSS.log`

## Performance Tips

1. **Use incremental mode** for regular imports - it skips timestamp comparison for older files
2. **Use hardlink operation** on the same filesystem to save disk space
3. **Disable deduplication** if you're sure there are no duplicates
4. **Use dry-run first** to preview changes before actual processing

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
