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

const DETENT_STABLE_SAMPLES: u8 = 10;

/// Tracks one half-pulse of a 9-pulse/18-click encoder.
///
/// Direction is determined by the first contact that leaves the current
/// detent. Later contact bounce cannot reverse that decision. A click is only
/// emitted after the opposite detent (`00` or `11`) remains stable.
pub struct DetentTracker {
    origin: u8,
    leader: EncoderPhase,
    origin_samples: u8,
    target_samples: u8,
    finished: bool,
}

impl DetentTracker {
    pub const fn new(origin: u8, captured_phase: EncoderPhase) -> Self {
        Self {
            origin,
            leader: captured_phase,
            origin_samples: 0,
            target_samples: 0,
            finished: false,
        }
    }

    pub fn sample(&mut self, a_high: bool, b_high: bool) -> TrackingResult {
        if self.finished {
            return TrackingResult::Cancelled;
        }

        let state = state_from_pins(a_high, b_high);
        let target = self.origin ^ 0b11;

        if state == self.origin {
            // Keep the hardware-captured leader through short contact bounce.
            // A stable return cancels this attempt; the caller then rearms PPI
            // before accepting a different leader.
            self.target_samples = 0;
            self.origin_samples = self.origin_samples.saturating_add(1);
            if self.origin_samples >= DETENT_STABLE_SAMPLES {
                self.finished = true;
                return TrackingResult::Cancelled;
            }
            return TrackingResult::InProgress;
        }

        self.origin_samples = 0;
        if state == target {
            self.target_samples = self.target_samples.saturating_add(1);
            if self.target_samples < DETENT_STABLE_SAMPLES {
                return TrackingResult::InProgress;
            }

            self.finished = true;
            return match self.leader {
                EncoderPhase::A => TrackingResult::Detent(DetentDirection::Negative),
                EncoderPhase::B => TrackingResult::Detent(DetentDirection::Positive),
            };
        }

        self.target_samples = 0;
        TrackingResult::InProgress
    }
}

pub const fn state_from_pins(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | b_high as u8
}

pub const fn is_detent_state(state: u8) -> bool {
    matches!(state, 0b00 | 0b11)
}

