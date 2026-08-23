//! Quadrature decoder for the left-half rotary encoder (9 pulse / 18 click).
//!
//! ## What the datasheet waveform actually shows
//!
//! In the "ENCODER OUTPUT SIGNAL" diagram (CW rotation, detents marked with
//! vertical dash-dot lines), the two phases are *not* symmetric:
//!
//! - Phase A toggles once per click, roughly midway **between** detents.
//! - Phase B toggles once per click, essentially **at** the detent rest
//!   positions.
//!
//! So while the knob rests in a detent, phase B sits right on its own
//! switching threshold. It can chatter there indefinitely, and whether it has
//! "already" switched at a given rest is effectively undefined. Phase A, in
//! contrast, rests far away from its threshold and is clean.
//!
//! This explains every failure recorded in this repository's history:
//!
//! - Sampling B at A's edge: in the direction where B moves first, A's edge
//!   arrives while B is still bouncing, so the read alternates -> the
//!   "one direction fine, other direction alternates up/down" symptom.
//! - Hardware QDEC with its debounce filter: B never presents a stable level
//!   around the rests, so its transition is accepted late (or merged with A's
//!   into an invalid double transition) and the accumulator reports 0 or ±1
//!   per click -> the "responds once per two clicks" symptom.
//! - Any state machine that discards partial progress on an unexpected
//!   transition: rest chatter constantly resets it -> lost clicks.
//!
//! ## How this decoder works
//!
//! Both phases are sampled at a fixed fast rate and every *valid* Gray-code
//! transition of the raw pair adds ±1 to a signed position counter. Nothing
//! is ever gated on stability and no partial progress is discarded:
//!
//! - Contact bounce and rest chatter produce +1/-1 pairs that cancel exactly.
//! - A real click always nets exactly ±2 (one A toggle plus one B toggle).
//!
//! A detent is emitted whenever the position moves 2 counts away from the
//! position of the last emitted detent (the anchor). Rest chatter only ever
//! moves the position ±1 around the anchor, so it can never emit; a real
//! click always reaches ±2, so it always emits, in both directions, at any
//! speed the sampling can follow.
//!
//! If both phases change within one sample (only possible when a bounce edge
//! coincides with the other phase's edge in the same sample window), the
//! transition direction is unknowable. It contributes 0 and the tracked state
//! resyncs; at worst one click is swallowed once, after which the anchor is
//! aligned again. This cannot corrupt the direction.

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

/// Signed quarter-steps per detent: one A toggle plus one B toggle.
const COUNTS_PER_DETENT: i32 = 2;

/// `TRANSITION_DELTA[(previous << 2) | current]` where a state is
/// `(a as u8) << 1 | (b as u8)`.
///
/// +1 for each Gray step along 11 -> 01 -> 00 -> 10 -> 11 (datasheet CW),
/// -1 for the reverse, 0 for no change and for invalid double transitions.
const TRANSITION_DELTA: [i8; 16] = [
    0, -1, 1, 0, // from 00
    1, 0, 0, -1, // from 01
    -1, 0, 0, 1, // from 10
    0, 1, -1, 0, // from 11
];

pub struct QuadratureAccumulator {
    previous_state: u8,
    position: i32,
    anchor: i32,
}

const fn encode(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | (b_high as u8)
}

impl QuadratureAccumulator {
    pub const fn new(a_high: bool, b_high: bool) -> Self {
        Self {
            previous_state: encode(a_high, b_high),
            position: 0,
            anchor: 0,
        }
    }

