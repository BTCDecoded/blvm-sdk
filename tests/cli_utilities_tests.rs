//! CLI Utilities Tests
//!
//! Tests for CLI input/output formatting and parsing utilities.

use bllvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use bllvm_sdk::cli::input::{parse_comma_separated, parse_threshold};
use std::error::Error;

// ============================================================================
// Phase 1: Output Format Tests
// ============================================================================

#[test]
fn test_output_format_text() {
    // Test text output format
    let format = OutputFormat::Text;
    let formatter = OutputFormatter::new(format);
    
    // Formatter should be created
    // Test by formatting something
    let result = formatter.format(&"test");
    assert!(result.is_ok());
}

#[test]
fn test_output_format_json() {
    // Test JSON output format
    let format = OutputFormat::Json;
    let formatter = OutputFormatter::new(format);
    
    // Formatter should be created
    // Test by formatting something
    let result = formatter.format(&"test");
    assert!(result.is_ok());
}

#[test]
fn test_output_format_from_str() {
    // Test parsing output format from string
    let text_format: Result<OutputFormat, _> = "text".parse();
    assert!(text_format.is_ok());
    assert_eq!(text_format.unwrap(), OutputFormat::Text);
    
    let json_format: Result<OutputFormat, _> = "json".parse();
    assert!(json_format.is_ok());
    assert_eq!(json_format.unwrap(), OutputFormat::Json);
}

#[test]
fn test_output_format_invalid() {
    // Test invalid output format
    let invalid: Result<OutputFormat, _> = "invalid".parse();
    assert!(invalid.is_err());
}

// ============================================================================
// Phase 2: Output Formatter Tests
// ============================================================================

#[test]
fn test_output_formatter_creation() {
    // Test creating output formatter
    let formatter = OutputFormatter::new(OutputFormat::Text);
    
    // Should be created successfully
    // Test by formatting something
    let result = formatter.format(&"test");
    assert!(result.is_ok());
}

#[test]
fn test_output_formatter_format_error() {
    // Test formatting errors
    let formatter = OutputFormatter::new(OutputFormat::Text);
    let error: Box<dyn Error> = Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found"
    ));
    
    let formatted = formatter.format_error(&*error);
    // Should format error (may vary by format)
    assert!(!formatted.is_empty());
}

#[test]
fn test_output_formatter_json_error() {
    // Test JSON error formatting
    let formatter = OutputFormatter::new(OutputFormat::Json);
    let error: Box<dyn Error> = Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found"
    ));
    
    let formatted = formatter.format_error(&*error);
    // JSON format should produce JSON output
    assert!(formatted.contains("error") || formatted.contains("Error"));
}

// ============================================================================
// Phase 3: Input Parsing Tests
// ============================================================================

#[test]
fn test_parse_comma_separated_single() {
    // Test parsing single value
    let result = parse_comma_separated("value1");
    assert_eq!(result, vec!["value1".to_string()]);
}

#[test]
fn test_parse_comma_separated_multiple() {
    // Test parsing multiple comma-separated values
    let result = parse_comma_separated("value1,value2,value3");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "value1");
    assert_eq!(result[1], "value2");
    assert_eq!(result[2], "value3");
}

#[test]
fn test_parse_comma_separated_with_spaces() {
    // Test parsing with spaces (should trim)
    let result = parse_comma_separated("value1 , value2 , value3");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "value1");
    assert_eq!(result[1], "value2");
    assert_eq!(result[2], "value3");
}

#[test]
fn test_parse_comma_separated_empty() {
    // Test parsing empty string (should return empty vec after filtering)
    let result: Vec<String> = parse_comma_separated("");
    // parse_comma_separated filters out empty strings
    assert_eq!(result, Vec::<String>::new());
}

#[test]
fn test_parse_threshold_valid() {
    // Test parsing valid threshold
    let result = parse_threshold("3-of-5");
    assert!(result.is_ok());
    let (threshold, total) = result.unwrap();
    assert_eq!(threshold, 3);
    assert_eq!(total, 5);
}

#[test]
fn test_parse_threshold_different_formats() {
    // Test parsing different threshold formats
    // Only "3-of-5" format is supported
    let result = parse_threshold("3-of-5");
    assert!(result.is_ok());
    let (threshold, total) = result.unwrap();
    assert_eq!(threshold, 3);
    assert_eq!(total, 5);
    
    // Other formats should fail
    assert!(parse_threshold("3/5").is_err());
    assert!(parse_threshold("3:5").is_err());
}

