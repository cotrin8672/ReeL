const GAIN_SCALE: i32 = 256;

/// Independent sensitivity control. The default is unity; the intentional
/// 800-CPI sensor setting is not changed by this feature.
pub const TRACKBALL_GAIN_NUMERATOR: i32 = GAIN_SCALE;

#[derive(Default)]
pub struct MotionGain {
    numerator: i32,
    x_remainder: i64,
    y_remainder: i64,
}

impl MotionGain {
    pub const fn new() -> Self {
        Self::with_gain(TRACKBALL_GAIN_NUMERATOR)
    }

    pub const fn with_gain(numerator: i32) -> Self {
        Self {
            numerator,
            x_remainder: 0,
            y_remainder: 0,
        }
    }

    pub fn apply(&mut self, x: i16, y: i16) -> (i16, i16) {
        let x_numerator = i64::from(x) * i64::from(self.numerator) + self.x_remainder;
        let y_numerator = i64::from(y) * i64::from(self.numerator) + self.y_remainder;
        let x = x_numerator / i64::from(GAIN_SCALE);
        let y = y_numerator / i64::from(GAIN_SCALE);

        self.x_remainder = x_numerator - x * i64::from(GAIN_SCALE);
        self.y_remainder = y_numerator - y * i64::from(GAIN_SCALE);

        (
            x.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
            y.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MotionGain;

    #[test]
    fn applies_one_and_a_half_gain_with_remainders() {
        let mut gain = MotionGain::with_gain(384);
        let mut total = 0_i32;
        for _ in 0..256 {
            total += i32::from(gain.apply(1, 0).0);
        }
        assert!((383..=385).contains(&total));
    }

    #[test]
    fn default_gain_is_unity() {
        let mut gain = MotionGain::new();
        assert_eq!(gain.apply(12, -7), (12, -7));
    }
}
