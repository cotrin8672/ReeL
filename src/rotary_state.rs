#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderDirection {
    Clockwise,
    CounterClockwise,
}

/// Tracks only debounced phase-A state.
///
/// One stable A transition is one mechanical click on the encoder fitted to
/// ReeL. Phase B is captured at the first A edge, before debounce can erase
/// the phase relationship used to determine direction.
pub struct RotaryState {
    stable_a_low: bool,
}

impl RotaryState {
    pub const fn new(a_low: bool) -> Self {
        Self {
            stable_a_low: a_low,
        }
    }

    pub fn settle(
        &mut self,
        b_low_at_a_edge: bool,
        settled_a_low: bool,
    ) -> Option<EncoderDirection> {
        if settled_a_low == self.stable_a_low {
            return None;
        }

        self.stable_a_low = settled_a_low;
        Some(if settled_a_low != b_low_at_a_edge {
            EncoderDirection::Clockwise
        } else {
            EncoderDirection::CounterClockwise
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EncoderDirection, RotaryState};

    #[test]
    fn emits_once_for_every_stable_a_transition() {
        let mut state = RotaryState::new(false);

        assert_eq!(state.settle(false, true), Some(EncoderDirection::Clockwise));
        assert_eq!(state.settle(true, false), Some(EncoderDirection::Clockwise));
        assert_eq!(state.settle(false, true), Some(EncoderDirection::Clockwise));
        assert_eq!(state.settle(true, false), Some(EncoderDirection::Clockwise));
    }

    #[test]
    fn rejects_bounce_that_returns_to_the_previous_state() {
        let mut state = RotaryState::new(false);

        assert_eq!(state.settle(true, false), None);
        assert_eq!(state.settle(false, true), Some(EncoderDirection::Clockwise));
    }

    #[test]
    fn phase_b_selects_a_consistent_direction() {
        let mut clockwise = RotaryState::new(false);
        let mut counter_clockwise = RotaryState::new(false);

        assert_eq!(
            clockwise.settle(false, true),
            Some(EncoderDirection::Clockwise)
        );
        assert_eq!(
            counter_clockwise.settle(true, true),
            Some(EncoderDirection::CounterClockwise)
        );
    }
}
