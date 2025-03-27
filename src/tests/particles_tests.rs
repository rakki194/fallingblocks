#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::components::{Particle, Position};
    use crate::particles::spawn_particle;
    use ratatui::style::Color;

    #[test]
    fn test_spawn_particles() {
        // Create test particle at a specific position
        let position = Position { x: 10, y: 10 };
        let color = Color::Red;
        let velocity = (0.5, -0.5);
        let lifetime = 1.0;
        let size = 0.8;

        // Create a test world
        let mut world = crate::tests::test_utils::create_test_world();

        // Spawn a particle
        spawn_particle(&mut world, position, velocity, color, lifetime, size);

        // Verify particle was created
        let particles: Vec<&Particle> = world.query::<&Particle>().iter(&world).collect();

        assert_eq!(particles.len(), 1, "Should create exactly one particle");

        // Check particle properties
        let particle = particles[0];
        assert_eq!(particle.position.x, position.x);
        assert_eq!(particle.position.y, position.y);
        assert_eq!(particle.velocity.0, velocity.0);
        assert_eq!(particle.velocity.1, velocity.1);
        assert_eq!(particle.color, color);
        assert_eq!(particle.lifetime, lifetime);
        assert_eq!(particle.size, size);
    }

    #[test]
    fn test_particle_creation() {
        // Test manual particle creation
        let position = Position { x: 5, y: 7 };
        let velocity = (1.5, -2.0);
        let color = Color::Blue;
        let lifetime = 2.5;
        let size = 1.2;

        let particle = Particle {
            position,
            velocity,
            color,
            lifetime,
            size,
        };

        // Check particle properties
        assert_eq!(particle.position.x, 5);
        assert_eq!(particle.position.y, 7);
        assert_eq!(particle.velocity.0, 1.5);
        assert_eq!(particle.velocity.1, -2.0);
        assert_eq!(particle.color, Color::Blue);
        assert_eq!(particle.lifetime, 2.5);
        assert_eq!(particle.size, 1.2);
    }

    #[test]
    fn test_particle_update() {
        // Create a test world
        let mut world = crate::tests::test_utils::create_test_world();

        // Spawn a test particle
        let initial_position = Position { x: 10, y: 10 };
        let velocity = (1.0, 2.0);
        spawn_particle(
            &mut world,
            initial_position,
            velocity,
            Color::Green,
            1.0,
            1.0,
        );

        // Update particles with a time step
        let delta_seconds = 0.5;
        crate::particles::update_particles(&mut world, delta_seconds);

        // Check particle position was updated
        let particles: Vec<&Particle> = world.query::<&Particle>().iter(&world).collect();
        assert_eq!(particles.len(), 1);

        let updated_particle = particles[0];
        // Position should be updated based on velocity * time
        // Note: The actual update logic may be more complex with friction, etc.
        assert!(
            updated_particle.position.x > initial_position.x,
            "Particle should have moved in x direction"
        );
        assert!(
            updated_particle.position.y > initial_position.y,
            "Particle should have moved in y direction"
        );

        // Lifetime should be reduced
        assert!(updated_particle.lifetime < 1.0, "Lifetime should decrease");
    }
}
