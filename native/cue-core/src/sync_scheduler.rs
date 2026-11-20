//! Monotonic scheduling policy; no timers, credentials or network side effects.
use std::time::Duration;

#[derive(Clone, Copy)]
pub(crate) enum Outcome {
    Success,
    Retry,
    Rejected,
}

#[derive(Default)]
pub(crate) struct Scheduler {
    next_ticket: u64,
    in_flight: Option<u64>,
    retry_at: Duration,
    failures: u8,
    paused: bool,
}
impl Scheduler {
    pub(crate) fn begin(&mut self, now: Duration) -> Option<u64> {
        if self.paused || self.in_flight.is_some() || now < self.retry_at {
            return None;
        }
        self.next_ticket = self.next_ticket.checked_add(1)?;
        self.in_flight = Some(self.next_ticket);
        self.in_flight
    }
    pub(crate) fn finish(&mut self, ticket: u64, now: Duration, outcome: Outcome) -> bool {
        if self.in_flight != Some(ticket) {
            return false;
        }
        self.in_flight = None;
        match outcome {
            Outcome::Success => {
                self.failures = 0;
                self.retry_at = now;
            }
            Outcome::Retry => {
                let seconds = [1, 2, 4, 8, 16, 30][usize::from(self.failures.min(5))];
                self.failures = self.failures.saturating_add(1);
                self.retry_at = now.saturating_add(Duration::from_secs(seconds));
            }
            Outcome::Rejected => self.paused = true,
        }
        true
    }
    pub(crate) fn retry_now(&mut self, now: Duration) {
        if !self.paused && self.in_flight.is_none() {
            self.failures = 0;
            self.retry_at = now;
        }
    }
    /// Owner retirement or successful reauthentication invalidates old callbacks.
    /// The ticket counter intentionally survives, so old replies cannot match.
    pub(crate) fn invalidate(&mut self, now: Duration) {
        self.in_flight = None;
        self.failures = 0;
        self.paused = false;
        self.retry_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_flight_backoff_caps_and_success_resets_delay() {
        let mut scheduler = Scheduler::default();
        let mut now = Duration::ZERO;
        for seconds in [1, 2, 4, 8, 16, 30, 30] {
            let ticket = scheduler.begin(now).unwrap();
            assert!(scheduler.begin(now).is_none());
            scheduler.retry_now(now);
            assert!(
                scheduler.begin(now).is_none(),
                "Explicit retry must not overlap a live request"
            );
            assert!(scheduler.finish(ticket, now, Outcome::Retry));
            now += Duration::from_secs(seconds);
            assert!(scheduler.begin(now - Duration::from_nanos(1)).is_none());
        }
        let ticket = scheduler.begin(now).unwrap();
        assert!(scheduler.finish(ticket, now, Outcome::Success));
        let ticket = scheduler.begin(now).unwrap();
        assert!(scheduler.finish(ticket, now, Outcome::Retry));
        assert!(scheduler.begin(now + Duration::from_millis(999)).is_none());
        assert!(scheduler.begin(now + Duration::from_secs(1)).is_some());
    }
    #[test]
    fn rejection_pauses_and_invalidated_completions_cannot_release_new_flight() {
        let mut scheduler = Scheduler::default();
        let zero = Duration::ZERO;
        let old = scheduler.begin(zero).unwrap();
        scheduler.invalidate(zero);
        let current = scheduler.begin(zero).unwrap();
        assert_ne!(current, old);
        assert!(!scheduler.finish(old, zero, Outcome::Success));
        assert!(scheduler.begin(zero).is_none());
        assert!(scheduler.finish(current, zero, Outcome::Rejected));
        assert!(scheduler.begin(Duration::from_secs(3600)).is_none());
        scheduler.retry_now(Duration::from_secs(3600));
        assert!(
            scheduler.begin(Duration::from_secs(3600)).is_none(),
            "Retry cannot bypass reauthentication"
        );
        scheduler.invalidate(Duration::from_secs(3600));
        assert!(scheduler.begin(Duration::from_secs(3600)).is_some());
    }
}
