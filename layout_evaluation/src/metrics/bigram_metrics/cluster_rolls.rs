//! This metric takes into account how an intra-cluster roll feels, because (at
//! least for me):
//! - center -> south and pad -> up feel *great*
//! - center -> (in|out) feels decent
//! - a bunch of the other ones are *terrible*

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
    pub costs: AHashMap<Direction, AHashMap<Direction, f64>>,
    pub finger_multipliers: AHashMap<Finger, f64>,
    /// Minimum relative bigram frequency to apply heavy penalty (as fraction, e.g., 0.0004 = 0.04%)
    pub critical_bigram_fraction: Option<f64>,
    /// Multiplier for bigrams above critical_bigram_fraction (e.g., 100.0 = 100x penalty)
    pub critical_bigram_factor: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct ClusterRolls {
    default_cost: f64,
    ignore_thumb: bool,
    costs: AHashMap<Direction, AHashMap<Direction, f64>>,
    finger_multipliers: AHashMap<Finger, f64>,
    critical_bigram_fraction: Option<f64>,
    critical_bigram_factor: Option<f64>,
}

impl ClusterRolls {
    pub fn new(params: &Parameters) -> Self {
        Self {
            costs: params.costs.clone(),
            ignore_thumb: params.ignore_thumb,
            default_cost: params.default_cost,
            finger_multipliers: params.finger_multipliers.clone(),
            critical_bigram_fraction: params.critical_bigram_fraction,
            critical_bigram_factor: params.critical_bigram_factor,
        }
    }
}

impl BigramMetric for ClusterRolls {
    fn name(&self) -> &str {
        "Cluster Rolls"
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
        if (k1 == k2 && k1.is_modifier.is_some())
            || k1.key.hand != k2.key.hand
            || k1.key.finger != k2.key.finger
            || (self.ignore_thumb && k1.key.finger == Finger::Thumb)
        {
            return Some(0.0);
        }

        let finger = k1.key.finger; // same for k1, k2
        let dir_from = k1.key.direction;
        let dir_to = k2.key.direction;

        let base_cost = match self.costs.get(&dir_from) {
            Some(m) => match m.get(&dir_to) {
                Some(base_cost) => *base_cost,
                _ => self.default_cost,
            }
            _ => self.default_cost,
        };

        let finger_multiplier = match self.finger_multipliers.get(&finger) {
            Some(m) => *m,
            _ => 1.0,
        };

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
