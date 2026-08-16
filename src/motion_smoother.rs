const FILTER_SCALE: i32 = 256;

/// A one-sample low-pass filter for relative pointing reports.
///
/// The state is kept in fixed point so sub-count motion is not discarded. When
/// the sensor stops reporting, the pending state is flushed by the caller at
/// the normal report rate instead of being left in the filter indefinitely.
#[derive(Default)]
pub struct MotionSmoother {
    x_state: i32,
    y_state: i32,
    x_remainder: i32,
    y_remainder: i32,
}

impl MotionSmoother {
    pub const fn new() -> Self {
        Self {
            x_state: 0,
            y_state: 0,
            x_remainder: 0,
            y_remainder: 0,
        }
    }

    pub fn has_pending(&self) -> bool {
        self.x_state != 0 || self.y_state != 0
    }

    pub fn apply(&mut self, x: i16, y: i16) -> (i16, i16) {
        let x = filter_axis(&mut self.x_state, &mut self.x_remainder, i32::from(x));
        let y = filter_axis(&mut self.y_state, &mut self.y_remainder, i32::from(y));
        (x, y)
    }
}

fn filter_axis(state: &mut i32, remainder: &mut i32, input: i32) -> i16 {
    let target = input * FILTER_SCALE;
    // alpha = 1/2: one report of latency at 125 Hz, with a mild reduction of
    // fine hand jitter and no separate gain change for sustained motion.
    *state = (*state + target) / 2;

    if *state == 0 {
        *remainder = 0;
        return 0;
    }

    let numerator = *state + *remainder;
    let output = numerator / FILTER_SCALE;
    *remainder = numerator - output * FILTER_SCALE;
    output.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests {
    use super::MotionSmoother;

    #[test]
    fn smooths_a_single_step_and_flushes_tail() {
        let mut smoother = MotionSmoother::new();
        let mut output = 0_i32;
        assert_eq!(smoother.apply(8, 0), (4, 0));
        output += 4;
        assert!(smoother.has_pending());
        for _ in 0..16 {
            output += i32::from(smoother.apply(0, 0).0);
            if !smoother.has_pending() {
                break;
            }
        }
        assert!(!smoother.has_pending());
        assert!((7..=8).contains(&output));
    }

    #[test]
    fn keeps_sustained_motion_near_the_input() {
        let mut smoother = MotionSmoother::new();
        let mut output = 0_i32;
        for _ in 0..8 {
            output += i32::from(smoother.apply(4, 0).0);
        }
        assert!(output >= 27);
        assert!(output <= 32);
    }
}
