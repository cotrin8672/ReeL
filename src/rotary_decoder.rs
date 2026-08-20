#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderPhase {
    A,
    B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetentDirection {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingResult {
    InProgress,
    Cancelled,
    Detent(DetentDirection),
}

const INTERMEDIATE_STABLE_SAMPLES: u8 = 2;
const DETENT_STABLE_SAMPLES: u8 = 10;

/// Tracks one half-pulse of a 9-pulse/18-click encoder.
///
/// Direction is determined by the first contact that leaves the current
/// detent. Later contact bounce cannot reverse that decision. A click is only
/// emitted after the opposite detent (`00` or `11`) remains stable.
pub struct DetentTracker {
    origin: u8,
    wake_phase: Option<EncoderPhase>,
    leader: Option<EncoderPhase>,
    intermediate: Option<EncoderPhase>,
    intermediate_samples: u8,
    origin_samples: u8,
    target_samples: u8,
}

impl DetentTracker {
    pub const fn new(origin: u8, wake_phase: EncoderPhase) -> Self {
        Self {
            origin,
            wake_phase: Some(wake_phase),
            leader: None,
            intermediate: None,
            intermediate_samples: 0,
            origin_samples: 0,
            target_samples: 0,
        }
    }

    pub fn sample(&mut self, a_high: bool, b_high: bool) -> TrackingResult {
        let state = state_from_pins(a_high, b_high);
        let target = self.origin ^ 0b11;

        if state == self.origin {
            // A sampled return to the starting detent means the first edge was
            // bounce. Let a subsequent departure choose the leader again.
            self.wake_phase = None;
            self.leader = None;
            self.intermediate = None;
            self.intermediate_samples = 0;
            self.target_samples = 0;
            self.origin_samples = self.origin_samples.saturating_add(1);
            return if self.origin_samples >= DETENT_STABLE_SAMPLES {
                TrackingResult::Cancelled
            } else {
                TrackingResult::InProgress
            };
        }

        self.origin_samples = 0;
        if state == target {
            if self.leader.is_none() {
                self.leader = self.intermediate.or(self.wake_phase);
            }
            self.target_samples = self.target_samples.saturating_add(1);
            if self.target_samples < DETENT_STABLE_SAMPLES {
                return TrackingResult::InProgress;
            }

            return match self.leader {
                Some(EncoderPhase::A) => TrackingResult::Detent(DetentDirection::Negative),
                Some(EncoderPhase::B) => TrackingResult::Detent(DetentDirection::Positive),
                None => TrackingResult::Cancelled,
            };
        }

        self.target_samples = 0;
        let phase = if state ^ self.origin == 0b10 {
            EncoderPhase::A
        } else {
            EncoderPhase::B
        };
        if self.intermediate == Some(phase) {
            self.intermediate_samples = self.intermediate_samples.saturating_add(1);
        } else {
            self.intermediate = Some(phase);
            self.intermediate_samples = 1;
        }
        if self.intermediate_samples >= INTERMEDIATE_STABLE_SAMPLES {
            self.leader = Some(phase);
        }
        TrackingResult::InProgress
    }
}

pub const fn state_from_pins(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | b_high as u8
}

pub const fn is_detent_state(state: u8) -> bool {
    matches!(state, 0b00 | 0b11)
}

#[cfg(test)]
mod tests {
    use super::{DetentDirection, DetentTracker, EncoderPhase, TrackingResult, state_from_pins};

    fn finish(tracker: &mut DetentTracker, a_high: bool, b_high: bool) -> TrackingResult {
        let mut result = TrackingResult::InProgress;
        for _ in 0..10 {
            result = tracker.sample(a_high, b_high);
        }
        result
    }

    #[test]
    fn a_leads_in_both_negative_half_pulses() {
        let mut from_high = DetentTracker::new(state_from_pins(true, true), EncoderPhase::A);
        from_high.sample(false, true);
        from_high.sample(false, true);
        assert_eq!(
            finish(&mut from_high, false, false),
            TrackingResult::Detent(DetentDirection::Negative)
        );

        let mut from_low = DetentTracker::new(state_from_pins(false, false), EncoderPhase::A);
        from_low.sample(true, false);
        from_low.sample(true, false);
        assert_eq!(
            finish(&mut from_low, true, true),
            TrackingResult::Detent(DetentDirection::Negative)
        );
    }

    #[test]
    fn b_leads_in_both_positive_half_pulses() {
        let mut from_high = DetentTracker::new(state_from_pins(true, true), EncoderPhase::B);
        from_high.sample(true, false);
        from_high.sample(true, false);
        assert_eq!(
            finish(&mut from_high, false, false),
            TrackingResult::Detent(DetentDirection::Positive)
        );

        let mut from_low = DetentTracker::new(state_from_pins(false, false), EncoderPhase::B);
        from_low.sample(false, true);
        from_low.sample(false, true);
        assert_eq!(
            finish(&mut from_low, true, true),
            TrackingResult::Detent(DetentDirection::Positive)
        );
    }

    #[test]
    fn bounce_back_to_origin_does_not_leak_the_old_leader() {
        let mut tracker = DetentTracker::new(state_from_pins(true, true), EncoderPhase::A);
        tracker.sample(false, true);
        tracker.sample(false, true);
        tracker.sample(true, true);
        tracker.sample(true, false);
        tracker.sample(true, false);
        assert_eq!(
            finish(&mut tracker, false, false),
            TrackingResult::Detent(DetentDirection::Positive)
        );
    }
}
