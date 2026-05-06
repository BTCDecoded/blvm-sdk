//! # CLI Output Formatting
//!
//! Output formatting utilities for CLI tools.

use serde::Serialize;
use std::fmt;

/// Output format options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid output format: {s}")),
        }
    }
}

/// Output formatter for CLI tools
pub struct OutputFormatter {
    format: OutputFormat,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Format a value for output
    pub fn format<T>(&self, value: &T) -> Result<String, String>
    where
        T: Serialize + fmt::Display,
    {
        match self.format {
            OutputFormat::Text => Ok(value.to_string()),
            OutputFormat::Json => serde_json::to_string_pretty(value)
                .map_err(|e| format!("JSON serialization error: {e}")),
        }
    }

    /// Format an error for output
    pub fn format_error(&self, error: &dyn std::error::Error) -> String {
        match self.format {
            OutputFormat::Text => format!("Error: {error}"),
            OutputFormat::Json => {
                let error_json = serde_json::json!({
                    "error": true,
                    "message": error.to_string()
                });
                serde_json::to_string_pretty(&error_json)
                    .unwrap_or_else(|_| format!("{{\"error\": true, \"message\": \"{error}\"}}"))
            }
        }
    }

    /// Format a success message
    pub fn format_success(&self, message: &str) -> String {
        match self.format {
            OutputFormat::Text => format!("Success: {message}"),
            OutputFormat::Json => {
                let success_json = serde_json::json!({
                    "success": true,
                    "message": message
                });
                serde_json::to_string_pretty(&success_json).unwrap_or_else(|_| {
                    format!("{{\"success\": true, \"message\": \"{message}\"}}")
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("txt".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_text_formatting() {
        let formatter = OutputFormatter::new(OutputFormat::Text);
        let result = formatter.format(&"test message");
        assert_eq!(result.unwrap(), "test message");
    }

    #[test]
    fn test_json_formatting() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let result = formatter.format(&serde_json::json!({"message": "test"}));
        assert!(result.unwrap().contains("test"));
    }
}
