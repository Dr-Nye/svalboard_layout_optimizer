//! SFB (Same Finger Bigram) metric.
//!
//! A same-finger bigram (SFB) occurs when consecutive keystrokes are typed with the same finger
//! on different keys.
//!
//! This implementation provides:
//! - Directional cost matrices: Different movement directions have different costs
//! - Per-finger multipliers: Some fingers handle SFBs better than others
//! - Critical bigram penalties: High-frequency SFBs can receive additional penalties
//! - Optional thumb exclusion: Thumbs can be excluded from SFB calculations
use super::BigramMetric;

use ahash::AHashMap;
use keyboard_layout::{
    key::{Direction, Finger},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    pub default_cost: f64,
    pub ignore_thumb: bool,
    pub exclude_modifiers: Option<bool>,
    pub costs: AHashMap<Direction, AHashMap<Direction, f64>>,
    pub finger_factors: AHashMap<Finger, f64>,
    /// Minimum relative bigram frequency to apply heavy penalty (as fraction, e.g., 0.0004 = 0.04%)
    pub critical_bigram_fraction: Option<f64>,
    /// Multiplier for bigrams above critical_bigram_fraction (e.g., 100.0 = 100x penalty)
    pub critical_bigram_factor: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Sfb {
    default_cost: f64,
    ignore_thumb: bool,
    exclude_modifiers: bool,
    costs: AHashMap<Direction, AHashMap<Direction, f64>>,
    finger_factors: AHashMap<Finger, f64>,
    critical_bigram_fraction: Option<f64>,
    critical_bigram_factor: Option<f64>,
}

impl Sfb {
    pub fn new(params: &Parameters) -> Self {
        Self {
            costs: params.costs.clone(),
            ignore_thumb: params.ignore_thumb,
            exclude_modifiers: params.exclude_modifiers.unwrap_or(false),
            default_cost: params.default_cost,
            finger_factors: params.finger_factors.clone(),
            critical_bigram_fraction: params.critical_bigram_fraction,
            critical_bigram_factor: params.critical_bigram_factor,
        }
    }
}

impl BigramMetric for Sfb {
    fn name(&self) -> &str {
        "SFB"
    }

    #[inline(always)]
    fn individual_cost(
        &self,
        k1: &LayerKey,
        k2: &LayerKey,
        weight: f64,
        total_weight: f64,
        _layout: &Layout,
    ) -> Option<f64> {
        // Skip modifiers if configured
        if self.exclude_modifiers && (k1.is_modifier.is_some() || k2.is_modifier.is_some()) {
            return Some(0.0);
        }

        // Skip same-key repeats (e.g., "ee" in "feed")
        if k1 == k2 {
            return Some(0.0);
        }

        // Different hands - not an SFB
        if k1.key.hand != k2.key.hand {
            return Some(0.0);
        }

        // Different fingers - not an SFB
        if k1.key.finger != k2.key.finger {
            return Some(0.0);
        }

        // Skip thumbs if configured
        if self.ignore_thumb && k1.key.finger == Finger::Thumb {
            return Some(0.0);
        }

        let finger = k1.key.finger; // same for k1, k2
        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;

        let base_cost = self
            .costs
            .get(&dir_from)
            .and_then(|m| m.get(&dir_to))
            .copied()
            .unwrap_or(self.default_cost);

        let finger_multiplier = self.finger_factors.get(&finger).copied().unwrap_or(1.0);

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

        let cost = weight * base_cost * finger_multiplier * frequency_multiplier;

        Some(cost)
    }
}
