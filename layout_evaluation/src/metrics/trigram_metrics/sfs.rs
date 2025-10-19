//! SFS (Same Finger Skipgram) metric that evaluates skipgrams (k1_k3 patterns).
//! A skipgram is a sequence of two keystrokes separated by one keystroke.
//! For example, in "mouse", m_u, o_s, and u_e are skipgrams.

use super::TrigramMetric;

use ahash::AHashMap;
use keyboard_layout::{
    key::Finger,
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    pub ignore_thumb: bool,
    pub ignore_modifiers: bool,
    pub finger_factors: AHashMap<Finger, f64>,
}

#[derive(Clone, Debug)]
pub struct Sfs {
    ignore_thumb: bool,
    ignore_modifiers: bool,
    finger_factors: AHashMap<Finger, f64>,
}

impl Sfs {
    pub fn new(params: &Parameters) -> Self {
        Self {
            ignore_thumb: params.ignore_thumb,
            ignore_modifiers: params.ignore_modifiers,
            finger_factors: params.finger_factors.clone(),
        }
    }
}

impl TrigramMetric for Sfs {
    fn name(&self) -> &str {
        "SFS"
    }

    #[inline(always)]
    fn individual_cost(
        &self,
        k1: &LayerKey,
        _k2: &LayerKey,
        k3: &LayerKey,
        weight: f64,
        _total_weight: f64,
        _layout: &Layout,
    ) -> Option<f64> {
        // Skip modifiers if configured
        if self.ignore_modifiers && (k1.is_modifier.is_some() || k3.is_modifier.is_some()) {
            return Some(0.0);
        }

        // Skip same-key repeats (e.g., holding a modifier)
        if k1 == k3 {
            return Some(0.0);
        }

        // Different hands - not an SFS
        if k1.key.hand != k3.key.hand {
            return Some(0.0);
        }

        // Different fingers - not an SFS
        if k1.key.finger != k3.key.finger {
            return Some(0.0);
        }

        // Skip thumbs if configured
        if self.ignore_thumb && k1.key.finger == Finger::Thumb {
            return Some(0.0);
        }

        let finger = k1.key.finger;
        let finger_multiplier = self.finger_factors.get(&finger).copied().unwrap_or(1.0);
        let cost = weight * finger_multiplier;

        Some(cost)
    }
}
