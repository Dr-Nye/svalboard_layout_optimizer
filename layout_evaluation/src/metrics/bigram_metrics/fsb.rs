//! Key-cost-based full scissoring metric for adjacent finger movements.
//!
//! ## Core Principle
//!
//! Identifies uncomfortable "full scissor" motions where adjacent fingers have mismatched
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
//! **Full Scissor Vertical** - Opposite vertical directions (North ↔ South)
//! **Full Scissor Squeeze** - Fingers moving toward each other (In ↔ Out, inward motion - more uncomfortable)
//! **Full Scissor Splay** - Fingers moving apart (In ↔ Out, outward motion - less uncomfortable)
//!
//! ## Configuration
//!
//! All factors and frequency thresholds are configurable in the evaluation metrics:
//! - `vertical_factor`: Multiplier for vertical scissors
//! - `squeeze_factor`: Multiplier for squeeze motion
//! - `splay_factor`: Multiplier for splay motion
//! - `critical_bigram_fraction`: Frequency threshold for high-penalty bigrams (optional)
//! - `critical_bigram_factor`: Multiplier for high-frequency bigrams (optional)

use super::{
    scissor_base::{is_adjacent_fingers, ScissorCategory, ScissorCompute, ScissorMetric},
    BigramMetric,
};

use colored::Colorize;
use keyboard_layout::{
    key::Direction::*,
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FsbCategory {
    Vertical,
    Squeeze,
    Splay,
}

impl ScissorCategory for FsbCategory {
    fn display_order() -> &'static [Self] {
        &[
            FsbCategory::Vertical,
            FsbCategory::Squeeze,
            FsbCategory::Splay,
        ]
    }

    fn display_name(&self) -> String {
        match self {
            FsbCategory::Vertical => "Vertical".underline().to_string(),
            FsbCategory::Squeeze => "Squeeze".underline().to_string(),
            FsbCategory::Splay => "Splay".underline().to_string(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    /// Base cost factor for Vertical (North-South opposition)
    pub vertical_factor: f64,
    /// Base cost factor for Squeeze (fingers moving inward)
    pub squeeze_factor: f64,
    /// Base cost factor for Splay (fingers moving outward)
    pub splay_factor: f64,
    /// Minimum relative bigram frequency to apply heavy penalty (as fraction, e.g., 0.0004 = 0.04%)
    pub critical_bigram_fraction: Option<f64>,
    /// Multiplier for bigrams above critical_bigram_fraction (e.g., 100.0 = 100x penalty)
    pub critical_bigram_factor: Option<f64>,
}

#[derive(Clone, Debug)]
struct FsbCompute {
    vertical_factor: f64,
    squeeze_factor: f64,
    splay_factor: f64,
}

impl ScissorCompute<FsbCategory> for FsbCompute {
    fn compute_cost(&self, k1: &LayerKey, k2: &LayerKey, _layout: &Layout) -> Option<(f64, FsbCategory)> {
        if !is_adjacent_fingers(k1, k2) {
            return None;
        }

        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;
        let cost_from = k1.key.cost;
        let cost_to = k2.key.cost;
        let cost_diff = (cost_from - cost_to).abs();

        match (dir_from, dir_to) {
            // FSB: Full Scissor Vertical - North-South opposition
            (South, North) | (North, South) => {
                Some((self.vertical_factor * cost_diff, FsbCategory::Vertical))
            }

            // FSB: Full Scissor Lateral - In-Out opposition (squeeze/splay)
            (In, Out) | (Out, In) => {
                let finger_from = k1.key.finger;
                let finger_to = k2.key.finger;
                let inward_motion = finger_from.numeric_index() > finger_to.numeric_index();
                let is_squeeze = inward_motion ^ (dir_from == Out);

                let (factor, category) = if is_squeeze {
                    (self.squeeze_factor, FsbCategory::Squeeze)
                } else {
                    (self.splay_factor, FsbCategory::Splay)
                };

                Some((factor * cost_diff, category))
            }

            // All other combinations: not full scissors
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fsb {
    inner: ScissorMetric<FsbCategory, FsbCompute>,
}

impl Fsb {
    pub fn new(params: &Parameters) -> Self {
        let compute = FsbCompute {
            vertical_factor: params.vertical_factor,
            squeeze_factor: params.squeeze_factor,
            splay_factor: params.splay_factor,
        };

        Self {
            inner: ScissorMetric::new(
                "FSB",
                params.critical_bigram_fraction,
                params.critical_bigram_factor,
                compute,
            ),
        }
    }
}

impl BigramMetric for Fsb {
    fn name(&self) -> &str {
        self.inner.name()
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
        self.inner.individual_cost(k1, k2, weight, total_weight, layout)
    }

    fn total_cost(
        &self,
        bigrams: &[((&LayerKey, &LayerKey), f64)],
        total_weight: Option<f64>,
        layout: &Layout,
    ) -> (f64, Option<String>) {
        self.inner.total_cost(bigrams, total_weight, layout)
    }
}
