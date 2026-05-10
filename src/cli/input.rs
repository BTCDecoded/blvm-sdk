//! # CLI Input Parsing
//!
//! Input parsing and validation utilities for CLI tools.

use std::path::Path;
use std::str::FromStr;

/// Input validation errors
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Parse a file path and validate it exists
pub fn parse_file_path(path: &str) -> Result<String, InputError> {
    let path = Path::new(path);

    if !path.exists() {
        return Err(InputError::FileNotFound(path.to_string_lossy().to_string()));
    }

    Ok(path.to_string_lossy().to_string())
}

/// Parse a hex string
pub fn parse_hex(hex_str: &str) -> Result<Vec<u8>, InputError> {
    hex::decode(hex_str).map_err(|e| InputError::InvalidFormat(format!("Invalid hex string: {e}")))
}

/// Parse a base64 string
pub fn parse_base64(base64_str: &str) -> Result<Vec<u8>, InputError> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| InputError::InvalidFormat(format!("Invalid base64 string: {e}")))
}

/// Parse a number from string
pub fn parse_number<T>(value: &str) -> Result<T, InputError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|e| InputError::InvalidValue(format!("Invalid number: {e}")))
}

/// Parse a comma-separated list
pub fn parse_comma_separated(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Validate a threshold string (e.g., "3-of-5")
pub fn parse_threshold(threshold: &str) -> Result<(usize, usize), InputError> {
    let parts: Vec<&str> = threshold.split("-of-").collect();

    if parts.len() != 2 {
        return Err(InputError::InvalidFormat(
            "Threshold must be in format 'N-of-M'".to_string(),
        ));
    }

    let threshold_num = parts[0]
        .parse::<usize>()
        .map_err(|e| InputError::InvalidValue(format!("Invalid threshold number: {e}")))?;

    let total_num = parts[1]
        .parse::<usize>()
        .map_err(|e| InputError::InvalidValue(format!("Invalid total number: {e}")))?;

    if threshold_num > total_num {
        return Err(InputError::InvalidValue(
            "Threshold cannot be greater than total".to_string(),
        ));
    }

    Ok((threshold_num, total_num))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_hex() {
        let result = parse_hex("deadbeef");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_parse_invalid_hex() {
        let result = parse_hex("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_base64() {
        let result = parse_base64("dGVzdA==");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"test");
    }

    #[test]
    fn test_parse_number() {
        let result: Result<u32, _> = parse_number("42");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_parse_comma_separated() {
        let result = parse_comma_separated("a, b, c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_threshold() {
        let result = parse_threshold("3-of-5");
        assert_eq!(result.unwrap(), (3, 5));
    }

    #[test]
    fn test_parse_invalid_threshold() {
        let result = parse_threshold("3-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test").unwrap();

        let result = parse_file_path(file_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = parse_file_path("/nonexistent/file.txt");
        assert!(result.is_err());
    }
}
