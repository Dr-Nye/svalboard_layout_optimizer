//! Key-cost-based half scissoring metric for adjacent finger movements.
//!
//! ## Core Principle
//!
//! Identifies uncomfortable diagonal and lateral motions where adjacent fingers have mismatched
//! effort levels (e.g., weak finger doing hard work while strong finger gets easy work).
//! Penalties scale proportionally with the absolute cost difference between keys:
//!
//! ```
//! penalty = scissor_factor × |cost_from - cost_to| × finger_factor × freq_multiplier
//! ```
//!
//! Where:
//! - `scissor_factor`: Movement-type factor (diagonal/lateral)
//! - `finger_factor`: Max of the two fingers' factors (weaker finger dominates)
//! - `freq_multiplier`: Optional high-frequency bigram penalty
//!
//! Key costs are defined in the keyboard configuration (`key_costs` section) and represent
//! the difficulty of reaching each position. Factors are configured per movement type in
//! the evaluation metrics configuration.
//!
//! ## Movement Classification
//!
//! **Diagonal** - Diagonal movements with reduced conflict:
//! - Lateral + Vertical: One finger moves laterally (In/Out), other vertically (North/South)
//!
//! **Lateral** - Lateral displacement:
//! - Lateral + Center: One finger moves laterally (In/Out), other presses Center
//!
//! ## Configuration
//!
//! All factors and frequency thresholds are configurable in the evaluation metrics:
//! - `diagonal_factor`: Multiplier for diagonal movements
//! - `lateral_factor`: Multiplier for lateral+center
//! - `finger_factors`: Per-finger multipliers (e.g., pinky scissors are worse than index)
//! - `critical_bigram_fraction`: Frequency threshold for high-penalty bigrams (optional)
//! - `critical_bigram_factor`: Multiplier for high-frequency bigrams (optional)

use super::{
    scissor_base::{is_adjacent_fingers, ScissorCategory, ScissorCompute, ScissorMetric},
    BigramMetric,
};

use ahash::AHashMap;
use colored::Colorize;
use keyboard_layout::{
    key::{Direction::*, Finger},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HsbCategory {
    Diagonal,
    Lateral,
}

impl ScissorCategory for HsbCategory {
    fn display_order() -> &'static [Self] {
        &[HsbCategory::Diagonal, HsbCategory::Lateral]
    }

    fn display_name(&self) -> String {
        match self {
            HsbCategory::Diagonal => "Diagonal".underline().to_string(),
            HsbCategory::Lateral => "Lateral".underline().to_string(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    /// Base cost factor for diagonal (lateral+vertical)
    pub diagonal_factor: f64,
    /// Base cost factor for Lateral (lateral+center)
    pub lateral_factor: f64,
    /// Per-finger multipliers (e.g., pinky: 1.5, index: 0.75)
    pub finger_factors: Option<AHashMap<Finger, f64>>,
    /// Minimum relative bigram frequency to apply heavy penalty (as fraction, e.g., 0.0004 = 0.04%)
    pub critical_bigram_fraction: Option<f64>,
    /// Multiplier for bigrams above critical_bigram_fraction (e.g., 100.0 = 100x penalty)
    pub critical_bigram_factor: Option<f64>,
}

#[derive(Clone, Debug)]
struct HsbCompute {
    diagonal_factor: f64,
    lateral_factor: f64,
}

impl ScissorCompute<HsbCategory> for HsbCompute {
    fn compute_cost(
        &self,
        k1: &LayerKey,
        k2: &LayerKey,
        _layout: &Layout,
    ) -> Option<(f64, HsbCategory)> {
        if !is_adjacent_fingers(k1, k2) {
            return None;
        }

        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;
        let cost_from = k1.key.cost;
        let cost_to = k2.key.cost;
        let cost_diff = (cost_from - cost_to).abs();

        match (dir_from, dir_to) {
            // HSB: Half Scissor - Diagonal movements (lateral + vertical)
            (In, North)
            | (Out, North)
            | (North, In)
            | (North, Out)
            | (In, South)
            | (Out, South)
            | (South, In)
            | (South, Out) => Some((self.diagonal_factor * cost_diff, HsbCategory::Diagonal)),

            // Lateral - Lateral displacement with center
            (In, Center) | (Out, Center) | (Center, In) | (Center, Out) => {
                Some((self.lateral_factor * cost_diff, HsbCategory::Lateral))
            }

            // All other combinations: not considered half scissors or lateral
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hsb {
    inner: ScissorMetric<HsbCategory, HsbCompute>,
}

impl Hsb {
    pub fn new(params: &Parameters) -> Self {
        let compute = HsbCompute {
            diagonal_factor: params.diagonal_factor,
            lateral_factor: params.lateral_factor,
        };

        Self {
            inner: ScissorMetric::new(
                "HSB",
                params.critical_bigram_fraction,
                params.critical_bigram_factor,
                params.finger_factors.clone(),
                compute,
            ),
        }
    }
}

impl BigramMetric for Hsb {
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
        self.inner
            .individual_cost(k1, k2, weight, total_weight, layout)
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
