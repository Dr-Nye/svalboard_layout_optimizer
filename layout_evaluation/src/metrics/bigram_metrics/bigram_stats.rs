//! Bigram statistics metric that tracks percentages of various bigram categories.
//! This is informational only and not used for optimization.

use super::{
    scissor_base::{classify_scissor, ScissorType},
    BigramMetric,
};

use colored::Colorize;
use keyboard_layout::{
    key::{Direction, Finger},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    pub ignore_thumbs: bool,
    pub ignore_modifiers: bool,
    /// List of SFB movements to ignore from the count (e.g., [[Center, South], [In, South]])
    #[serde(default = "default_ignore_movements")]
    pub ignore_movements: Vec<(Direction, Direction)>,
}

fn default_ignore_movements() -> Vec<(Direction, Direction)> {
    vec![]
}

#[derive(Clone, Debug)]
pub struct BigramStats {
    ignore_thumbs: bool,
    ignore_modifiers: bool,
    ignore_movements: Vec<(Direction, Direction)>,
}

/// Format a percentage with up to 2 meaningful decimal places (strips trailing zeros)
fn format_percentage(value: f64) -> String {
    format!("{:.2}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

impl BigramStats {
    pub fn new(params: &Parameters) -> Self {
        Self {
            ignore_thumbs: params.ignore_thumbs,
            ignore_modifiers: params.ignore_modifiers,
            ignore_movements: params.ignore_movements.clone(),
        }
    }

    fn should_ignore_key(&self, key: &LayerKey) -> bool {
        (self.ignore_thumbs && key.key.finger == Finger::Thumb)
            || (self.ignore_modifiers && key.is_modifier.is_some())
    }

    /// Check if this SFB movement should be ignored from statistics
    fn should_ignore_movement(&self, k1: &LayerKey, k2: &LayerKey) -> bool {
        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;

        self.ignore_movements.contains(&(dir_from, dir_to))
    }
}

impl BigramMetric for BigramStats {
    fn name(&self) -> &str {
        "Bigram Statistics"
    }

    fn total_cost(
        &self,
        bigrams: &[((&LayerKey, &LayerKey), f64)],
        total_weight: Option<f64>,
        _layout: &Layout,
    ) -> (f64, Option<String>) {
        let mut sfb_weight = 0.0;
        let mut full_vertical_weight = 0.0;
        let mut squeeze_weight = 0.0;
        let mut splay_weight = 0.0;
        let mut diagonal_weight = 0.0;
        let mut lateral_weight = 0.0;

        let total_weight = total_weight.unwrap_or_else(|| bigrams.iter().map(|(_, w)| w).sum());

        for ((k1, k2), weight) in bigrams {
            // Skip same-key repeats
            if k1 == k2 {
                continue;
            }

            // Skip ignored keys for all metrics
            if self.should_ignore_key(k1) || self.should_ignore_key(k2) {
                continue;
            }

            // Check for SFB
            if k1.key.hand == k2.key.hand && k1.key.finger == k2.key.finger {
                if !self.should_ignore_movement(k1, k2) {
                    sfb_weight += weight;
                }
            }

            // Check for scissor categories using shared classification function
            if let Some(scissor_type) = classify_scissor(k1, k2) {
                match scissor_type {
                    ScissorType::Vertical => full_vertical_weight += weight,
                    ScissorType::Squeeze => squeeze_weight += weight,
                    ScissorType::Splay => splay_weight += weight,
                    ScissorType::Diagonal => diagonal_weight += weight,
                    ScissorType::Lateral => lateral_weight += weight,
                }
            }
        }

        let sfb_percentage = crate::metrics::to_percentage(sfb_weight, total_weight);
        let full_vertical_percentage =
            crate::metrics::to_percentage(full_vertical_weight, total_weight);
        let squeeze_percentage = crate::metrics::to_percentage(squeeze_weight, total_weight);
        let splay_percentage = crate::metrics::to_percentage(splay_weight, total_weight);
        let diagonal_percentage = crate::metrics::to_percentage(diagonal_weight, total_weight);
        let lateral_percentage = crate::metrics::to_percentage(lateral_weight, total_weight);

        let message = format!(
            "{}: {}%, {}: {}%, {}: {}%, {}: {}%, {}: {}%, {}: {}%",
            "SFB".underline(),
            format_percentage(sfb_percentage),
            "Vertical".underline(),
            format_percentage(full_vertical_percentage),
            "Squeeze".underline(),
            format_percentage(squeeze_percentage),
            "Splay".underline(),
            format_percentage(splay_percentage),
            "Diagonal".underline(),
            format_percentage(diagonal_percentage),
            "Lateral".underline(),
            format_percentage(lateral_percentage)
        );

        // Return 0 cost since this is informational only
        (0.0, Some(message))
    }
}
