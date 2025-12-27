//! xxHash-based file hashing for deduplication
//!
//! For regular files, computes the full file hash.
//! For large files (videos), samples 1MB from head, middle, and tail
//! to compute a fast approximate hash.

use crate::error::{Error, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tracing::trace;
use xxhash_rust::xxh3::xxh3_64;

/// Size of each sample chunk for large files (1MB)
const SAMPLE_SIZE: u64 = 1024 * 1024;

/// Compute hash for a file
///
/// For files smaller than the threshold, computes full file hash.
/// For larger files, samples from head, middle, and tail.
pub fn compute_file_hash(path: &Path, large_file_threshold: u64) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();

    if file_size > large_file_threshold {
        compute_sampled_hash(path, file_size)
    } else {
        compute_full_hash(path)
    }
}

/// Compute full file hash using xxHash3
fn compute_full_hash(path: &Path) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| Error::HashComputation {
            path: path.to_path_buf(),
            message: format!("Failed to read file: {}", e),
        })?;

    let hash = xxh3_64(&buffer);
    trace!(?path, hash, "Computed full file hash");
    Ok(hash)
}

/// Compute sampled hash for large files
///
/// Takes 1MB samples from:
/// - Beginning of file
/// - Middle of file
/// - End of file
///
/// Also incorporates file size into the hash for additional uniqueness
fn compute_sampled_hash(path: &Path, file_size: u64) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut hasher_data = Vec::with_capacity((SAMPLE_SIZE * 3 + 8) as usize);

    // Include file size in hash
    hasher_data.extend_from_slice(&file_size.to_le_bytes());

    // Read head sample
    let head_size = std::cmp::min(SAMPLE_SIZE, file_size);
    let mut head_buffer = vec![0u8; head_size as usize];
    file.read_exact(&mut head_buffer)
        .map_err(|e| Error::HashComputation {
            path: path.to_path_buf(),
            message: format!("Failed to read file head: {}", e),
        })?;
    hasher_data.extend_from_slice(&head_buffer);

    // Read middle sample (if file is large enough)
    if file_size > SAMPLE_SIZE * 2 {
        let middle_start = (file_size - SAMPLE_SIZE) / 2;
        file.seek(SeekFrom::Start(middle_start))
            .map_err(|e| Error::HashComputation {
                path: path.to_path_buf(),
                message: format!("Failed to seek to middle: {}", e),
            })?;
        let mut middle_buffer = vec![0u8; SAMPLE_SIZE as usize];
        file.read_exact(&mut middle_buffer)
            .map_err(|e| Error::HashComputation {
                path: path.to_path_buf(),
                message: format!("Failed to read file middle: {}", e),
            })?;
        hasher_data.extend_from_slice(&middle_buffer);
    }

    // Read tail sample (if file is large enough)
    if file_size > SAMPLE_SIZE {
        let tail_start = file_size - SAMPLE_SIZE;
        file.seek(SeekFrom::Start(tail_start))
            .map_err(|e| Error::HashComputation {
                path: path.to_path_buf(),
                message: format!("Failed to seek to tail: {}", e),
            })?;
        let mut tail_buffer = vec![0u8; SAMPLE_SIZE as usize];
        file.read_exact(&mut tail_buffer)
            .map_err(|e| Error::HashComputation {
                path: path.to_path_buf(),
                message: format!("Failed to read file tail: {}", e),
            })?;
        hasher_data.extend_from_slice(&tail_buffer);
    }

    let hash = xxh3_64(&hasher_data);
    trace!(?path, file_size, hash, "Computed sampled file hash");
    Ok(hash)
}

/// Compute a quick hash based on file metadata only
/// Used for incremental processing to detect file changes
pub fn compute_metadata_hash(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    let mut data = Vec::new();

    // Include file size
    data.extend_from_slice(&metadata.len().to_le_bytes());

    // Include modification time if available
    if let Ok(modified) = metadata.modified()
        && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
    {
        data.extend_from_slice(&duration.as_secs().to_le_bytes());
        data.extend_from_slice(&duration.subsec_nanos().to_le_bytes());
    }

    Ok(xxh3_64(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_full_hash() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();
        file.flush().unwrap();

        let hash = compute_full_hash(file.path()).unwrap();
        assert!(hash != 0);

        // Same content should produce same hash
        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"test content").unwrap();
        file2.flush().unwrap();

        let hash2 = compute_full_hash(file2.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let mut file1 = NamedTempFile::new().unwrap();
        file1.write_all(b"content 1").unwrap();
        file1.flush().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"content 2").unwrap();
        file2.flush().unwrap();

        let hash1 = compute_full_hash(file1.path()).unwrap();
        let hash2 = compute_full_hash(file2.path()).unwrap();

        assert_ne!(hash1, hash2);
    }
}