pub const fn first_captured_phase(
    a_timestamp: Option<u32>,
    b_timestamp: Option<u32>,
) -> Option<EncoderPhase> {
    match (a_timestamp, b_timestamp) {
        (Some(a), Some(b)) if b < a => Some(EncoderPhase::B),
        (Some(_), _) => Some(EncoderPhase::A),
        (None, Some(_)) => Some(EncoderPhase::B),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DetentDirection, DetentTracker, EncoderPhase, TrackingResult, first_captured_phase,
    };

    const HIGH_DETENT: u8 = 0b11;
    const LOW_DETENT: u8 = 0b00;

    fn sample_state(tracker: &mut DetentTracker, state: u8) -> TrackingResult {
        tracker.sample(state & 0b10 != 0, state & 0b01 != 0)
    }

    fn settle(tracker: &mut DetentTracker, state: u8) -> TrackingResult {
        let mut result = TrackingResult::InProgress;
        for _ in 0..10 {
            result = sample_state(tracker, state);
        }
        result
    }

    fn intermediate(origin: u8, phase: EncoderPhase) -> u8 {
        origin
            ^ match phase {
                EncoderPhase::A => 0b10,
                EncoderPhase::B => 0b01,
            }
    }

    fn run_documented_half_click(origin: u8, leading_phase: EncoderPhase) -> (u8, DetentDirection) {
        let target = origin ^ 0b11;
        let mut tracker = DetentTracker::new(origin, leading_phase);
        let middle = intermediate(origin, leading_phase);
        assert_eq!(
            sample_state(&mut tracker, middle),
            TrackingResult::InProgress
        );
        assert_eq!(
            sample_state(&mut tracker, middle),
            TrackingResult::InProgress
        );
        let TrackingResult::Detent(direction) = settle(&mut tracker, target) else {
            panic!("documented half-click did not emit exactly one detent");
        };
        assert_eq!(
            sample_state(&mut tracker, target),
            TrackingResult::Cancelled
        );
        (target, direction)
    }

    #[test]
    fn documented_clockwise_waveform_emits_18_identical_directions() {
        let mut state = HIGH_DETENT;
        for click in 0..18 {
            let (next, direction) = run_documented_half_click(state, EncoderPhase::A);
            assert_eq!(direction, DetentDirection::Negative, "click {click}");
            state = next;
        }
        assert_eq!(state, HIGH_DETENT);
    }

    #[test]
    fn documented_counterclockwise_waveform_emits_18_identical_directions() {
        let mut state = HIGH_DETENT;
        for click in 0..18 {
            let (next, direction) = run_documented_half_click(state, EncoderPhase::B);
            assert_eq!(direction, DetentDirection::Positive, "click {click}");
            state = next;
        }
        assert_eq!(state, HIGH_DETENT);
    }

    #[test]
    fn both_detent_polarities_use_the_same_direction() {
        for origin in [HIGH_DETENT, LOW_DETENT] {
            assert_eq!(
                run_documented_half_click(origin, EncoderPhase::A).1,
                DetentDirection::Negative
            );
            assert_eq!(
                run_documented_half_click(origin, EncoderPhase::B).1,
                DetentDirection::Positive
            );
        }
    }

    #[test]
    fn opposite_contact_bounce_at_target_cannot_overwrite_the_leader() {
        for origin in [HIGH_DETENT, LOW_DETENT] {
            for (leader, opposite, expected) in [
                (EncoderPhase::A, EncoderPhase::B, DetentDirection::Negative),
                (EncoderPhase::B, EncoderPhase::A, DetentDirection::Positive),
            ] {
                let target = origin ^ 0b11;
                let mut tracker = DetentTracker::new(origin, leader);
                sample_state(&mut tracker, intermediate(origin, leader));
                sample_state(&mut tracker, intermediate(origin, leader));
                for _ in 0..4 {
                    sample_state(&mut tracker, target);
                }

                // This used to overwrite the latched leader and reverse every
                // other click.
                sample_state(&mut tracker, intermediate(origin, opposite));
                sample_state(&mut tracker, intermediate(origin, opposite));
                assert_eq!(
                    settle(&mut tracker, target),
                    TrackingResult::Detent(expected),
                    "origin={origin:02b}, leader={leader:?}"
                );
            }
        }
    }

    #[test]
    fn sampled_states_cannot_replace_the_hardware_captured_leader() {
        for origin in [HIGH_DETENT, LOW_DETENT] {
            for (captured, sampled, expected) in [
                (EncoderPhase::A, EncoderPhase::B, DetentDirection::Negative),
                (EncoderPhase::B, EncoderPhase::A, DetentDirection::Positive),
            ] {
                let mut tracker = DetentTracker::new(origin, captured);
                sample_state(&mut tracker, intermediate(origin, sampled));
                sample_state(&mut tracker, origin);
                sample_state(&mut tracker, intermediate(origin, sampled));
                sample_state(&mut tracker, intermediate(origin, sampled));
                assert_eq!(
                    settle(&mut tracker, origin ^ 0b11),
                    TrackingResult::Detent(expected),
                    "origin={origin:02b}, captured={captured:?}, sampled={sampled:?}"
                );
            }
        }
    }

    #[test]
    fn stable_return_rearms_before_accepting_a_real_reversal() {
        let mut first_attempt = DetentTracker::new(HIGH_DETENT, EncoderPhase::A);
        sample_state(&mut first_attempt, 0b01);
        assert_eq!(
            settle(&mut first_attempt, HIGH_DETENT),
            TrackingResult::Cancelled
        );

        let mut rearmed = DetentTracker::new(HIGH_DETENT, EncoderPhase::B);
        assert_eq!(
            settle(&mut rearmed, LOW_DETENT),
            TrackingResult::Detent(DetentDirection::Positive)
        );
    }

    #[test]
    fn contact_chatter_does_not_emit_or_multiply_clicks() {
        let mut tracker = DetentTracker::new(HIGH_DETENT, EncoderPhase::A);
        for _ in 0..8 {
            assert_eq!(sample_state(&mut tracker, 0b01), TrackingResult::InProgress);
            assert_eq!(
                sample_state(&mut tracker, HIGH_DETENT),
                TrackingResult::InProgress
            );
        }
        sample_state(&mut tracker, 0b01);
        sample_state(&mut tracker, 0b01);
        assert_eq!(
            settle(&mut tracker, LOW_DETENT),
            TrackingResult::Detent(DetentDirection::Negative)
        );
        for _ in 0..20 {
            assert_eq!(
                sample_state(&mut tracker, LOW_DETENT),
                TrackingResult::Cancelled
            );
        }
    }

    #[test]
    fn a_single_intermediate_sample_still_decodes_fast_rotation() {
        for (phase, expected) in [
            (EncoderPhase::A, DetentDirection::Negative),
            (EncoderPhase::B, DetentDirection::Positive),
        ] {
            let mut tracker = DetentTracker::new(HIGH_DETENT, phase);
            sample_state(&mut tracker, intermediate(HIGH_DETENT, phase));
            assert_eq!(
                settle(&mut tracker, LOW_DETENT),
                TrackingResult::Detent(expected)
            );
        }
    }

    #[test]
    fn wake_phase_decodes_when_fast_rotation_skips_the_intermediate_sample() {
        for (phase, expected) in [
            (EncoderPhase::A, DetentDirection::Negative),
            (EncoderPhase::B, DetentDirection::Positive),
        ] {
            let mut tracker = DetentTracker::new(HIGH_DETENT, phase);
            assert_eq!(
                settle(&mut tracker, LOW_DETENT),
                TrackingResult::Detent(expected)
            );
        }
    }

    #[test]
    fn an_edge_that_settles_back_at_origin_is_cancelled() {
        let mut tracker = DetentTracker::new(HIGH_DETENT, EncoderPhase::A);
        sample_state(&mut tracker, 0b01);
        assert_eq!(settle(&mut tracker, HIGH_DETENT), TrackingResult::Cancelled);
    }

    #[test]
    fn hardware_capture_order_wins_over_software_poll_order() {
        assert_eq!(first_captured_phase(Some(10), None), Some(EncoderPhase::A));
        assert_eq!(first_captured_phase(None, Some(10)), Some(EncoderPhase::B));
        assert_eq!(
            first_captured_phase(Some(20), Some(10)),
            Some(EncoderPhase::B)
        );
        assert_eq!(
            first_captured_phase(Some(10), Some(20)),
            Some(EncoderPhase::A)
        );
        assert_eq!(first_captured_phase(None, None), None);
    }
}
