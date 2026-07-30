use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use embassy_time::Timer;
use rmk::core_traits::Runnable;
use rmk::host::KeyboardContext;

pub const CALIBRATION_BLOB_SIZE: usize = 28;
pub const CALIBRATION_MACRO_OFFSET: usize = rmk::MACRO_SPACE_SIZE - CALIBRATION_BLOB_SIZE;

const MAGIC: [u8; 4] = *b"RLC1";
const FORMAT_VERSION: u8 = 1;
const MAX_ABS_COEFFICIENT: i32 = 16_000;
const MIN_ABS_DETERMINANT: i64 = 10_000;

const _: () = assert!(rmk::MACRO_SPACE_SIZE >= CALIBRATION_BLOB_SIZE);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatrixCoefficients {
    pub m00: i32,
    pub m01: i32,
    pub m10: i32,
    pub m11: i32,
}

impl MatrixCoefficients {
    pub const DEFAULT: Self = Self {
        m00: -265,
        m01: 1142,
        m10: -831,
        m11: 562,
    };

    fn is_safe(self) -> bool {
        let values = [self.m00, self.m01, self.m10, self.m11];
        if values
            .iter()
            .any(|value| value.unsigned_abs() > MAX_ABS_COEFFICIENT as u32)
        {
            return false;
        }

        let determinant =
            i64::from(self.m00) * i64::from(self.m11) - i64::from(self.m01) * i64::from(self.m10);
        determinant.unsigned_abs() >= MIN_ABS_DETERMINANT as u64
    }
}

static MATRIX_M00: AtomicI32 = AtomicI32::new(MatrixCoefficients::DEFAULT.m00);
static MATRIX_M01: AtomicI32 = AtomicI32::new(MatrixCoefficients::DEFAULT.m01);
static MATRIX_M10: AtomicI32 = AtomicI32::new(MatrixCoefficients::DEFAULT.m10);
static MATRIX_M11: AtomicI32 = AtomicI32::new(MatrixCoefficients::DEFAULT.m11);
static MATRIX_GENERATION: AtomicU32 = AtomicU32::new(0);

pub fn current_matrix() -> MatrixCoefficients {
    loop {
        let before = MATRIX_GENERATION.load(Ordering::Acquire);
        if before & 1 != 0 {
            core::hint::spin_loop();
            continue;
        }
        let matrix = MatrixCoefficients {
            m00: MATRIX_M00.load(Ordering::Relaxed),
            m01: MATRIX_M01.load(Ordering::Relaxed),
            m10: MATRIX_M10.load(Ordering::Relaxed),
            m11: MATRIX_M11.load(Ordering::Relaxed),
        };
        if MATRIX_GENERATION.load(Ordering::Acquire) == before {
            return matrix;
        }
    }
}

