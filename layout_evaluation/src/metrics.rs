//! The `metrics` module provides traits for layout, unigram, bigram, and trigram metrics.

pub mod bigram_metrics;
pub mod format_utils;
pub mod layout_metrics;
pub mod trigram_metrics;
pub mod unigram_metrics;

/// Helper function to convert weight to percentage
///
/// This is used by stats metrics to calculate percentages from frequencies.
#[inline]
pub(crate) fn to_percentage(weight: f64, total: f64) -> f64 {
    if total > 0.0 {
        (weight / total) * 100.0
    } else {
        0.0
    }
}
