use crate::calibration_config::MatrixCoefficients;

pub const MATRIX_SCALE: i64 = 1000;

const NORMALIZATION_SCALE: u64 = MATRIX_SCALE as u64;

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

    pub fn apply(&mut self, raw_x: i16, raw_y: i16, matrix: MatrixCoefficients) -> (i16, i16) {
        let output_x_numerator =
            i64::from(raw_x) * i64::from(matrix.m00) + i64::from(raw_y) * i64::from(matrix.m01);
        let output_y_numerator =
            i64::from(raw_x) * i64::from(matrix.m10) + i64::from(raw_y) * i64::from(matrix.m11);

        let raw_x = i64::from(raw_x);
        let raw_y = i64::from(raw_y);
        let raw_length = integer_sqrt(
            (raw_x * raw_x + raw_y * raw_y) as u64 * NORMALIZATION_SCALE * NORMALIZATION_SCALE,
        );
        let transformed_length = integer_sqrt(
            output_x_numerator.unsigned_abs() * output_x_numerator.unsigned_abs()
                + output_y_numerator.unsigned_abs() * output_y_numerator.unsigned_abs(),
        );

        if raw_length == 0 || transformed_length == 0 {
            return (0, 0);
        }

        // Rescale the transformed vector to the original vector length while
        // retaining the calibrated direction.  The extra MATRIX_SCALE keeps
        // the existing fractional remainder mechanism intact.
        let normalized_x_numerator = i128::from(output_x_numerator) * i128::from(raw_length)
            / i128::from(transformed_length);
        let normalized_y_numerator = i128::from(output_y_numerator) * i128::from(raw_length)
            / i128::from(transformed_length);

        let output_x_numerator = normalized_x_numerator + i128::from(self.output_x_remainder);
        let output_y_numerator = normalized_y_numerator + i128::from(self.output_y_remainder);
        let output_x = output_x_numerator / i128::from(MATRIX_SCALE);
        let output_y = output_y_numerator / i128::from(MATRIX_SCALE);

        self.output_x_remainder = (output_x_numerator - output_x * i128::from(MATRIX_SCALE)) as i64;
        self.output_y_remainder = (output_y_numerator - output_y * i128::from(MATRIX_SCALE)) as i64;

        (
            output_x.clamp(i128::from(i16::MIN), i128::from(i16::MAX)) as i16,
            output_y.clamp(i128::from(i16::MIN), i128::from(i16::MAX)) as i16,
        )
    }
}

fn integer_sqrt(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    let bit_length = 64 - value.leading_zeros();
    let mut estimate = 1_u64 << ((bit_length + 1) / 2);
    loop {
        let next = (estimate + value / estimate) / 2;
        if next >= estimate {
            return estimate;
        }
        estimate = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_sqrt_rounds_down() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(2), 1);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(15), 3);
    }

    #[test]
    fn applies_calibrated_matrix() {
        let mut transform = TrackballTransform::new();
        let (x, y) = transform.apply(1000, 0, MatrixCoefficients::DEFAULT);
        let length_squared = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y);
        assert!((995_000..=1_005_000).contains(&length_squared));

        let (x, y) = transform.apply(0, 1000, MatrixCoefficients::DEFAULT);
        let length_squared = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y);
        assert!((995_000..=1_005_000).contains(&length_squared));
    }

    #[test]
    fn retains_fractional_motion() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(1, 0, MatrixCoefficients::DEFAULT), (0, 0));
        assert_eq!(transform.apply(1, 0, MatrixCoefficients::DEFAULT), (0, -1));
        assert_eq!(transform.apply(1, 0, MatrixCoefficients::DEFAULT), (0, -1));
        assert_eq!(transform.apply(1, 0, MatrixCoefficients::DEFAULT), (-1, -1));
    }

    #[test]
    fn uses_truncation_toward_zero() {
        let mut transform = TrackballTransform::new();
        assert_eq!(transform.apply(1, 0, MatrixCoefficients::DEFAULT), (0, 0));
        assert_eq!(transform.apply(-1, 0, MatrixCoefficients::DEFAULT), (0, 0));
    }

    #[test]
    fn clamps_to_i16() {
        let mut transform = TrackballTransform::new();
        let (x, y) = transform.apply(i16::MIN, i16::MAX, MatrixCoefficients::DEFAULT);
        assert_eq!(x, i16::MAX);
        assert!(y < i16::MAX);
    }
}