#[test]
fn test_parse_threshold_invalid() {
    // Test parsing invalid threshold
    let result = parse_threshold("invalid");
    assert!(result.is_err());
}

#[test]
fn test_parse_threshold_edge_cases() {
    // Test edge cases for threshold parsing
    // 1-of-1 (minimum)
    let result = parse_threshold("1-of-1");
    if result.is_ok() {
        let (threshold, total) = result.unwrap();
        assert_eq!(threshold, 1);
        assert_eq!(total, 1);
    }
    
    // 5-of-5 (all required)
    let result = parse_threshold("5-of-5");
    if result.is_ok() {
        let (threshold, total) = result.unwrap();
        assert_eq!(threshold, 5);
        assert_eq!(total, 5);
    }
}

// ============================================================================
// Phase 4: CLI Integration Tests
// ============================================================================

#[test]
fn test_cli_output_format_roundtrip() {
    // Test output format can be converted to string and back
    let format = OutputFormat::Text;
    // OutputFormat doesn't implement Display, but we can test parsing
    let parsed: Result<OutputFormat, _> = "text".parse();
    
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap(), format);
}

#[test]
fn test_cli_output_format_all_variants() {
    // Test all output format variants
    let formats = vec![OutputFormat::Text, OutputFormat::Json];
    
    for format in formats {
        let formatter = OutputFormatter::new(format);
        // Test by formatting something
        let result = formatter.format(&"test");
        assert!(result.is_ok());
    }
}

#[test]
fn test_cli_input_parsing_robustness() {
    // Test input parsing handles various edge cases
    // Empty values (filtered out)
    let result = parse_comma_separated("value1,,value3");
    assert_eq!(result.len(), 2); // Empty value is filtered
    assert_eq!(result[0], "value1");
    assert_eq!(result[1], "value3");
    
    // Only commas (all filtered out)
    let result = parse_comma_separated(",,,");
    assert_eq!(result.len(), 0); // All empty values filtered
}

#[test]
fn test_cli_threshold_validation() {
    // Test threshold validation logic
    // Threshold should be <= total
    let result = parse_threshold("3-of-5");
    assert!(result.is_ok());
    let (threshold, total) = result.unwrap();
    assert!(threshold <= total);
    
    // Threshold can be 0 (parsing succeeds, validation happens elsewhere)
    let result = parse_threshold("0-of-5");
    // Parsing succeeds (validation happens at usage time)
    assert!(result.is_ok());
    let (threshold, total) = result.unwrap();
    assert_eq!(threshold, 0);
    assert_eq!(total, 5);
}

// ============================================================================
// Phase 5: Error Handling Tests
// ============================================================================

#[test]
fn test_output_formatter_error_handling() {
    // Test formatter handles various error types
    let formatter = OutputFormatter::new(OutputFormat::Text);
    
    // IO error
    let io_error: Box<dyn Error> = Box::new(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "Permission denied"
    ));
    let formatted = formatter.format_error(&*io_error);
    assert!(!formatted.is_empty());
    
    // String error
    let string_error: Box<dyn Error> = "Test error".into();
    let formatted = formatter.format_error(&*string_error);
    assert!(!formatted.is_empty());
}

#[test]
fn test_input_parsing_error_messages() {
    // Test that parsing errors provide useful messages
    let result = parse_threshold("invalid-format");
    assert!(result.is_err());
    
    // Error should contain information
    let error_msg = format!("{}", result.unwrap_err());
    assert!(!error_msg.is_empty());
}

// ============================================================================
// Phase 6: Format Consistency Tests
// ============================================================================

#[test]
fn test_output_format_consistency() {
    // Test that same format produces consistent output
    let formatter1 = OutputFormatter::new(OutputFormat::Text);
    let formatter2 = OutputFormatter::new(OutputFormat::Text);
    
    let error: Box<dyn Error> = Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Test error"
    ));
    
    let formatted1 = formatter1.format_error(&*error);
    let formatted2 = formatter2.format_error(&*error);
    
    // Should produce same output for same format
    assert_eq!(formatted1, formatted2);
}

#[test]
fn test_output_format_differences() {
    // Test that different formats produce different output
    let text_formatter = OutputFormatter::new(OutputFormat::Text);
    let json_formatter = OutputFormatter::new(OutputFormat::Json);
    
    let error: Box<dyn Error> = Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Test error"
    ));
    
    let text_output = text_formatter.format_error(&*error);
    let json_output = json_formatter.format_error(&*error);
    
    // Should produce different output
    assert_ne!(text_output, json_output);
}

