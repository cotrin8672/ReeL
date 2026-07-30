pub const MATRIX_SCALE: i64 = 1000;
pub const MATRIX_M00: i64 = -265;
pub const MATRIX_M01: i64 = 1142;
pub const MATRIX_M10: i64 = -831;
pub const MATRIX_M11: i64 = 562;

#[derive(Default)]
pub struct TrackballTransform {
    output_x_remainder: i64,
    output_y_remainder: i64,
}

impl TrackballTransform {
    pub const fn new() -> Self {
        Self {
            output_x_remainder: 0,
            output_y_remainder: 0,
        }
    }

    pub fn apply(&mut self, raw_x: i16, raw_y: i16) -> (i16, i16) {
        let output_x_numerator =
            i64::from(raw_x) * MATRIX_M00 + i64::from(raw_y) * MATRIX_M01 + self.output_x_remainder;
        let output_y_numerator =
            i64::from(raw_x) * MATRIX_M10 + i64::from(raw_y) * MATRIX_M11 + self.output_y_remainder;

        let output_x = output_x_numerator / MATRIX_SCALE;
        let output_y = output_y_numerator / MATRIX_SCALE;

        self.output_x_remainder = output_x_numerator - output_x * MATRIX_SCALE;
        self.output_y_remainder = output_y_numerator - output_y * MATRIX_SCALE;

        (
            output_x.clamp(i16::MIN as i64, i16::MAX as i64) as i16,
            output_y.clamp(i16::MIN as i64, i16::MAX as i64) as i16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_calibrated_matrix() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(1000, 0), (-265, -831));
        assert_eq!(transform.apply(0, 1000), (1142, 562));
    }

    #[test]
    fn retains_fractional_motion() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(1, 0), (0, 0));
        assert_eq!(transform.apply(1, 0), (0, -1));
        assert_eq!(transform.apply(1, 0), (0, -1));
        assert_eq!(transform.apply(1, 0), (-1, -1));
    }

    #[test]
    fn uses_truncation_toward_zero_like_the_zmk_implementation() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(1, 0), (0, 0));
        assert_eq!(transform.apply(-1, 0), (0, 0));
    }

    #[test]
    fn clamps_to_i16() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(i16::MIN, i16::MAX), (i16::MAX, i16::MAX));
    }
}
