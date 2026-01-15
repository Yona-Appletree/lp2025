//! User-friendly message formatting helpers
//!
//! This module provides functions for formatting success and error messages
//! with actionable next steps and copy-pasteable commands.

/// Print success message with next steps
pub fn print_success(message: &str, next_steps: &[&str]) {
    println!("✓ {}", message);
    if !next_steps.is_empty() {
        println!("\nNext steps:");
        for step in next_steps {
            println!("  {}", step);
        }
    }
}

/// Print error message with suggestions
#[allow(dead_code)] // Will be used in phase 8
pub fn print_error(message: &str, suggestions: &[&str]) {
    eprintln!("✗ {}", message);
    if !suggestions.is_empty() {
        eprintln!();
        for suggestion in suggestions {
            eprintln!("  {}", suggestion);
        }
    }
}

/// Format command for copy-paste (with proper quoting if needed)
#[allow(dead_code)] // Will be used in phase 9
pub fn format_command(cmd: &str) -> String {
    // For now, just return as-is
    // In the future, we might want to add quoting for paths with spaces
    cmd.to_string()
}
