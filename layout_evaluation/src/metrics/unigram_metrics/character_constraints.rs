//! The unigram metric [`CharacterConstraints`] penalizes specific characters placed on
//! specific matrix positions with a configurable cost. This is useful for preventing
//! certain characters from being placed on difficult-to-reach keys.

use super::UnigramMetric;

use keyboard_layout::layout::{LayerKey, Layout};

use ahash::AHashMap;
use serde::Deserialize;

/// A tuple representing matrix position: (Column, Row)
type MatrixPosition = (u8, u8);

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    /// Mapping of characters to matrix positions and their costs
    pub costs: AHashMap<char, AHashMap<MatrixPosition, f64>>,
}

#[derive(Clone, Debug)]
pub struct CharacterConstraints {
    costs: AHashMap<char, AHashMap<MatrixPosition, f64>>,
}

impl CharacterConstraints {
    pub fn new(params: &Parameters) -> Self {
        Self {
            costs: params.costs.clone(),
        }
    }
}

impl UnigramMetric for CharacterConstraints {
    fn name(&self) -> &str {
        "Character Constraints"
    }

    #[inline(always)]
    fn individual_cost(
        &self,
        key: &LayerKey,
        weight: f64,
        _total_weight: f64,
        _layout: &Layout,
    ) -> Option<f64> {
        let symbol = key.symbol;

        if let Some(cost_map) = self.costs.get(&symbol) {
            let matrix_pos = (key.key.matrix_position.0, key.key.matrix_position.1);

            if let Some(cost) = cost_map.get(&matrix_pos) {
                log::trace!(
                    "Character Constraint: Symbol '{}' at position {:?}, Weight: {:>12.2}, Cost: {:>8.4}, Total: {:>14.4}",
                    symbol, matrix_pos, weight, cost, weight * cost
                );
                return Some(weight * cost);
            }
        }

        Some(0.0)
    }
}
