//! Additional tests for CLI output module to improve coverage.

use blvm_sdk::cli::output::{OutputFormat, OutputFormatter};
use serde_json::json;

#[test]
fn test_output_format_from_str() {
    use std::str::FromStr;

    // Test parsing from string
    assert_eq!(OutputFormat::from_str("text").unwrap(), OutputFormat::Text);
    assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str("TEXT").unwrap(), OutputFormat::Text);
    assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str("txt").unwrap(), OutputFormat::Text);

    // Test invalid format
    assert!(OutputFormat::from_str("invalid").is_err());
    assert!(OutputFormat::from_str("").is_err());
}

#[test]
fn test_output_format_debug() {
    assert!(format!("{:?}", OutputFormat::Text).contains("Text"));
    assert!(format!("{:?}", OutputFormat::Json).contains("Json"));
}

#[test]
fn test_output_formatter_new() {
    let formatter = OutputFormatter::new(OutputFormat::Text);
    // Test that formatter was created successfully
    let test_string = "test".to_string();
    assert!(formatter.format(&test_string).is_ok());

    let formatter = OutputFormatter::new(OutputFormat::Json);
    // Test that formatter was created successfully
    assert!(formatter.format(&test_string).is_ok());
}

#[test]
fn test_output_formatter_format_text() {
    let formatter = OutputFormatter::new(OutputFormat::Text);

    // Test with string
    let result = formatter.format(&"test string");
    assert_eq!(result, Ok("test string".to_string()));

    // Test with number
    let result = formatter.format(&42);
    assert_eq!(result, Ok("42".to_string()));

    // Test with boolean
    let result = formatter.format(&true);
    assert_eq!(result, Ok("true".to_string()));

    // Test with empty string
    let result = formatter.format(&"");
    assert_eq!(result, Ok("".to_string()));
}

#[test]
fn test_output_formatter_format_json() {
    let formatter = OutputFormatter::new(OutputFormat::Json);

    // Test with simple data
    let data = json!({"key": "value"});
    let result = formatter.format(&data);
    assert!(result.is_ok());
    let json_str = result.unwrap();
    assert!(json_str.contains("\"key\""));
    assert!(json_str.contains("\"value\""));

    // Test with complex nested data
    let complex_data = json!({
        "nested": {
            "array": [1, 2, 3],
            "string": "test",
            "boolean": true,
            "null": null
        }
    });
    let result = formatter.format(&complex_data);
    assert!(result.is_ok());
    let json_str = result.unwrap();
    assert!(json_str.contains("\"nested\""));
    assert!(json_str.contains("\"array\""));
    assert!(json_str.contains("\"string\""));
    assert!(json_str.contains("\"boolean\""));
    assert!(json_str.contains("\"null\""));

    // Test with array
    let array_data = json!([1, 2, 3, "test"]);
    let result = formatter.format(&array_data);
    assert!(result.is_ok());
    let json_str = result.unwrap();
    assert!(json_str.starts_with("["));
    assert!(json_str.ends_with("]"));
}

#[test]
fn test_output_formatter_format_json_error() {
    let formatter = OutputFormatter::new(OutputFormat::Json);

    // Test with data that can't be serialized (this is tricky to create)
    // We'll test with a very large number that might cause issues
    let large_data = json!({
        "large_number": 1e308,
        "infinity": f64::INFINITY
    });
    let result = formatter.format(&large_data);
    // This should still work as JSON can handle these
    assert!(result.is_ok());
}

#[test]
fn test_output_formatter_format_edge_cases() {
    let text_formatter = OutputFormatter::new(OutputFormat::Text);
    let json_formatter = OutputFormatter::new(OutputFormat::Json);

    // Test with special characters
    let special_chars = "test\nwith\ttabs\r\nand\0null";
    let text_result = text_formatter.format(&special_chars);
    assert_eq!(text_result, Ok(special_chars.to_string()));

    let json_result = json_formatter.format(&special_chars);
    assert!(json_result.is_ok());
    let json_str = json_result.unwrap();
    assert!(json_str.contains("\\n"));
    assert!(json_str.contains("\\t"));
    assert!(json_str.contains("\\r"));

    // Test with unicode
    let unicode_str = "测试中文🚀";
    let text_result = text_formatter.format(&unicode_str);
    assert_eq!(text_result, Ok(unicode_str.to_string()));

    let json_result = json_formatter.format(&unicode_str);
    assert!(json_result.is_ok());
    let json_str = json_result.unwrap();
    assert!(json_str.contains("测试"));
    assert!(json_str.contains("🚀"));
}

#[test]
fn test_output_formatter_format_with_serde_json_value() {
    let formatter = OutputFormatter::new(OutputFormat::Json);

    // Test with serde_json::Value directly
    let value = serde_json::Value::String("test".to_string());
    let result = formatter.format(&value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "\"test\"");

    // Test with number value
    let value = serde_json::Value::Number(serde_json::Number::from(42));
    let result = formatter.format(&value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "42");

    // Test with boolean value
    let value = serde_json::Value::Bool(true);
    let result = formatter.format(&value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "true");

    // Test with null value
    let value = serde_json::Value::Null;
    let result = formatter.format(&value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "null");
}
