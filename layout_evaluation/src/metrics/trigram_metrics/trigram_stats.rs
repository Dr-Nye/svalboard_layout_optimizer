use super::TrigramMetric;

use keyboard_layout::{
    key::{Finger, Hand},
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {
    pub ignore_modifiers: bool,
    pub ignore_thumbs: bool,
}

#[derive(Clone, Debug)]
pub struct TrigramStats {
    ignore_modifiers: bool,
    ignore_thumbs: bool,
}

impl TrigramStats {
    pub fn new(params: &Parameters) -> Self {
        Self {
            ignore_modifiers: params.ignore_modifiers,
            ignore_thumbs: params.ignore_thumbs,
        }
    }

    fn should_ignore_key(&self, key: &LayerKey) -> bool {
        (self.ignore_thumbs && key.key.finger == Finger::Thumb)
            || (self.ignore_modifiers && key.is_modifier.is_some())
    }

    /// Classify a trigram roll into its category
    /// Returns: (is_inward, is_outward, is_center_south)
    fn classify_roll(&self, k1: &LayerKey, k2: &LayerKey, k3: &LayerKey) -> (bool, bool, bool) {
        let h1 = k1.key.hand;
        let h2 = k2.key.hand;
        let h3 = k3.key.hand;

        let first_roll = h1 == h2 && h2 != h3;
        let second_roll = h1 != h2 && h2 == h3;

        if !(first_roll || second_roll) {
            return (false, false, false);
        }

        let (kr1, kr2) = if first_roll { (k1, k2) } else { (k2, k3) };

        // Same-finger vertical roll (center->south only)
        if kr1.key.finger == kr2.key.finger {
            let is_center_south = kr1.key.matrix_position.0 == kr2.key.matrix_position.0
                && kr1.key.matrix_position.1 == 2 // center row
                && kr2.key.matrix_position.1 == 3; // south row
            return (false, false, is_center_south);
        }

        // Different fingers: check inward vs outward
        let inwards = match kr1.key.hand {
            Hand::Left => kr1.key.matrix_position.0 < kr2.key.matrix_position.0,
            Hand::Right => kr1.key.matrix_position.0 > kr2.key.matrix_position.0,
        };

        if inwards {
            (true, false, false)
        } else {
            (false, true, false)
        }
    }
}

#[inline(always)]
fn inwards(k1: &LayerKey, k2: &LayerKey) -> bool {
    if k1.key.hand == Hand::Left {
        k1.key.matrix_position.0 < k2.key.matrix_position.0
    } else {
        k1.key.matrix_position.0 > k2.key.matrix_position.0
    }
}

/// Check if a trigram is a same-hand roll (all 3 keys on same hand, different fingers, directional)
/// Returns: (is_roll_in, is_roll_out)
fn classify_same_hand_roll(k1: &LayerKey, k2: &LayerKey, k3: &LayerKey) -> (bool, bool) {
    let h1 = k1.key.hand;
    let h2 = k2.key.hand;
    let h3 = k3.key.hand;

    // Must be same hand (one-handed trigram)
    if !(h1 == h2 && h2 == h3) {
        return (false, false);
    }

    let f1 = k1.key.finger;
    let f2 = k2.key.finger;
    let f3 = k3.key.finger;

    // Must use different fingers (no same-finger bigrams)
    if f1 == f2 || f2 == f3 {
        return (false, false);
    }

    // Check if all three movements are in the same direction
    let inwards1 = inwards(k1, k2);
    let inwards2 = inwards(k2, k3);

    let outwards1 = inwards(k2, k1);
    let outwards2 = inwards(k3, k2);

    // Roll in: both movements inward
    if inwards1 && inwards2 {
        return (true, false);
    }

    // Roll out: both movements outward
    if outwards1 && outwards2 {
        return (false, true);
    }

    (false, false)
}

/// Check if a trigram is a redirect: one-handed with direction change
/// Returns: (is_redirect, is_weak_redirect)
fn classify_redirect(k1: &LayerKey, k2: &LayerKey, k3: &LayerKey) -> (bool, bool) {
    let h1 = k1.key.hand;
    let h2 = k2.key.hand;
    let h3 = k3.key.hand;

    // Must be same hand (one-handed trigram)
    if !(h1 == h2 && h2 == h3) {
        return (false, false);
    }

    let f1 = k1.key.finger;
    let f2 = k2.key.finger;
    let f3 = k3.key.finger;

    // Must use different fingers (no same-finger bigrams)
    if f1 == f2 || f2 == f3 {
        return (false, false);
    }

    let inwards1 = inwards(k1, k2);
    let inwards2 = inwards(k2, k3);

    let outwards1 = inwards(k2, k1);
    let outwards2 = inwards(k3, k2);

    // Check for direction change: inward->outward or outward->inward
    let is_redirect = (inwards1 && outwards2) || (outwards1 && inwards2);

    if !is_redirect {
        return (false, false);
    }

    // Check if it's weak (no index finger)
    let has_index = f1 == Finger::Index || f2 == Finger::Index || f3 == Finger::Index;
    let is_weak = !has_index;

    (true, is_weak)
}

impl TrigramMetric for TrigramStats {
    fn name(&self) -> &str {
        "Trigram Statistics"
    }

    fn total_cost(
        &self,
        trigrams: &[((&LayerKey, &LayerKey, &LayerKey), f64)],
        _total_weight: Option<f64>,
        _layout: &Layout,
    ) -> (f64, Option<String>) {
        let mut bigram_inward_rolls_weight = 0.0;
        let mut bigram_outward_rolls_weight = 0.0;
        let mut center_south_rolls_weight = 0.0;
        let mut roll_in_weight = 0.0;
        let mut roll_out_weight = 0.0;
        let mut alternation_weight = 0.0;
        let mut redirects_weight = 0.0;
        let mut weak_redirects_weight = 0.0;
        let mut valid_trigrams_weight = 0.0;

        for ((k1, k2, k3), weight) in trigrams {
            // Skip ignored keys
            if self.should_ignore_key(k1)
                || self.should_ignore_key(k2)
                || self.should_ignore_key(k3)
            {
                continue;
            }

            valid_trigrams_weight += weight;

            let h1 = k1.key.hand;
            let h2 = k2.key.hand;
            let h3 = k3.key.hand;

            if h1 == h2 && h2 == h3 {
                // Same hand (all 3 keys) - check roll in/out or redirect
                let (is_roll_in, is_roll_out) = classify_same_hand_roll(k1, k2, k3);

                if is_roll_in {
                    roll_in_weight += weight;
                } else if is_roll_out {
                    roll_out_weight += weight;
                } else {
                    // Not a roll, check for redirect
                    let (is_redirect, is_weak) = classify_redirect(k1, k2, k3);
                    if is_redirect {
                        redirects_weight += weight;
                        if is_weak {
                            weak_redirects_weight += weight;
                        }
                    }
                }
            } else if h1 == h3 && h1 != h2 {
                // Alternation (LRL or RLR)
                alternation_weight += weight;
            } else {
                // Bigram pattern (2,1 or 1,2) - check bigram rolls
                let (is_inward, is_outward, is_center_south) = self.classify_roll(k1, k2, k3);

                if is_inward {
                    bigram_inward_rolls_weight += weight;
                } else if is_outward {
                    bigram_outward_rolls_weight += weight;
                } else if is_center_south {
                    center_south_rolls_weight += weight;
                }
            }
        }

        let bigram_inward_percentage = if valid_trigrams_weight > 0.0 {
            (bigram_inward_rolls_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let bigram_outward_percentage = if valid_trigrams_weight > 0.0 {
            (bigram_outward_rolls_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let center_south_percentage = if valid_trigrams_weight > 0.0 {
            (center_south_rolls_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let roll_in_percentage = if valid_trigrams_weight > 0.0 {
            (roll_in_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let roll_out_percentage = if valid_trigrams_weight > 0.0 {
            (roll_out_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let alternation_percentage = if valid_trigrams_weight > 0.0 {
            (alternation_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let redirect_percentage = if valid_trigrams_weight > 0.0 {
            (redirects_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let weak_redirect_percentage = if valid_trigrams_weight > 0.0 {
            (weak_redirects_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let total_bigram_rolls_weight = bigram_inward_rolls_weight + bigram_outward_rolls_weight + center_south_rolls_weight;
        let total_bigram_rolls_percentage = if valid_trigrams_weight > 0.0 {
            (total_bigram_rolls_weight / valid_trigrams_weight) * 100.0
        } else {
            0.0
        };

        let message = format!(
            "Total bigram roll: {:.1}%, Bigram roll in: {:.1}%, Bigram roll out: {:.1}%, Center->South: {:.1}%, Roll in: {:.1}%, Roll out: {:.1}%, Alt: {:.1}%, Redirect: {:.1}%, Weak redirect: {:.1}%",
            total_bigram_rolls_percentage,
            bigram_inward_percentage, bigram_outward_percentage, center_south_percentage,
            roll_in_percentage, roll_out_percentage, alternation_percentage,
            redirect_percentage, weak_redirect_percentage
        );

        // Return 0 cost since this is informational only
        (0.0, Some(message))
    }
}