fn apply_matrix(matrix: MatrixCoefficients) {
    MATRIX_GENERATION.fetch_add(1, Ordering::AcqRel);
    MATRIX_M00.store(matrix.m00, Ordering::Relaxed);
    MATRIX_M01.store(matrix.m01, Ordering::Relaxed);
    MATRIX_M10.store(matrix.m10, Ordering::Relaxed);
    MATRIX_M11.store(matrix.m11, Ordering::Relaxed);
    MATRIX_GENERATION.fetch_add(1, Ordering::Release);
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

fn decode_blob(blob: &[u8; CALIBRATION_BLOB_SIZE]) -> Option<MatrixCoefficients> {
    if blob[..4] != MAGIC || blob[4] != FORMAT_VERSION || blob[5] != 0 {
        return None;
    }
    if u16::from_le_bytes([blob[6], blob[7]]) != 1000 {
        return None;
    }
    if u32::from_le_bytes(blob[24..28].try_into().ok()?) != checksum(&blob[..24]) {
        return None;
    }

    let matrix = MatrixCoefficients {
        m00: i32::from_le_bytes(blob[8..12].try_into().ok()?),
        m01: i32::from_le_bytes(blob[12..16].try_into().ok()?),
        m10: i32::from_le_bytes(blob[16..20].try_into().ok()?),
        m11: i32::from_le_bytes(blob[20..24].try_into().ok()?),
    };
    matrix.is_safe().then_some(matrix)
}

fn encode_blob(matrix: MatrixCoefficients) -> [u8; CALIBRATION_BLOB_SIZE] {
    let mut blob = [0u8; CALIBRATION_BLOB_SIZE];
    blob[..4].copy_from_slice(&MAGIC);
    blob[4] = FORMAT_VERSION;
    blob[6..8].copy_from_slice(&1000u16.to_le_bytes());
    blob[8..12].copy_from_slice(&matrix.m00.to_le_bytes());
    blob[12..16].copy_from_slice(&matrix.m01.to_le_bytes());
    blob[16..20].copy_from_slice(&matrix.m10.to_le_bytes());
    blob[20..24].copy_from_slice(&matrix.m11.to_le_bytes());
    let checksum = checksum(&blob[..24]);
    blob[24..28].copy_from_slice(&checksum.to_le_bytes());
    blob
}

pub struct CalibrationConfigWatcher<'a, 'keymap> {
    context: &'a KeyboardContext<'keymap>,
    last_applied: MatrixCoefficients,
}

impl<'a, 'keymap> CalibrationConfigWatcher<'a, 'keymap> {
    pub fn new(context: &'a KeyboardContext<'keymap>) -> Self {
        Self {
            context,
            last_applied: MatrixCoefficients::DEFAULT,
        }
    }

    fn read_blob(&self) -> [u8; CALIBRATION_BLOB_SIZE] {
        let mut blob = [0u8; CALIBRATION_BLOB_SIZE];
        self.context
            .read_macro_buffer(CALIBRATION_MACRO_OFFSET, &mut blob);
        blob
    }

    pub async fn initialize(&mut self) {
        let blob = self.read_blob();
        let matrix = if let Some(matrix) = decode_blob(&blob) {
            matrix
        } else {
            let matrix = MatrixCoefficients::DEFAULT;
            self.context
                .write_macro_buffer(CALIBRATION_MACRO_OFFSET, &encode_blob(matrix))
                .await;
            matrix
        };
        apply_matrix(matrix);
        self.last_applied = matrix;
    }

    fn refresh(&mut self) {
        if let Some(matrix) = decode_blob(&self.read_blob())
            && matrix != self.last_applied
        {
            apply_matrix(matrix);
            self.last_applied = matrix;
        }
    }
}

impl Runnable for CalibrationConfigWatcher<'_, '_> {
    async fn run(&mut self) -> ! {
        loop {
            self.refresh();
            Timer::after_millis(25).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_round_trip() {
        let matrix = MatrixCoefficients::DEFAULT;
        assert_eq!(decode_blob(&encode_blob(matrix)), Some(matrix));
    }

    #[test]
    fn default_blob_matches_browser_wire_format() {
        assert_eq!(
            encode_blob(MatrixCoefficients::DEFAULT),
            [
                0x52, 0x4c, 0x43, 0x31, 0x01, 0x00, 0xe8, 0x03, 0xf7, 0xfe, 0xff, 0xff, 0x76, 0x04,
                0x00, 0x00, 0xc1, 0xfc, 0xff, 0xff, 0x32, 0x02, 0x00, 0x00, 0x8d, 0x69, 0x15, 0x6a,
            ]
        );
    }

    #[test]
    fn rejects_corruption() {
        let mut blob = encode_blob(MatrixCoefficients::DEFAULT);
        blob[10] ^= 0x80;
        assert_eq!(decode_blob(&blob), None);
    }

    #[test]
    fn rejects_dangerous_matrix() {
        assert!(
            !MatrixCoefficients {
                m00: 0,
                m01: 0,
                m10: 0,
                m11: 0,
            }
            .is_safe()
        );
    }
}
