#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::Time;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_time_new() {
        let time = Time::new();
        assert_eq!(time.delta, Duration::default());
    }

    #[test]
    fn test_time_update() {
        let mut time = Time::new();

        // Initial delta should be zero
        assert_eq!(time.delta, Duration::default());

        // Sleep to allow some time to pass
        sleep(Duration::from_millis(10));

        // Update should change the delta
        time.update();
        assert!(time.delta > Duration::default());
    }

    #[test]
    fn test_delta_seconds() {
        let mut time = Time::new();

        // Sleep for a known duration
        let sleep_duration = Duration::from_millis(10);
        sleep(sleep_duration);

        // Update the time
        time.update();

        // Check delta_seconds is approximately correct (with a small margin of error)
        let expected = sleep_duration.as_secs_f32();
        let actual = time.delta_seconds();

        // Allow a small margin for timing discrepancies
        assert!((actual - expected).abs() < 0.1);
    }
}
