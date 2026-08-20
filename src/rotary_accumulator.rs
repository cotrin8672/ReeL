pub struct DetentAccumulator {
    counts_per_detent: i32,
    remainder: i32,
}

impl DetentAccumulator {
    pub const fn new(counts_per_detent: i16) -> Self {
        assert!(counts_per_detent > 0);
        Self {
            counts_per_detent: counts_per_detent as i32,
            remainder: 0,
        }
    }

    /// Add raw quadrature counts and return the number of completed detents.
    ///
    /// Positive and negative partial movements cancel each other, so contact
    /// bounce does not become an encoder event. Any incomplete detent is kept
    /// for the next QDEC sample.
    pub fn push(&mut self, counts: i16) -> i16 {
        self.remainder += i32::from(counts);
        let detents = self.remainder / self.counts_per_detent;
        self.remainder %= self.counts_per_detent;
        detents as i16
    }
}

#[cfg(test)]
mod tests {
    use super::DetentAccumulator;

    #[test]
    fn two_quadrature_counts_make_one_detent() {
        let mut accumulator = DetentAccumulator::new(2);

        assert_eq!(accumulator.push(1), 0);
        assert_eq!(accumulator.push(1), 1);
    }

    #[test]
    fn negative_counts_preserve_direction() {
        let mut accumulator = DetentAccumulator::new(2);

        assert_eq!(accumulator.push(-1), 0);
        assert_eq!(accumulator.push(-1), -1);
    }

    #[test]
    fn opposite_bounce_cancels_without_an_event() {
        let mut accumulator = DetentAccumulator::new(2);

        assert_eq!(accumulator.push(1), 0);
        assert_eq!(accumulator.push(-1), 0);
        assert_eq!(accumulator.push(1), 0);
        assert_eq!(accumulator.push(1), 1);
    }

    #[test]
    fn burst_keeps_incomplete_detent_for_the_next_sample() {
        let mut accumulator = DetentAccumulator::new(2);

        assert_eq!(accumulator.push(5), 2);
        assert_eq!(accumulator.push(1), 1);
    }
}
