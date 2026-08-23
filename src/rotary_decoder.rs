//! Decoder for the left-half rotary encoder (BM4.0A01 style, 9 pulse /
//! 18 click).
//!
//! ## What the part actually outputs (measured on the device)
//!
//! On-device diagnostics (raw transition counters on the left LCD, sampled
//! every ~61 us) showed, for 4 slow clicks in one direction:
//!
//! - 4 clean single transitions of phase A, one per click, all with the
//!   correct direction sign — A toggles midway between detents where the
//!   knob moves slowly, exactly as the datasheet draws it.
//! - 0 clean single transitions of phase B. B toggles *at* the detent rest
//!   positions, and the detent spring snaps the shaft through that zone in
//!   well under a sample period, together with contact bounce on both
//!   phases. B's edge is therefore never observable as a valid Gray-code
//!   step: it always arrives as an ambiguous both-bits-changed double.
//!
//! So a full quadrature accumulator can only ever integrate ±1 per click
//! (A's step); the ±2-per-click threshold then fires on every second click.
//! Treating this part as a generic quadrature encoder is what every failed
//! attempt in this repository's history has in common.
//!
//! ## How this decoder works
//!
//! The part is decoded the way its geometry intends:
//!
//! - **Phase A is the clock.** One debounced A level change = one click.
//!   A rests far from its own threshold, so it is quiet at rest; its edge
//!   sits mid-travel, clear of the snap. Bounce is removed by requiring the
//!   new level to persist for [`DEBOUNCE_SAMPLES`] consecutive samples.
//! - **Phase B is the direction bit.** At A's edge the direction is
//!   `stable_a XOR stable_b`, using B's last *debounced* level — i.e. the
//!   level B held during travel, from *before* the edge. (The historical
//!   bug was sampling B shortly *after* A's edge, which can land inside the
//!   snap where B is bouncing; that produced the "one direction fine, the
//!   other alternates up/down" symptom.)
//! - B never emits anything. Its rest chatter and its unreadable snap edge
//!   only ever update the direction reference once a level has been held
//!   long enough to be trustworthy.
//!
//! Between two adjacent detents B is constant (its edges are at the
//! detents), so by the time A's mid-travel edge arrives, B has been settled
//! for about half a click of travel in either rotation direction — orders
//! of magnitude longer than the debounce window, even when spinning fast.

/// One physical detent click.
///
/// `Clockwise` corresponds to the datasheet's CW rotation, i.e. the raw
/// (A, B) state sequence 11 -> 01 -> 00 -> 10 -> 11. This matches the
/// direction previously reported as `Direction::Clockwise` by the original
/// GPIO decoder, so existing Vial encoder mappings keep their meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detent {
    Clockwise,
    CounterClockwise,
}

/// Consecutive identical samples required before a level counts as settled.
/// At the ~61 us sample period this is ~1 ms — longer than contact bounce,
/// far shorter than the shortest plateau between edges when spinning fast.
pub const DEBOUNCE_SAMPLES: u8 = 16;

pub struct ClockedDetentDecoder {
    stable_a: bool,
    stable_b: bool,
    candidate_a: bool,
    candidate_b: bool,
    run_a: u8,
    run_b: u8,
}

impl ClockedDetentDecoder {
    pub const fn new(a_high: bool, b_high: bool) -> Self {
        Self {
            stable_a: a_high,
            stable_b: b_high,
            candidate_a: a_high,
            candidate_b: b_high,
            run_a: DEBOUNCE_SAMPLES,
            run_b: DEBOUNCE_SAMPLES,
        }
    }

    /// Feed one raw sample of both phases. Returns a detent when phase A
    /// completes a debounced level change.
    pub fn update(&mut self, a_high: bool, b_high: bool) -> Option<Detent> {
        if a_high == self.candidate_a {
            self.run_a = self.run_a.saturating_add(1);
        } else {
            self.candidate_a = a_high;
            self.run_a = 1;
        }
        if b_high == self.candidate_b {
            self.run_b = self.run_b.saturating_add(1);
        } else {
            self.candidate_b = b_high;
            self.run_b = 1;
        }

        let mut detent = None;
        if self.run_a >= DEBOUNCE_SAMPLES && self.candidate_a != self.stable_a {
            self.stable_a = self.candidate_a;
            // Direction from B's level held during travel, before this edge.
            detent = Some(if self.stable_a != self.stable_b {
                Detent::Clockwise
            } else {
                Detent::CounterClockwise
            });
        }
        // Promote B only after the direction was taken, so that in the
        // (theoretical) case of both phases settling in the same sample the
        // pre-edge B level is still the one that decides.
        if self.run_b >= DEBOUNCE_SAMPLES && self.candidate_b != self.stable_b {
            self.stable_b = self.candidate_b;
        }
        detent
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockedDetentDecoder, DEBOUNCE_SAMPLES, Detent};

    const STABLE: usize = DEBOUNCE_SAMPLES as usize;

    struct Harness {
        decoder: ClockedDetentDecoder,
        clockwise: u32,
        counterclockwise: u32,
    }

    impl Harness {
        fn new(a: bool, b: bool) -> Self {
            Self {
                decoder: ClockedDetentDecoder::new(a, b),
                clockwise: 0,
                counterclockwise: 0,
            }
        }

        /// Feed the same raw sample `count` times.
        fn hold(&mut self, (a, b): (bool, bool), count: usize) {
            for _ in 0..count {
                match self.decoder.update(a, b) {
                    Some(Detent::Clockwise) => self.clockwise += 1,
                    Some(Detent::CounterClockwise) => self.counterclockwise += 1,
                    None => {}
                }
            }
        }

