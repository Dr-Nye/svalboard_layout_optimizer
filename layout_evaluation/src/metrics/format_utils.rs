//! Utility functions for formatting metric output

use colored::Colorize;

/// Format cost and frequency percentages with dimmed color
///
/// Returns a formatted string like "27.5%|0.05%" with dimmed gray color
pub fn format_percentages(cost_percent: f64, freq_percent: f64) -> String {
    format!(
        "{:.1}%|{:.2}%",
        cost_percent,
        freq_percent
    ).truecolor(150, 150, 150)
    .to_string()
}

/// Replace whitespace characters with visible symbols for display
///
/// Replaces space with "␣" to make whitespace visible in output
pub fn visualize_whitespace(s: &str) -> String {
    s.replace(' ', "␣")
}
