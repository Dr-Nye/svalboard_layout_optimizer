//! Key-cost-based scissoring metric for adjacent finger movements.
//!
//! ## Core Principle
//!
//! Identifies uncomfortable "scissor" motions where adjacent fingers have mismatched
//! effort levels (e.g., weak finger doing hard work while strong finger gets easy work).
//! Penalties scale proportionally with the absolute cost difference between keys:
//!
//! ```
//! penalty = factor × |cost_from - cost_to|
//! ```
//!
//! Key costs are defined in the keyboard configuration (`key_costs` section) and represent
//! the difficulty of reaching each position. Factors are configured per movement type in
//! the evaluation metrics configuration.
//!
//! ## Movement Classification
//!
//! **Full Scissor** - Maximum anatomical conflict:
//! - **Vertical** (North ↔ South): Opposite vertical directions
//! - **Squeeze** (In ↔ Out, inward motion): Fingers moving toward each other (more uncomfortable)
//! - **Splay** (In ↔ Out, outward motion): Fingers moving apart (less uncomfortable)
//!
//! **Half Scissor** - Diagonal movements with reduced conflict:
//! - Lateral + Vertical: One finger moves laterally (In/Out), other vertically (North/South)
//!
//! **Lateral Stretch** - Lateral displacement:
//! - Lateral + Center: One finger moves laterally (In/Out), other presses Center
//!
//! **Not penalized**: Same lateral direction (In→In, Out→Out) are pure rolls.
//!
//! ## Configuration
//!
//! All factors and frequency thresholds are configurable in the evaluation metrics:
//! - `full_scissor_vertical_factor`: Multiplier for vertical scissors
//! - `full_scissor_squeeze_factor`: Multiplier for squeeze motion
//! - `full_scissor_splay_factor`: Multiplier for splay motion
//! - `half_scissor_factor`: Multiplier for diagonal movements
//! - `lateral_stretch_factor`: Multiplier for lateral+center
//! - `critical_bigram_fraction`: Frequency threshold for high-penalty bigrams (optional)
//! - `critical_bigram_factor`: Multiplier for high-frequency bigrams (optional)

use super::BigramMetric;

use keyboard_layout::{
    key::{Direction::*, Finger},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    /// Base cost factor for Full Scissor Vertical (North-South opposition)
    pub full_scissor_vertical_factor: f64,
    /// Base cost factor for Full Scissor Squeeze (fingers moving inward)
    pub full_scissor_squeeze_factor: f64,
    /// Base cost factor for Full Scissor Splay (fingers moving outward)
    pub full_scissor_splay_factor: f64,
    /// Base cost factor for Half Scissor (diagonal lateral+vertical)
    pub half_scissor_factor: f64,
    /// Base cost factor for Lateral Stretch (lateral+center)
    pub lateral_stretch_factor: f64,
    /// Minimum relative bigram frequency to apply heavy penalty (as fraction, e.g., 0.0004 = 0.04%)
    pub critical_bigram_fraction: Option<f64>,
    /// Multiplier for bigrams above critical_bigram_fraction (e.g., 100.0 = 100x penalty)
    pub critical_bigram_factor: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Scissors {
    full_scissor_vertical_factor: f64,
    full_scissor_squeeze_factor: f64,
    full_scissor_splay_factor: f64,
    half_scissor_factor: f64,
    lateral_stretch_factor: f64,
    critical_bigram_fraction: Option<f64>,
    critical_bigram_factor: Option<f64>,
}

impl Scissors {
    pub fn new(params: &Parameters) -> Self {
        Self {
            full_scissor_vertical_factor: params.full_scissor_vertical_factor,
            full_scissor_squeeze_factor: params.full_scissor_squeeze_factor,
            full_scissor_splay_factor: params.full_scissor_splay_factor,
            half_scissor_factor: params.half_scissor_factor,
            lateral_stretch_factor: params.lateral_stretch_factor,
            critical_bigram_fraction: params.critical_bigram_fraction,
            critical_bigram_factor: params.critical_bigram_factor,
        }
    }

    fn cost_difference_penalty(
        &self,
        cost_from: f64,
        cost_to: f64,
        base_factor: f64,
    ) -> Option<f64> {
        let cost_diff = (cost_from - cost_to).abs();

        Some(base_factor * cost_diff)
    }

    fn bigram_cost(&self, k1: &LayerKey, k2: &LayerKey, _layout: &Layout) -> Option<f64> {
        // Only adjacent non-thumb fingers
        if (k1 == k2 && k1.is_modifier.is_some())
            || k1.key.hand != k2.key.hand
            || k1.key.finger.distance(&k2.key.finger) != 1
            || k1.key.finger == Finger::Thumb
            || k2.key.finger == Finger::Thumb
        {
            return None;
        }

        let finger_from = k1.key.finger;
        let finger_to = k2.key.finger;
        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;
        let cost_from = k1.key.cost;
        let cost_to = k2.key.cost;

        match (dir_from, dir_to) {
            // NOT a scissor: just rolling (same lateral direction)
            (In, In) | (Out, Out) => None,

            // FSB: Full Scissor Vertical - North-South opposition
            (South, North) | (North, South) => {
                self.cost_difference_penalty(cost_from, cost_to, self.full_scissor_vertical_factor)
            }

            // FSB: Full Scissor Lateral - In-Out opposition (squeeze/splay)
            (In, Out) | (Out, In) => {
                let inward_motion = finger_from.numeric_index() > finger_to.numeric_index();
                let is_squeeze = inward_motion ^ (dir_from == Out);

                let factor = if is_squeeze {
                    self.full_scissor_squeeze_factor
                } else {
                    self.full_scissor_splay_factor
                };

                self.cost_difference_penalty(cost_from, cost_to, factor)
            }

            // HSB: Half Scissor - Diagonal movements (lateral + vertical)
            (In, North) | (Out, North) | (North, In) | (North, Out)
            | (In, South) | (Out, South) | (South, In) | (South, Out) => {
                self.cost_difference_penalty(cost_from, cost_to, self.half_scissor_factor)
            }

            // LSB: Lateral Stretch - Lateral displacement with center
            (In, Center) | (Out, Center) | (Center, In) | (Center, Out) => {
                self.cost_difference_penalty(cost_from, cost_to, self.lateral_stretch_factor)
            }

            // All other combinations: not considered scissors
            _ => None,
        }
    }
}

impl BigramMetric for Scissors {
    fn name(&self) -> &str {
        "Scissors"
    }

    #[inline(always)]
    fn individual_cost(
        &self,
        k1: &LayerKey,
        k2: &LayerKey,
        weight: f64,
        total_weight: f64,
        layout: &Layout,
    ) -> Option<f64> {
        match self.bigram_cost(k1, k2, layout) {
            Some(base_cost) => {
                // Apply frequency-based multiplier if configured
                let frequency_multiplier = if let (Some(threshold), Some(factor)) =
                    (self.critical_bigram_fraction, self.critical_bigram_factor)
                {
                    let relative_weight = weight / total_weight;
                    if relative_weight > threshold {
                        factor
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };

                Some(weight * base_cost * frequency_multiplier)
            }
            None => Some(0.0),
        }
    }
}
