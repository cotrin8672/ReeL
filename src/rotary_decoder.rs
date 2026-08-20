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

/// Half-step decoder for an encoder whose mechanical detents alternate
/// between the stable AB states `00` and `11`.
///
/// Contact bounce retraces the transition table and cancels naturally. A
/// direction is emitted only after two ordered single-phase transitions reach
/// the adjacent stable detent; partial movement is never carried across a
/// detent or assigned to the next direction.
pub struct HalfStepDecoder {
    state: u8,
    movement: i8,
}

const TRANSITIONS: [i8; 16] = [0, 1, -1, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, -1, 1, 0];

impl HalfStepDecoder {
    pub const fn new(a_high: bool, b_high: bool) -> Self {
        Self {
            state: ((a_high as u8) << 1) | b_high as u8,
            movement: 0,
        }
    }

    pub fn update(&mut self, phase: EncoderPhase, high: bool) -> Option<DetentDirection> {
        let bit = match phase {
            EncoderPhase::A => 0b10,
            EncoderPhase::B => 0b01,
        };
        let next_state = if high {
            self.state | bit
        } else {
            self.state & !bit
        };

        if next_state == self.state {
            return None;
        }

        let transition = TRANSITIONS[((self.state << 2) | next_state) as usize];
        self.state = next_state;

        if transition == 0 {
            self.movement = 0;
            return None;
        }

        self.movement += transition;
        if self.state != 0b00 && self.state != 0b11 {
            return None;
        }

        let direction = if self.movement >= 2 {
            Some(DetentDirection::Positive)
        } else if self.movement <= -2 {
            Some(DetentDirection::Negative)
        } else {
            None
        };
        self.movement = 0;
        direction
    }
}

#[cfg(test)]
mod tests {
    use super::{DetentDirection, EncoderPhase, HalfStepDecoder};

    #[test]
    fn decodes_both_positive_half_steps() {
        let mut from_low = HalfStepDecoder::new(false, false);
        assert_eq!(from_low.update(EncoderPhase::B, true), None);
        assert_eq!(
            from_low.update(EncoderPhase::A, true),
            Some(DetentDirection::Positive)
        );

        let mut from_high = HalfStepDecoder::new(true, true);
        assert_eq!(from_high.update(EncoderPhase::B, false), None);
        assert_eq!(
            from_high.update(EncoderPhase::A, false),
            Some(DetentDirection::Positive)
        );
    }

    #[test]
    fn decodes_both_negative_half_steps() {
        let mut from_low = HalfStepDecoder::new(false, false);
        assert_eq!(from_low.update(EncoderPhase::A, true), None);
        assert_eq!(
            from_low.update(EncoderPhase::B, true),
            Some(DetentDirection::Negative)
        );

        let mut from_high = HalfStepDecoder::new(true, true);
        assert_eq!(from_high.update(EncoderPhase::A, false), None);
        assert_eq!(
            from_high.update(EncoderPhase::B, false),
            Some(DetentDirection::Negative)
        );
    }

    #[test]
    fn bounce_back_to_the_same_detent_does_not_emit() {
        let mut decoder = HalfStepDecoder::new(false, false);
        assert_eq!(decoder.update(EncoderPhase::B, true), None);
        assert_eq!(decoder.update(EncoderPhase::B, false), None);
        assert_eq!(decoder.update(EncoderPhase::B, true), None);
        assert_eq!(
            decoder.update(EncoderPhase::A, true),
            Some(DetentDirection::Positive)
        );
    }

    #[test]
    fn reversing_a_partial_step_does_not_leak_old_direction() {
        let mut decoder = HalfStepDecoder::new(false, false);
        assert_eq!(decoder.update(EncoderPhase::B, true), None);
        assert_eq!(decoder.update(EncoderPhase::B, false), None);
        assert_eq!(decoder.update(EncoderPhase::A, true), None);
        assert_eq!(
            decoder.update(EncoderPhase::B, true),
            Some(DetentDirection::Negative)
        );
    }
}