    /// Net signed quarter-steps integrated since construction.
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Feed one raw sample of both phases. Returns a detent when the knob
    /// has completed one full click since the last emitted detent.
    ///
    /// A single sample changes the position by at most 1, so at most one
    /// detent can be emitted per sample.
    pub fn update(&mut self, a_high: bool, b_high: bool) -> Option<Detent> {
        let state = encode(a_high, b_high);
        let delta = TRANSITION_DELTA[usize::from((self.previous_state << 2) | state)];
        self.previous_state = state;
        self.position += i32::from(delta);

        if self.position - self.anchor >= COUNTS_PER_DETENT {
            self.anchor += COUNTS_PER_DETENT;
            Some(Detent::Clockwise)
        } else if self.position - self.anchor <= -COUNTS_PER_DETENT {
            self.anchor -= COUNTS_PER_DETENT;
            Some(Detent::CounterClockwise)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Detent, QuadratureAccumulator};

    const S11: (bool, bool) = (true, true);
    const S01: (bool, bool) = (false, true);
    const S00: (bool, bool) = (false, false);
    const S10: (bool, bool) = (true, false);

    fn feed(
        decoder: &mut QuadratureAccumulator,
        states: &[(bool, bool)],
    ) -> (u32, u32) {
        let mut clockwise = 0;
        let mut counterclockwise = 0;
        for &(a, b) in states {
            match decoder.update(a, b) {
                Some(Detent::Clockwise) => clockwise += 1,
                Some(Detent::CounterClockwise) => counterclockwise += 1,
                None => {}
            }
        }
        (clockwise, counterclockwise)
    }

    #[test]
    fn clockwise_click_with_bounce_on_both_phases_emits_exactly_once() {
        let mut decoder = QuadratureAccumulator::new(true, true);
        // A bounces while leaving the rest, then B bounces on arrival at the
        // next detent. Every flip-flop pair cancels; the click nets +2.
        let states = [
            S11, S01, S11, S01, S11, S01, // A edge with bounce
            S01, S01, S01, // stable mid-travel
            S00, S01, S00, S01, S00, // B edge with bounce at arrival
            S00, S00,
        ];
        assert_eq!(feed(&mut decoder, &states), (1, 0));
    }

    #[test]
    fn counterclockwise_click_with_bounce_emits_exactly_once() {
        let mut decoder = QuadratureAccumulator::new(false, false);
        // CCW from a 00 rest: B moves first (at departure), then A.
        let states = [
            S00, S01, S00, S01, S00, S01, // B edge with bounce at departure
            S01, S01, S01, // stable mid-travel
            S11, S01, S11, S01, S11, // A edge with bounce
            S11, S11,
        ];
        assert_eq!(feed(&mut decoder, &states), (0, 1));
    }

    #[test]
    fn rest_chatter_around_the_detent_never_emits() {
        // Phase B switches exactly at the detents, so it may chatter
        // indefinitely while the knob rests. That is a +1/-1 oscillation
        // around the anchor and must never reach the +-2 threshold.
        let mut decoder = QuadratureAccumulator::new(true, true);
        let mut states = [S11; 100];
        for (index, state) in states.iter_mut().enumerate() {
            if index % 2 == 0 {
                *state = S10;
            }
        }
        assert_eq!(feed(&mut decoder, &states), (0, 0));

        let mut decoder = QuadratureAccumulator::new(false, false);
        let mut states = [S00; 100];
        for (index, state) in states.iter_mut().enumerate() {
            if index % 2 == 0 {
                *state = S01;
            }
        }
        assert_eq!(feed(&mut decoder, &states), (0, 0));
    }

    #[test]
    fn chatter_at_the_new_rest_after_a_click_does_not_double_emit() {
        let mut decoder = QuadratureAccumulator::new(true, true);
        let click = [S01, S00];
        assert_eq!(feed(&mut decoder, &click), (1, 0));
        // Arrived at the 00 rest; B chatters on its threshold (00 <-> 01).
        let chatter = [S01, S00, S01, S00, S01, S00, S01, S00];
        assert_eq!(feed(&mut decoder, &chatter), (0, 0));
    }

    #[test]
    fn eighteen_clicks_per_revolution_in_each_direction() {
        let cw_cycle = [S01, S00, S10, S11];
        let mut decoder = QuadratureAccumulator::new(true, true);
        let mut clockwise = 0;
        for _ in 0..9 {
            for state in cw_cycle {
                // Repeat each sample: duplicates must not matter.
                let (cw, ccw) = feed(&mut decoder, &[state, state, state]);
                clockwise += cw;
                assert_eq!(ccw, 0);
            }
        }
        assert_eq!(clockwise, 18);

        let ccw_cycle = [S10, S00, S01, S11];
        let mut counterclockwise = 0;
        for _ in 0..9 {
            for state in ccw_cycle {
                let (cw, ccw) = feed(&mut decoder, &[state, state, state]);
                counterclockwise += ccw;
                assert_eq!(cw, 0);
            }
        }
        assert_eq!(counterclockwise, 18);
    }

    #[test]
    fn direction_reversal_loses_no_motion() {
        let mut decoder = QuadratureAccumulator::new(true, true);
        assert_eq!(feed(&mut decoder, &[S01, S00]), (1, 0));
        assert_eq!(feed(&mut decoder, &[S01, S11]), (0, 1));
        assert_eq!(feed(&mut decoder, &[S01, S00]), (1, 0));
    }

    #[test]
    fn a_missed_intermediate_state_swallows_at_most_one_click() {
        let mut decoder = QuadratureAccumulator::new(true, true);
        // Both phases appear to change in one sample: direction unknowable,
        // so nothing may be emitted for that click.
        assert_eq!(feed(&mut decoder, &[S00]), (0, 0));
        // The decoder resynchronized at 00; following clicks emit normally.
        assert_eq!(feed(&mut decoder, &[S10, S11]), (1, 0));
        assert_eq!(feed(&mut decoder, &[S01, S00]), (1, 0));
    }

    #[test]
    fn slow_sampling_that_skips_bounce_still_counts_the_click() {
        // Levels sampled after every bounce settled: the pure sequence.
        let mut decoder = QuadratureAccumulator::new(true, true);
        assert_eq!(feed(&mut decoder, &[S01]), (0, 0));
        assert_eq!(feed(&mut decoder, &[S00]), (1, 0));
    }
}
