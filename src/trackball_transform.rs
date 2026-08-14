use crate::calibration_config::MatrixCoefficients;

pub const MATRIX_SCALE: i64 = 1000;
const LENGTH_SCALE: i64 = 64;

fn integer_sqrt(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    let bit_count = 64 - value.leading_zeros();
    let mut lower = 0;
    let mut upper = 1u64 << ((bit_count + 1) / 2);

    while lower + 1 < upper {
        let middle = (lower + upper) / 2;
        if middle <= value / middle {
            lower = middle;
        } else {
            upper = middle;
        }
    }

    lower
}

fn normalize_component(component: i64, raw_length_scaled: u64, transformed_length: u64) -> i64 {
    let numerator =
        i128::from(component) * i128::from(raw_length_scaled) * i128::from(MATRIX_SCALE);
    let denominator = i128::from(transformed_length) * i128::from(LENGTH_SCALE);
    (numerator / denominator) as i64
}

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
        let transformed_x_numerator =
            i64::from(raw_x) * i64::from(matrix.m00) + i64::from(raw_y) * i64::from(matrix.m01);
        let transformed_y_numerator =
            i64::from(raw_x) * i64::from(matrix.m10) + i64::from(raw_y) * i64::from(matrix.m11);

        let raw_length_squared = {
            let raw_x = i64::from(raw_x).unsigned_abs();
            let raw_y = i64::from(raw_y).unsigned_abs();
            raw_x * raw_x + raw_y * raw_y
        };
        let raw_length_scaled =
            integer_sqrt(raw_length_squared * (LENGTH_SCALE as u64) * (LENGTH_SCALE as u64));
        let transformed_length_squared = transformed_x_numerator
            .unsigned_abs()
            .saturating_mul(transformed_x_numerator.unsigned_abs())
            .saturating_add(
                transformed_y_numerator
                    .unsigned_abs()
                    .saturating_mul(transformed_y_numerator.unsigned_abs()),
            );
        let transformed_length = integer_sqrt(transformed_length_squared);

        let (output_x_numerator, output_y_numerator) = if transformed_length == 0 {
            (0, 0)
        } else {
            (
                normalize_component(
                    transformed_x_numerator,
                    raw_length_scaled,
                    transformed_length,
                ),
                normalize_component(
                    transformed_y_numerator,
                    raw_length_scaled,
                    transformed_length,
                ),
            )
        };

        let output_x_numerator = output_x_numerator + self.output_x_remainder;
        let output_y_numerator = output_y_numerator + self.output_y_remainder;

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
        let (x, y) = transform.apply(1000, 0, MatrixCoefficients::DEFAULT);
        let length_squared = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y);
        assert!(x < 0 && y < 0);
        assert!((length_squared - 1_000_000).abs() < 5_000);

        let mut transform = TrackballTransform::new();
        let (x, y) = transform.apply(0, 1000, MatrixCoefficients::DEFAULT);
        let length_squared = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y);
        assert!(x > 0 && y > 0);
        assert!((length_squared - 1_000_000).abs() < 5_000);
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
        assert_eq!(
            transform.apply(
                i16::MAX,
                i16::MAX,
                MatrixCoefficients {
                    m00: 1000,
                    m01: 1000,
                    m10: -1000,
                    m11: 1000,
                },
            ),
            (i16::MAX, 0)
        );
    }
}
