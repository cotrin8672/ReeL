#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetentDirection {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateResult {
    Idle,
    Settling,
    Resynchronized,
    Detent(DetentDirection),
}

/// P1_14 remained a reliable one-change-per-click signal on the device, but
/// sampling the other phase at that edge produced alternating directions.
/// QDEC, conversely, provided a reliable signed movement but its raw count is
/// not a detent count. This decoder deliberately keeps those jobs separate.
pub struct StableADirectionDecoder {
    confirmed_a_high: bool,
    candidate_a_high: bool,
    candidate_samples: u8,
    origin_samples: u8,
    movement: i32,
}

/// The encoder allows up to 3 ms of contact chatter. At a 100 us task period,
/// 31 consecutive samples span the initial sample plus 3 ms.
const A_STABLE_SAMPLES: u8 = 31;

impl StableADirectionDecoder {
    pub const fn new(initial_a_high: bool) -> Self {
        Self {
            confirmed_a_high: initial_a_high,
            candidate_a_high: initial_a_high,
            candidate_samples: 0,
            origin_samples: 0,
            movement: 0,
        }
    }

    pub fn update(&mut self, a_high: bool, qdec_delta: i16) -> UpdateResult {
        self.movement = self.movement.saturating_add(i32::from(qdec_delta));

        if a_high == self.confirmed_a_high {
            self.candidate_a_high = self.confirmed_a_high;
            self.candidate_samples = 0;
            self.origin_samples = self.origin_samples.saturating_add(1);

            // A departure that settles back at the accepted level was contact
            // chatter or an abandoned partial turn. Do not carry its residual
            // signed movement into the next real click.
            if self.origin_samples >= A_STABLE_SAMPLES {
                self.movement = 0;
            }
            return UpdateResult::Idle;
        }

        self.origin_samples = 0;
        if a_high != self.candidate_a_high {
            self.candidate_a_high = a_high;
            self.candidate_samples = 1;
        } else {
            self.candidate_samples = self.candidate_samples.saturating_add(1);
        }

        if self.candidate_samples < A_STABLE_SAMPLES {
            return UpdateResult::Settling;
        }

        self.confirmed_a_high = self.candidate_a_high;
        self.candidate_samples = 0;
        let movement = core::mem::replace(&mut self.movement, 0);

        if movement > 0 {
            UpdateResult::Detent(DetentDirection::Positive)
        } else if movement < 0 {
            UpdateResult::Detent(DetentDirection::Negative)
        } else {
            // A stable level change without a valid signed QDEC transition can
            // only be a startup/resynchronization case or a hardware double
            // transition. Accept the new baseline without inventing a click.
            UpdateResult::Resynchronized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{A_STABLE_SAMPLES, DetentDirection, StableADirectionDecoder, UpdateResult};

    fn settle(
        decoder: &mut StableADirectionDecoder,
        a_high: bool,
        first_delta: i16,
    ) -> UpdateResult {
        let mut result = decoder.update(a_high, first_delta);
        for _ in 1..A_STABLE_SAMPLES {
            result = decoder.update(a_high, 0);
        }
        result
    }

    #[test]
    fn qdec_magnitude_never_multiplies_one_a_click() {
        for delta in [1, 2, 3, 12] {
            let mut decoder = StableADirectionDecoder::new(false);
            assert_eq!(
                settle(&mut decoder, true, delta),
                UpdateResult::Detent(DetentDirection::Positive),
                "delta={delta}"
            );
            for _ in 0..A_STABLE_SAMPLES {
                assert_eq!(decoder.update(true, delta), UpdateResult::Idle);
            }
        }
    }

    #[test]
    fn eighteen_a_changes_emit_eighteen_identical_clicks() {
        for (delta, expected) in [
            (2, DetentDirection::Positive),
            (-2, DetentDirection::Negative),
        ] {
            let mut decoder = StableADirectionDecoder::new(false);
            let mut a_high = false;
            for click in 0..18 {
                a_high = !a_high;
                assert_eq!(
                    settle(&mut decoder, a_high, delta),
                    UpdateResult::Detent(expected),
                    "click={click}"
                );
            }
        }
    }

    #[test]
    fn qdec_chatter_cancels_without_alternating_the_direction() {
        let mut decoder = StableADirectionDecoder::new(false);
        assert_eq!(decoder.update(true, 1), UpdateResult::Settling);
        assert_eq!(decoder.update(true, -1), UpdateResult::Settling);
        assert_eq!(decoder.update(true, 1), UpdateResult::Settling);

        let mut result = UpdateResult::Settling;
        for _ in 3..A_STABLE_SAMPLES {
            result = decoder.update(true, 0);
        }
        assert_eq!(result, UpdateResult::Detent(DetentDirection::Positive));
    }

    #[test]
    fn first_click_after_reversal_uses_only_the_new_movement() {
        let mut decoder = StableADirectionDecoder::new(false);
        assert_eq!(
            settle(&mut decoder, true, 2),
            UpdateResult::Detent(DetentDirection::Positive)
        );
        assert_eq!(
            settle(&mut decoder, false, -2),
            UpdateResult::Detent(DetentDirection::Negative)
        );
    }

    #[test]
    fn an_incomplete_departure_cannot_leak_into_the_next_click() {
        let mut decoder = StableADirectionDecoder::new(false);
        for _ in 0..10 {
            assert_eq!(decoder.update(true, 1), UpdateResult::Settling);
            assert_eq!(decoder.update(false, -1), UpdateResult::Idle);
        }
        for _ in 0..A_STABLE_SAMPLES {
            assert_eq!(decoder.update(false, 0), UpdateResult::Idle);
        }

        assert_eq!(
            settle(&mut decoder, true, -2),
            UpdateResult::Detent(DetentDirection::Negative)
        );
    }

    #[test]
    fn stable_a_change_without_qdec_direction_only_resynchronizes() {
        let mut decoder = StableADirectionDecoder::new(false);
        assert_eq!(settle(&mut decoder, true, 0), UpdateResult::Resynchronized);
        assert_eq!(decoder.update(true, 1), UpdateResult::Idle);
    }
}
