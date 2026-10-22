use std::time::Duration;

/// Visual position follows the logical row target without queuing animations per input event.
#[derive(Default)]
pub(super) struct ScrollMotion {
    position: f64,
    target: f64,
}

impl ScrollMotion {
    pub(super) fn position(&self) -> f64 {
        self.position
    }

    pub(super) fn active(&self) -> bool {
        self.position != self.target
    }

    pub(super) fn retarget(&mut self, row: usize, jump_threshold: usize) {
        let next = row as f64;
        if (next - self.target).abs() > jump_threshold as f64 {
            self.jump(row);
        } else {
            self.target = next;
        }
    }

    pub(super) fn jump(&mut self, row: usize) {
        self.position = row as f64;
        self.target = self.position;
    }

    pub(super) fn advance(&mut self, elapsed: Duration) -> bool {
        if !self.active() || elapsed.is_zero() {
            return false;
        }
        let previous = self.position;
        // A short, time-based ease-out settles a one-row input in about 140ms at 60Hz.
        let response = 1.0 - (-elapsed.as_secs_f64() / 0.028).exp();
        self.position += (self.target - self.position) * response;
        if (self.target - self.position).abs() < 0.01 {
            self.position = self.target;
        }
        self.position != previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_scroll_has_sub_row_frames_and_converges_without_overshoot() {
        let mut motion = ScrollMotion::default();
        motion.retarget(1, 20);
        let mut previous = 0.0;
        for _ in 0..10 {
            motion.advance(Duration::from_micros(16_667));
            assert!(motion.position() >= previous && motion.position() <= 1.0);
            previous = motion.position();
        }
        assert_eq!(motion.position(), 1.0);
        assert!(!motion.active());
        motion.jump(0);
        motion.retarget(1, 20);
        motion.advance(Duration::from_micros(16_667));
        assert!(motion.position() > 0.0 && motion.position() < 1.0);
    }

    #[test]
    fn new_input_replaces_the_destination_and_reversal_responds_next_frame() {
        let mut motion = ScrollMotion::default();
        motion.retarget(5, 20);
        motion.advance(Duration::from_millis(16));
        let previous = motion.position();
        motion.retarget(0, 20);
        motion.advance(Duration::from_millis(16));
        assert!(motion.position() >= 0.0 && motion.position() < previous);
        motion.retarget(12, 20);
        motion.advance(Duration::from_secs(1));
        assert_eq!(motion.position(), 12.0);
    }

    #[test]
    fn page_sized_jumps_do_not_animate_through_intermediate_pages() {
        let mut motion = ScrollMotion::default();
        motion.retarget(40, 20);
        assert_eq!(motion.position(), 40.0);
        assert!(!motion.active());
        motion.retarget(0, 20);
        assert_eq!(motion.position(), 0.0);
    }

    #[test]
    fn motion_is_consistent_across_frame_rates() {
        let mut fast = ScrollMotion::default();
        let mut slow = ScrollMotion::default();
        fast.retarget(10, 20);
        slow.retarget(10, 20);
        for _ in 0..8 {
            fast.advance(Duration::from_millis(8));
        }
        for _ in 0..4 {
            slow.advance(Duration::from_millis(16));
        }
        assert!((fast.position() - slow.position()).abs() < 0.000001);
    }
}
