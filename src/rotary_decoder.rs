//! Decoder for the left-half rotary encoder (BM4.0A01 style, 9 pulse /
//! 18 click).
//!
//! ## What the part actually outputs (measured on the device)
//!
//! On-device diagnostics (raw transition counters on the left LCD, sampled
//! every ~61 us) established, click by click:
//!
//! - Phase A produces exactly one real level change per click, in both
//!   directions. It is the only reliable per-click signal ("the clock").
//! - Phase B toggles once per click, but *where* it toggles depends on the
//!   rotation direction (contact hysteresis):
//!   - In one direction B toggles inside the detent snap *after* A's edge,
//!     buried in bounce, often in the very same sample as an A bounce
//!     (ambiguous double transitions).
//!   - In the other direction B toggles *just before* A's edge —
//!     sub-millisecond before it, with its bounce tail crossing the edge.
//!
//! That second case broke the previous revision of this decoder: it read
//! B's *debounced* level at the moment A's edge confirmed, and in the
//! B-leading direction that level was still the stale pre-toggle value on
//! every other click (B's rise and fall sit at different offsets), which
//! produced the alternating up/down output.
//!
//! ## How this decoder works
//!
//! - **Phase A is the clock.** One debounced A level change
//!   ([`DEBOUNCE_SAMPLES`] consecutive samples, ~1 ms) = one click. A rests
//!   far from its threshold, so it is quiet at rest and its edge sits
//!   mid-travel, clear of the snap.
//! - **Direction is resolved [`DIRECTION_RESOLVE_SAMPLES`] samples (~3 ms)
//!   after the confirmed A edge**, as `stable_a XOR stable_b`. By then:
//!   - If B toggled just before / around the edge (B-leading direction),
//!     its new level has settled and is used — correct.
//!   - If B toggles at the *next* snap (A-leading direction), that snap is
//!     at least half a click of travel away (tens of ms at human speeds),
//!     far outside the window, so the pre-edge level is used — correct.
//! - B itself never emits anything; its rest chatter and snap bounce only
//!   ever update the debounced direction reference.
//!
//! The ~4 ms total latency (debounce + resolve window) is imperceptible.
//! The scheme misreads direction only if a full click takes less than the
//! resolve window (hundreds of clicks per second — not reachable by hand).

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
/// At the ~61 us sample period this is ~1 ms — longer than most contact
/// bounce, far shorter than the shortest plateau between edges when
/// spinning fast.
pub const DEBOUNCE_SAMPLES: u8 = 16;

/// Samples between a confirmed A edge and the direction decision (~3 ms at
/// the ~61 us sample period). Long enough for a B toggle that rides just
/// ahead of A's edge (bounce tail included) to settle; short enough that
/// the *next* B toggle — at least half a click of travel away — can never
/// intrude at human rotation speeds.
pub const DIRECTION_RESOLVE_SAMPLES: u8 = 49;