        /// Alternate between two raw samples, `period` samples each,
        /// `flips` times — bounce or chatter, always shorter than the
        /// debounce window per level.
        fn flap(&mut self, first: (bool, bool), second: (bool, bool), period: usize, flips: usize) {
            assert!(period < STABLE);
            for flip in 0..flips {
                let state = if flip % 2 == 0 { first } else { second };
                self.hold(state, period);
            }
        }

        fn assert_events(&mut self, clockwise: u32, counterclockwise: u32) {
            assert_eq!(
                (self.clockwise, self.counterclockwise),
                (clockwise, counterclockwise)
            );
            self.clockwise = 0;
            self.counterclockwise = 0;
        }
    }

    const S11: (bool, bool) = (true, true);
    const S01: (bool, bool) = (false, true);
    const S00: (bool, bool) = (false, false);
    const S10: (bool, bool) = (true, false);

    /// One measured-style CW click starting from a settled (1,1) rest:
    /// bouncy A edge mid-travel, then the snap where both phases jitter
    /// together and B comes out flipped.
    fn cw_click_from_11(harness: &mut Harness) {
        harness.flap(S01, S11, 3, 5); // A edge with bounce
        harness.hold(S01, 200); // mid-travel plateau
        harness.flap(S00, S11, 2, 6); // snap: ambiguous doubles
        harness.hold(S00, 200); // settled at the next rest
    }

    fn cw_click_from_00(harness: &mut Harness) {
        harness.flap(S10, S00, 3, 5);
        harness.hold(S10, 200);
        harness.flap(S11, S00, 2, 6);
        harness.hold(S11, 200);
    }

    /// One CCW click starting from a settled (0,0) rest: B crawls across
    /// its threshold at departure (long flaps), then A edges mid-travel.
    fn ccw_click_from_00(harness: &mut Harness) {
        harness.hold(S01, STABLE + 5); // B settles high at departure
        harness.hold(S00, 3); // one late B bounce, too short to matter
        harness.hold(S01, 200);
        harness.flap(S11, S01, 3, 5); // A edge with bounce
        harness.hold(S11, 200); // settled at the next rest
    }

    fn ccw_click_from_11(harness: &mut Harness) {
        harness.hold(S10, STABLE + 5);
        harness.hold(S11, 3);
        harness.hold(S10, 200);
        harness.flap(S00, S10, 3, 5);
        harness.hold(S00, 200);
    }

    #[test]
    fn cw_clicks_emit_exactly_once_per_click() {
        let mut harness = Harness::new(true, true);
        cw_click_from_11(&mut harness);
        harness.assert_events(1, 0);
        cw_click_from_00(&mut harness);
        harness.assert_events(1, 0);
    }

    #[test]
    fn ccw_clicks_emit_exactly_once_per_click() {
        let mut harness = Harness::new(false, false);
        ccw_click_from_00(&mut harness);
        harness.assert_events(0, 1);
        ccw_click_from_11(&mut harness);
        harness.assert_events(0, 1);
    }

    #[test]
    fn eighteen_clicks_per_revolution_in_each_direction() {
        let mut harness = Harness::new(true, true);
        for _ in 0..9 {
            cw_click_from_11(&mut harness);
            cw_click_from_00(&mut harness);
        }
        harness.assert_events(18, 0);
        for _ in 0..9 {
            ccw_click_from_11(&mut harness);
            ccw_click_from_00(&mut harness);
        }
        harness.assert_events(0, 18);
    }

    #[test]
    fn direction_reversal_is_immediate_and_correct() {
        let mut harness = Harness::new(true, true);
        cw_click_from_11(&mut harness);
        harness.assert_events(1, 0);
        ccw_click_from_00(&mut harness);
        harness.assert_events(0, 1);
        cw_click_from_11(&mut harness);
        harness.assert_events(1, 0);
    }

    #[test]
    fn rest_chatter_on_b_never_emits() {
        // B sits on its own switching threshold at every detent and may
        // chatter there indefinitely — fast or slow.
        let mut harness = Harness::new(false, false);
        harness.flap(S01, S00, 2, 100); // fast chatter
        for _ in 0..10 {
            harness.hold(S01, STABLE + 10); // slow chatter: each level settles
            harness.hold(S00, STABLE + 10);
        }
        harness.assert_events(0, 0);
    }

    #[test]
    fn short_glitches_on_a_are_ignored() {
        let mut harness = Harness::new(true, true);
        harness.flap(S01, S11, 4, 20);
        harness.hold(S11, 200);
        harness.assert_events(0, 0);
    }

    #[test]
    fn snap_doubles_alone_never_emit() {
        // Both phases jittering together (the arrival snap) with A settling
        // back where it was: no click may be reported.
        let mut harness = Harness::new(false, true);
        harness.flap(S00, S11, 2, 8);
        harness.hold(S00, 200);
        harness.assert_events(0, 0);
    }

    #[test]
    fn fast_rotation_still_counts_every_click() {
        // ~2 ms per half-click plateau: just over the debounce window.
        let mut harness = Harness::new(true, true);
        for _ in 0..9 {
            harness.hold(S01, STABLE * 2);
            harness.hold(S00, STABLE * 2);
            harness.hold(S10, STABLE * 2);
            harness.hold(S11, STABLE * 2);
        }
        harness.assert_events(18, 0);
    }
}
