#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::components::ScreenShake;
    use crate::screenshake::{trigger_screen_shake, update_screen_shake};
    use crate::tests::test_utils::create_test_world;

    #[test]
    fn test_default_screenshake() {
        let screenshake = ScreenShake::default();

        // Check default values
        assert_eq!(screenshake.duration, 0.0);
        assert_eq!(screenshake.intensity, 0.0);
        assert_eq!(screenshake.current_offset, (0, 0));
        assert!(!screenshake.is_active);
        assert!(!screenshake.horizontal_bias);
    }

    #[test]
    fn test_trigger_screenshake() {
        // Create a test world
        let mut world = create_test_world();

        // Initialize the screenshake resource
        world.insert_resource(ScreenShake::default());

        // Trigger screenshake
        let intensity = 3.0;
        let duration = 0.75;
        trigger_screen_shake(&mut world, intensity, duration);

        // Verify the screenshake properties
        let screenshake = world.resource::<ScreenShake>();
        assert_eq!(screenshake.duration, duration);
        assert_eq!(screenshake.intensity, intensity);
        assert!(screenshake.is_active);
    }

    #[test]
    fn test_update_screenshake() {
        // Create a test world
        let mut world = create_test_world();

        // Initialize with active screenshake
        let mut screenshake = ScreenShake::default();
        screenshake.intensity = 5.0;
        screenshake.duration = 1.0;
        screenshake.is_active = true;
        world.insert_resource(screenshake);

        // Get initial values
        let initial_shake = world.resource::<ScreenShake>().clone();

        // Update with a time step
        let time_step = 0.1;
        update_screen_shake(&mut world, time_step);

        // Get updated values
        let updated_shake = world.resource::<ScreenShake>().clone();

        // Check that duration decreased
        assert!(updated_shake.duration < initial_shake.duration);
        assert_eq!(updated_shake.duration, 0.9); // 1.0 - 0.1

        // Check that offset was updated (should be non-zero during active shake)
        assert!(
            updated_shake.current_offset != (0, 0),
            "Offset should be updated during active shake"
        );
    }

    #[test]
    fn test_screenshake_expiration() {
        // Create a test world
        let mut world = create_test_world();

        // Initialize with almost expired screenshake
        let mut screenshake = ScreenShake::default();
        screenshake.intensity = 5.0;
        screenshake.duration = 0.05;
        screenshake.is_active = true;
        screenshake.current_offset = (3, 3); // Some non-zero offset
        world.insert_resource(screenshake);

        // Update with a time step longer than the duration
        let time_step = 0.1;
        update_screen_shake(&mut world, time_step);

        // Get updated values
        let updated_shake = world.resource::<ScreenShake>().clone();

        // Check that duration is now zero or negative
        assert!(updated_shake.duration <= 0.0);

        // Check that values were reset
        assert_eq!(updated_shake.current_offset, (0, 0));
        assert!(!updated_shake.is_active);
        assert_eq!(updated_shake.intensity, 0.0);
    }
}
