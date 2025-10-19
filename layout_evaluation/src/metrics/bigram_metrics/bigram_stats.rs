//! Bigram statistics metric that tracks percentages of various bigram categories.
//! This is informational only and not used for optimization.

use super::BigramMetric;

use colored::Colorize;
use keyboard_layout::{
    key::{Direction::*, Finger},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    pub ignore_thumb: bool,
    pub ignore_modifiers: bool,
    /// Exclude "good" SFBs like Center→South from the count (default: true)
    #[serde(default = "default_exclude_center_south")]
    pub exclude_center_south: bool,
}

fn default_exclude_center_south() -> bool {
    true
}

#[derive(Clone, Debug)]
pub struct BigramStats {
    ignore_thumb: bool,
    ignore_modifiers: bool,
    exclude_center_south: bool,
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
            ignore_thumb: params.ignore_thumb,
            ignore_modifiers: params.ignore_modifiers,
            exclude_center_south: params.exclude_center_south,
        }
    }

    fn should_ignore_key(&self, key: &LayerKey) -> bool {
        (self.ignore_thumb && key.key.finger == Finger::Thumb)
            || (self.ignore_modifiers && key.is_modifier.is_some())
    }

    /// Check if this is a Center→South same-finger movement (the "good" SFB)
    fn is_center_south_sfb(&self, k1: &LayerKey, k2: &LayerKey) -> bool {
        k1.key.matrix_position.0 == k2.key.matrix_position.0
            && k1.key.matrix_position.1 == 2 // center row
            && k2.key.matrix_position.1 == 3 // south row
    }

    /// Classify a bigram into scissor categories
    /// Returns: (is_full_vertical, is_squeeze, is_splay, is_half_scissor, is_lateral)
    fn classify_scissor(
        &self,
        k1: &LayerKey,
        k2: &LayerKey,
    ) -> (bool, bool, bool, bool, bool) {
        // Only adjacent non-thumb fingers
        if k1.key.hand != k2.key.hand
            || k1.key.finger.distance(&k2.key.finger) != 1
            || k1.key.finger == Finger::Thumb
            || k2.key.finger == Finger::Thumb
        {
            return (false, false, false, false, false);
        }

        let finger_from = k1.key.finger;
        let finger_to = k2.key.finger;
        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;

        match (dir_from, dir_to) {
            // NOT a scissor: just rolling (same lateral direction)
            (In, In) | (Out, Out) => (false, false, false, false, false),

            // Full Scissor Vertical - North-South opposition
            (South, North) | (North, South) => (true, false, false, false, false),

            // Full Scissor Lateral - In-Out opposition (squeeze/splay)
            (In, Out) | (Out, In) => {
                let inward_motion = finger_from.numeric_index() > finger_to.numeric_index();
                let is_squeeze = inward_motion ^ (dir_from == Out);

                if is_squeeze {
                    (false, true, false, false, false)
                } else {
                    (false, false, true, false, false)
                }
            }

            // Half Scissor - Diagonal movements (lateral + vertical)
            (In, North)
            | (Out, North)
            | (North, In)
            | (North, Out)
            | (In, South)
            | (Out, South)
            | (South, In)
            | (South, Out) => (false, false, false, true, false),

            // Lateral - Lateral displacement with center
            (In, Center) | (Out, Center) | (Center, In) | (Center, Out) => {
                (false, false, false, false, true)
            }

            // All other combinations: not considered scissors
            _ => (false, false, false, false, false),
        }
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
        let mut half_scissor_weight = 0.0;
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
                // Exclude "good" Center→South SFBs if configured
                if !self.exclude_center_south || !self.is_center_south_sfb(k1, k2) {
                    sfb_weight += weight;
                }
            }

            // Check for scissor categories
            let (is_full_vertical, is_squeeze, is_splay, is_half_scissor, is_lateral) =
                self.classify_scissor(k1, k2);

            if is_full_vertical {
                full_vertical_weight += weight;
            } else if is_squeeze {
                squeeze_weight += weight;
            } else if is_splay {
                splay_weight += weight;
            } else if is_half_scissor {
                half_scissor_weight += weight;
            } else if is_lateral {
                lateral_weight += weight;
            }
        }

        let sfb_percentage = crate::metrics::to_percentage(sfb_weight, total_weight);
        let full_vertical_percentage = crate::metrics::to_percentage(full_vertical_weight, total_weight);
        let squeeze_percentage = crate::metrics::to_percentage(squeeze_weight, total_weight);
        let splay_percentage = crate::metrics::to_percentage(splay_weight, total_weight);
        let half_scissor_percentage = crate::metrics::to_percentage(half_scissor_weight, total_weight);
        let lateral_percentage = crate::metrics::to_percentage(lateral_weight, total_weight);

        let message = format!(
            "{}: {}%, {}: {}%, {}: {}%, {}: {}%, {}: {}%, {}: {}%",
            "SFB".underline(),
            format_percentage(sfb_percentage),
            "Full Vertical".underline(),
            format_percentage(full_vertical_percentage),
            "Squeeze".underline(),
            format_percentage(squeeze_percentage),
            "Splay".underline(),
            format_percentage(splay_percentage),
            "Half".underline(),
            format_percentage(half_scissor_percentage),
            "Lateral".underline(),
            format_percentage(lateral_percentage)
        );

        // Return 0 cost since this is informational only
        (0.0, Some(message))
    }
}
