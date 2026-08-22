#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetentDirection {
    Clockwise,
    CounterClockwise,
}

const DIR_CLOCKWISE: u8 = 0x10;
const DIR_COUNTER_CLOCKWISE: u8 = 0x20;
const STATE_MASK: u8 = 0x0f;
const DIRECTION_MASK: u8 = 0x30;

const START_00: u8 = 0;
const CCW_BEGIN: u8 = 1;
const CW_BEGIN: u8 = 2;
const START_11: u8 = 3;
const CW_BEGIN_11: u8 = 4;
const CCW_BEGIN_11: u8 = 5;

/// Half-step Gray-code state machine.
///
/// The 9-pulse/18-detent encoder has a detent at each half-cycle. Phase B is
/// not specified at the mechanical detent, so its instantaneous level must
/// not be used to decide direction. This table accepts only a completed valid
/// Gray-code path to 00 or 11 and naturally walks back on contact chatter.
const TRANSITIONS: [[u8; 4]; 6] = [
    // Input state:       00                 01                 10                 11
    /* START_00 */
    [START_00, CCW_BEGIN, CW_BEGIN, START_11],
    /* CCW_BEGIN */
    [
        START_00,
        CCW_BEGIN,
        START_00,
        START_11 | DIR_COUNTER_CLOCKWISE,
    ],
    /* CW_BEGIN */ [START_00, START_00, CW_BEGIN, START_11 | DIR_CLOCKWISE],
    /* START_11 */ [START_00, CW_BEGIN_11, CCW_BEGIN_11, START_11],
    /* CW_BEGIN_11 */ [START_00 | DIR_CLOCKWISE, CW_BEGIN_11, START_11, START_11],
    /* CCW_BEGIN_11 */
    [
        START_00 | DIR_COUNTER_CLOCKWISE,
        START_11,
        CCW_BEGIN_11,
        START_11,
    ],
];

pub struct HalfStepDecoder {
    state: u8,
}

impl HalfStepDecoder {
    pub const fn new(initial_pins: u8) -> Self {
        let state = match initial_pins & 0b11 {
            0b11 => START_11,
            _ => START_00,
        };
        Self { state }
    }

    pub fn update(&mut self, pins: u8) -> Option<DetentDirection> {
        self.state = TRANSITIONS[(self.state & STATE_MASK) as usize][(pins & 0b11) as usize];
        match self.state & DIRECTION_MASK {
            DIR_CLOCKWISE => Some(DetentDirection::Clockwise),
            DIR_COUNTER_CLOCKWISE => Some(DetentDirection::CounterClockwise),
            _ => None,
        }
    }
}

pub const fn state_from_pins(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | b_high as u8
}

#[cfg(test)]
mod tests {
    use super::{DetentDirection, HalfStepDecoder};

    fn collect(decoder: &mut HalfStepDecoder, states: &[u8]) -> std::vec::Vec<DetentDirection> {
        states
            .iter()
            .filter_map(|state| decoder.update(*state))
            .collect()
    }

    #[test]
    fn one_electrical_cycle_emits_two_clockwise_detents() {
        let mut decoder = HalfStepDecoder::new(0b00);
        assert_eq!(
            collect(&mut decoder, &[0b10, 0b11, 0b01, 0b00]),
            [DetentDirection::Clockwise, DetentDirection::Clockwise]
        );
    }

    #[test]
    fn one_electrical_cycle_emits_two_counter_clockwise_detents() {
        let mut decoder = HalfStepDecoder::new(0b00);
        assert_eq!(
            collect(&mut decoder, &[0b01, 0b11, 0b10, 0b00]),
            [
                DetentDirection::CounterClockwise,
                DetentDirection::CounterClockwise
            ]
        );
    }

    #[test]
    fn nine_pulses_emit_exactly_eighteen_identical_detents() {
        for (cycle, expected) in [
            ([0b10, 0b11, 0b01, 0b00], DetentDirection::Clockwise),
            ([0b01, 0b11, 0b10, 0b00], DetentDirection::CounterClockwise),
        ] {
            let mut decoder = HalfStepDecoder::new(0b00);
            let mut emitted = std::vec::Vec::new();
            for _ in 0..9 {
                emitted.extend(collect(&mut decoder, &cycle));
            }
            assert_eq!(emitted.len(), 18);
            assert!(emitted.iter().all(|direction| *direction == expected));
        }
    }

    #[test]
    fn contact_chatter_walks_back_without_emitting() {
        let mut decoder = HalfStepDecoder::new(0b00);
        assert_eq!(
            collect(&mut decoder, &[0b10, 0b00, 0b10, 0b00, 0b10, 0b11]),
            [DetentDirection::Clockwise]
        );
    }

    #[test]
    fn reversal_before_a_completed_detent_emits_only_the_new_direction() {
        let mut decoder = HalfStepDecoder::new(0b00);
        assert_eq!(
            collect(&mut decoder, &[0b10, 0b00, 0b01, 0b11]),
            [DetentDirection::CounterClockwise]
        );
    }

    #[test]
    fn invalid_two_bit_jump_does_not_emit() {
        let mut decoder = HalfStepDecoder::new(0b00);
        assert!(collect(&mut decoder, &[0b11]).is_empty());
    }
}
