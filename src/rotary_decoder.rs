#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetentDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingResult {
    InProgress,
    Cancelled,
    Detent(DetentDirection),
}

// The encoder specification permits up to 3 ms of contact chatter. The
// runtime samples every 100 us, so 30 consecutive samples confirms an A edge.
const A_STABLE_SAMPLES: u8 = 30;

/// Debounces one edge of phase A.
///
/// This encoder has 9 A pulses and 18 detents per revolution, so every
/// confirmed rising or falling A edge is exactly one click. Direction is
/// decoded from the level of B captured when that A edge first occurred:
///
/// CW:  A 0->1 with B=0, or A 1->0 with B=1
/// CCW: A 0->1 with B=1, or A 1->0 with B=0
pub struct AEdgeTracker {
    original_a_high: bool,
    b_high_at_edge: bool,
    original_samples: u8,
    changed_samples: u8,
    finished: bool,
}

impl AEdgeTracker {
    pub const fn new(original_a_high: bool, b_high_at_edge: bool) -> Self {
        Self {
            original_a_high,
            b_high_at_edge,
            original_samples: 0,
            changed_samples: 0,
            finished: false,
        }
    }

    pub fn sample(&mut self, a_high: bool) -> TrackingResult {
        if self.finished {
            return TrackingResult::Cancelled;
        }

        if a_high == self.original_a_high {
            self.changed_samples = 0;
            self.original_samples = self.original_samples.saturating_add(1);
            if self.original_samples >= A_STABLE_SAMPLES {
                self.finished = true;
                return TrackingResult::Cancelled;
            }
            return TrackingResult::InProgress;
        }

        self.original_samples = 0;
        self.changed_samples = self.changed_samples.saturating_add(1);
        if self.changed_samples < A_STABLE_SAMPLES {
            return TrackingResult::InProgress;
        }

        self.finished = true;
        TrackingResult::Detent(direction_from_a_edge(a_high, self.b_high_at_edge))
    }
}

/// Direction truth table from the encoder specification.
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
    use super::{AEdgeTracker, DetentDirection, TrackingResult, direction_from_a_edge};

    fn settle_changed(tracker: &mut AEdgeTracker, changed_a_high: bool) -> TrackingResult {
        let mut result = TrackingResult::InProgress;
        for _ in 0..30 {
            result = tracker.sample(changed_a_high);
        }
        result
    }

    #[test]
    fn implements_the_published_a_edge_b_level_truth_table() {
        assert_eq!(
            direction_from_a_edge(true, false),
            DetentDirection::Clockwise
        );
        assert_eq!(
            direction_from_a_edge(false, true),
            DetentDirection::Clockwise
        );
        assert_eq!(
            direction_from_a_edge(true, true),
            DetentDirection::CounterClockwise
        );
        assert_eq!(
            direction_from_a_edge(false, false),
            DetentDirection::CounterClockwise
        );
    }

    #[test]
    fn both_a_edges_in_each_pulse_are_clicks_in_the_same_direction() {
        // CW waveform: 00 -> 10 -> 11 -> 01 -> 00.
        assert_eq!(
            settle_changed(&mut AEdgeTracker::new(false, false), true),
            TrackingResult::Detent(DetentDirection::Clockwise)
        );
        assert_eq!(
            settle_changed(&mut AEdgeTracker::new(true, true), false),
            TrackingResult::Detent(DetentDirection::Clockwise)
        );

        // CCW waveform: 00 -> 01 -> 11 -> 10 -> 00.
        assert_eq!(
            settle_changed(&mut AEdgeTracker::new(false, true), true),
            TrackingResult::Detent(DetentDirection::CounterClockwise)
        );
        assert_eq!(
            settle_changed(&mut AEdgeTracker::new(true, false), false),
            TrackingResult::Detent(DetentDirection::CounterClockwise)
        );
    }

    #[test]
    fn contact_chatter_does_not_emit_an_extra_or_opposite_click() {
        let mut tracker = AEdgeTracker::new(false, false);
        for _ in 0..8 {
            assert_eq!(tracker.sample(true), TrackingResult::InProgress);
            assert_eq!(tracker.sample(false), TrackingResult::InProgress);
        }
        assert_eq!(
            settle_changed(&mut tracker, true),
            TrackingResult::Detent(DetentDirection::Clockwise)
        );
        assert_eq!(tracker.sample(false), TrackingResult::Cancelled);
    }

    #[test]
    fn an_a_edge_that_settles_back_at_its_original_level_is_cancelled() {
        let mut tracker = AEdgeTracker::new(false, false);
        for _ in 0..5 {
            assert_eq!(tracker.sample(true), TrackingResult::InProgress);
        }
        let mut result = TrackingResult::InProgress;
        for _ in 0..30 {
            result = tracker.sample(false);
        }
        assert_eq!(result, TrackingResult::Cancelled);
    }
}