pub struct ClockedDetentDecoder {
    stable_a: bool,
    stable_b: bool,
    candidate_a: bool,
    candidate_b: bool,
    run_a: u8,
    run_b: u8,
    /// A level of a click awaiting its direction decision.
    pending_a: Option<bool>,
    resolve_countdown: u8,
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
            pending_a: None,
            resolve_countdown: 0,
        }
    }

    fn resolve(&self, a_level: bool) -> Detent {
        if a_level != self.stable_b {
            Detent::Clockwise
        } else {
            Detent::CounterClockwise
        }
    }

    /// Feed one raw sample of both phases. Returns a detent once a debounced
    /// phase A level change has its direction resolved.
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

        // Promote B before anything else so a toggle settling in this very
        // sample is visible to a direction decision made below.
        if self.run_b >= DEBOUNCE_SAMPLES && self.candidate_b != self.stable_b {
            self.stable_b = self.candidate_b;
        }

        if self.run_a >= DEBOUNCE_SAMPLES && self.candidate_a != self.stable_a {
            self.stable_a = self.candidate_a;
            // A new click before the previous one resolved (not reachable at
            // human speeds): flush the old one with the best current guess.
            let flushed = self.pending_a.take().map(|level| self.resolve(level));
            self.pending_a = Some(self.stable_a);
            self.resolve_countdown = DIRECTION_RESOLVE_SAMPLES;
            return flushed;
        }

        if let Some(level) = self.pending_a {
            self.resolve_countdown -= 1;
            if self.resolve_countdown == 0 {
                self.pending_a = None;
                return Some(self.resolve(level));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockedDetentDecoder, DEBOUNCE_SAMPLES, DIRECTION_RESOLVE_SAMPLES, Detent};

    const STABLE: usize = DEBOUNCE_SAMPLES as usize;
    const RESOLVED: usize = DIRECTION_RESOLVE_SAMPLES as usize + STABLE + 1;

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

    /// Measured CW anatomy (A-leading direction): bouncy A edge mid-travel,
    /// then the arrival snap where both phases jitter together and B comes
    /// out flipped.
    fn cw_click_from_11(harness: &mut Harness) {
        harness.flap(S01, S11, 3, 5); // A edge with bounce
        harness.hold(S01, 200); // mid-travel plateau; direction resolves here
        harness.flap(S00, S11, 2, 6); // snap: ambiguous doubles
        harness.hold(S00, 200); // settled at the next rest
    }

    fn cw_click_from_00(harness: &mut Harness) {
        harness.flap(S10, S00, 3, 5);
        harness.hold(S10, 200);
        harness.flap(S11, S00, 2, 6);
        harness.hold(S11, 200);
    }

    /// Measured CCW anatomy (B-leading direction): B toggles just before
    /// A's edge with its bounce tail crossing it, then A edges. This is the
    /// case that made the previous revision alternate up/down.
    fn ccw_click_from_11(harness: &mut Harness) {
        harness.hold(S10, 3); // B real toggle, still bouncing...
        harness.hold(S11, 2);
        harness.hold(S10, 4);
        harness.hold(S00, 200); // ...A edges while B's tail settles
    }

    fn ccw_click_from_00(harness: &mut Harness) {
        harness.hold(S01, 3);
        harness.hold(S00, 2);
        harness.hold(S01, 4);
        harness.hold(S11, 200);
    }

    /// B-leading click where the user crawls: B settles well before A edges.
    fn slow_ccw_click_from_00(harness: &mut Harness) {
        harness.hold(S01, STABLE + 5); // B settles high at departure
        harness.hold(S00, 3); // one late B bounce, too short to matter
        harness.hold(S01, 200);
        harness.flap(S11, S01, 3, 5); // A edge with bounce
        harness.hold(S11, 200);
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
    fn ccw_clicks_emit_exactly_once_per_click_despite_b_leading_tightly() {
        let mut harness = Harness::new(true, true);
        ccw_click_from_11(&mut harness);
        harness.assert_events(0, 1);
        ccw_click_from_00(&mut harness);
        harness.assert_events(0, 1);
    }

    #[test]
    fn slow_b_leading_clicks_are_also_correct() {
        let mut harness = Harness::new(false, false);
        slow_ccw_click_from_00(&mut harness);
        harness.assert_events(0, 1);
    }

    #[test]
    fn settled_double_transition_resolves_as_b_led() {
        // Both phases flip in the same sample and stay: only the B-leading
        // anatomy produces this, so it must count as one CCW click.
        let mut harness = Harness::new(true, true);
        harness.hold(S00, RESOLVED + 200);
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
        // ~5 ms per quadrature state = ~50 clicks/s, faster than a hand
        // flick. Each direction decision must land inside its own plateau.
        let mut harness = Harness::new(true, true);
        for _ in 0..9 {
            harness.hold(S01, 80);
            harness.hold(S00, 80);
            harness.hold(S10, 80);
            harness.hold(S11, 80);
        }
        harness.assert_events(18, 0);
        for _ in 0..9 {
            harness.hold(S10, 80);
            harness.hold(S00, 80);
            harness.hold(S01, 80);
            harness.hold(S11, 80);
        }
        harness.assert_events(0, 18);
    }
}
