#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetentDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateResult {
    Idle,
    Settling,
    Detent(DetentDirection),
}

/// Number of consecutive 100 us samples required to accept a new A level.
///
/// The encoder permits up to 3 ms of contact chatter. A transition therefore
/// becomes one click only after the candidate A level has remained unchanged
/// for the complete chatter interval. The edge sample plus 30 later samples
/// spans 3 ms.
const A_STABLE_SAMPLES: u8 = 31;

/// Debounces the encoder's specified A phase and emits once per stable edge.
///
/// This 9-pulse/18-detent encoder specifies the A level, but not the B level,
/// at a mechanical detent. Every stable rising or falling edge of A is one of
/// the 18 click intervals. B is used only at that A edge to determine the
/// quadrature direction; its arbitrary level at the detent is ignored.
///
/// Crucially, `candidate_b_high` is replaced whenever A changes again. The
/// direction therefore comes from the last A edge that survives contact
/// chatter, never from the first bouncing edge.
pub struct StableAEdgeDecoder {
    confirmed_a_high: bool,
    candidate_a_high: bool,
    candidate_b_high: bool,
    stable_samples: u8,
}

impl StableAEdgeDecoder {
    pub const fn new(initial_a_high: bool) -> Self {
        Self {
            confirmed_a_high: initial_a_high,
            candidate_a_high: initial_a_high,
            candidate_b_high: false,
            stable_samples: 0,
        }
    }

    pub const fn is_settling(&self) -> bool {
        self.candidate_a_high != self.confirmed_a_high
    }

    pub fn update(&mut self, a_high: bool, b_high: bool) -> UpdateResult {
        if a_high == self.confirmed_a_high {
            self.candidate_a_high = self.confirmed_a_high;
            self.stable_samples = 0;
            return UpdateResult::Idle;
        }

        if a_high != self.candidate_a_high {
            self.candidate_a_high = a_high;
            self.candidate_b_high = b_high;
            self.stable_samples = 1;
        } else {
            self.stable_samples = self.stable_samples.saturating_add(1);
        }

        if self.stable_samples < A_STABLE_SAMPLES {
            return UpdateResult::Settling;
        }

        self.confirmed_a_high = self.candidate_a_high;
        self.stable_samples = 0;
        UpdateResult::Detent(direction_from_a_edge(
            self.confirmed_a_high,
            self.candidate_b_high,
        ))
    }
}

/// Quadrature truth table evaluated only at a confirmed A edge.
pub const fn direction_from_a_edge(
    a_high_after_edge: bool,
    b_high_at_edge: bool,
) -> DetentDirection {
    if a_high_after_edge != b_high_at_edge {
        DetentDirection::Clockwise
    } else {
        DetentDirection::CounterClockwise
    }
}

#[cfg(test)]
mod tests {
    use super::{A_STABLE_SAMPLES, DetentDirection, StableAEdgeDecoder, UpdateResult};

    fn settle(decoder: &mut StableAEdgeDecoder, a_high: bool, b_high: bool) -> UpdateResult {
        let mut result = UpdateResult::Idle;
        for _ in 0..A_STABLE_SAMPLES {
            result = decoder.update(a_high, b_high);
        }
        result
    }

    #[test]
    fn each_a_edge_emits_one_click_in_the_same_clockwise_direction() {
        let mut decoder = StableAEdgeDecoder::new(false);
        for click in 0..18 {
            let a_high = click % 2 == 0;
            let b_high = !a_high;
            assert_eq!(
                settle(&mut decoder, a_high, b_high),
                UpdateResult::Detent(DetentDirection::Clockwise),
                "click {click}"
            );
        }
    }

    #[test]
    fn each_a_edge_emits_one_click_in_the_same_counterclockwise_direction() {
        let mut decoder = StableAEdgeDecoder::new(false);
        for click in 0..18 {
            let a_high = click % 2 == 0;
            let b_high = a_high;
            assert_eq!(
                settle(&mut decoder, a_high, b_high),
                UpdateResult::Detent(DetentDirection::CounterClockwise),
                "click {click}"
            );
        }
    }

    #[test]
    fn arbitrary_b_at_detents_never_emits_or_reverses_a_click() {
        let mut decoder = StableAEdgeDecoder::new(false);

        for b_high in [true, false, true, false, true] {
            assert_eq!(decoder.update(false, b_high), UpdateResult::Idle);
        }
        assert_eq!(
            settle(&mut decoder, true, false),
            UpdateResult::Detent(DetentDirection::Clockwise)
        );

        for b_high in [false, true, false, true, false] {
            assert_eq!(decoder.update(true, b_high), UpdateResult::Idle);
        }
        assert_eq!(
            settle(&mut decoder, false, true),
            UpdateResult::Detent(DetentDirection::Clockwise)
        );
    }

    #[test]
    fn first_bouncing_edge_cannot_fix_the_wrong_direction() {
        let mut decoder = StableAEdgeDecoder::new(false);

        // The first departure carries the opposite direction but bounces
        // back. It must not survive into the following real A edge.
        for _ in 0..8 {
            assert_eq!(decoder.update(true, true), UpdateResult::Settling);
        }
        assert_eq!(decoder.update(false, true), UpdateResult::Idle);

        assert_eq!(
            settle(&mut decoder, true, false),
            UpdateResult::Detent(DetentDirection::Clockwise)
        );
    }

    #[test]
    fn b_changes_after_the_a_edge_cannot_change_its_direction() {
        let mut decoder = StableAEdgeDecoder::new(false);

        assert_eq!(decoder.update(true, false), UpdateResult::Settling);
        let mut result = UpdateResult::Settling;
        for _ in 1..A_STABLE_SAMPLES {
            result = decoder.update(true, true);
        }
        assert_eq!(result, UpdateResult::Detent(DetentDirection::Clockwise));
    }

    #[test]
    fn incomplete_a_edge_never_emits() {
        let mut decoder = StableAEdgeDecoder::new(false);
        for _ in 1..A_STABLE_SAMPLES {
            assert_eq!(decoder.update(true, false), UpdateResult::Settling);
        }
        assert_eq!(decoder.update(false, false), UpdateResult::Idle);
        assert!(!decoder.is_settling());
    }

    #[test]
    fn reversal_uses_only_the_new_edge_direction() {
        let mut decoder = StableAEdgeDecoder::new(false);
        assert_eq!(
            settle(&mut decoder, true, false),
            UpdateResult::Detent(DetentDirection::Clockwise)
        );

        // Reversing crosses the same A edge in the opposite direction. No
        // previous-direction latch is involved.
        assert_eq!(
            settle(&mut decoder, false, false),
            UpdateResult::Detent(DetentDirection::CounterClockwise)
        );
    }
}
