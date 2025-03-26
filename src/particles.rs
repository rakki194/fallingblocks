use bevy_ecs::prelude::*;
use log::{debug, trace};
use ratatui::style::Color;

use crate::components::{Particle, Position, ScreenShake, Tetromino};

pub fn spawn_lock_particles(world: &mut World, position: &Position, tetromino: &Tetromino) {
    // Clear any existing coyote time particles first
    clear_coyote_time_particles(world);

    debug!(
        "Spawning lock particles at position ({}, {})",
        position.x, position.y
    );

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();
    let color = tetromino.tetromino_type.get_color();

    const PARTICLES_PER_BLOCK: usize = 8;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity (with upward bias for collision effect)
            let vx = (fastrand::f32() - 0.5) * 4.0;
            let vy = (fastrand::f32() - 0.7) * 4.0; // Bias upward

            spawn_particle(
                world,
                block_pos.clone(),
                (vx, vy),
                color,
                fastrand::f32() * 0.8 + 0.2, // lifetime: 0.2 to 1.0 seconds
                fastrand::f32() * 0.8 + 0.2,
            ); // size: 0.2 to 1.0
        }
    }

    // Trigger screen shake effect
    trigger_screen_shake(world, 0.8, 0.3);
}

pub fn spawn_rotation_particles(world: &mut World, position: &Position, tetromino: &Tetromino) {
    trace!("Spawning rotation particles");

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();
    let color = tetromino.tetromino_type.get_color();

    const PARTICLES_PER_BLOCK: usize = 3;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity in all directions
            let vx = (fastrand::f32() - 0.5) * 2.0;
            let vy = (fastrand::f32() - 0.5) * 2.0;

            spawn_particle(
                world,
                block_pos.clone(),
                (vx, vy),
                color,
                fastrand::f32() * 0.4 + 0.1, // lifetime: 0.1 to 0.5 seconds
                fastrand::f32() * 0.6 + 0.2,
            ); // size: 0.2 to 0.8
        }
    }
}

pub fn spawn_coyote_time_particles(world: &mut World, position: &Position, tetromino: &Tetromino) {
    // Only spawn particles if we haven't already spawned them for this position
    let already_has_particles = world
        .query::<&Particle>()
        .iter(world)
        .any(|p| p.position.x == position.x && p.position.y == position.y);

    if already_has_particles {
        return;
    }

    debug!(
        "Spawning coyote time particles at position ({}, {})",
        position.x, position.y
    );

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();

    const PARTICLES_PER_BLOCK: usize = 12;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity with a more scattered pattern
            let vx = (fastrand::f32() - 0.5) * 5.0;
            let vy = (fastrand::f32() - 0.5) * 5.0;

            spawn_particle(
                world,
                block_pos.clone(),
                (vx, vy),
                Color::White,
                fastrand::f32() * 0.5 + 0.2, // lifetime: 0.2 to 0.7 seconds
                fastrand::f32() * 0.9 + 0.3,
            ); // size: 0.3 to 1.2
        }
    }

    // Add screen shake with less intensity
    trigger_screen_shake(world, 1.0, 0.1);
}

pub fn spawn_perfect_clear_particles(world: &mut World, board_width: usize, board_height: usize) {
    // Create a burst of particles across the entire bottom of the board
    for x in 0..board_width {
        let particle_pos = Position {
            x: x as i32,
            y: board_height as i32 - 1,
        };

        // Spawn extra particles for impressive clears
        for _ in 0..20 {
            let vx = (fastrand::f32() - 0.5) * 10.0;
            let vy = (fastrand::f32() - 0.8) * 10.0; // Bias upward

            spawn_particle(
                world,
                particle_pos.clone(),
                (vx, vy),
                Color::Yellow,
                fastrand::f32() * 1.2 + 0.5, // lifetime: 0.5 to 1.7 seconds
                fastrand::f32() * 1.5 + 0.5,
            ); // size: 0.5 to 2.0
        }
    }

    // Add intense screen shake for perfect clears
    trigger_screen_shake(world, 5.0, 0.8);
}

pub fn update_particles(world: &mut World, delta_seconds: f32) {
    // First update all particle lifetimes and collect entities to despawn
    let mut entities_to_despawn = Vec::new();

    // Update lifetimes and collect expired particles
    for (entity, mut particle) in world.query::<(Entity, &mut Particle)>().iter_mut(world) {
        // Reduce lifetime
        particle.lifetime -= delta_seconds;

        if particle.lifetime <= 0.0 {
            entities_to_despawn.push(entity);
        }
    }

    // Remove expired particles
    for entity in entities_to_despawn {
        world.despawn(entity);
    }

    // Update remaining particles
    for (_, mut particle) in world.query::<(Entity, &mut Particle)>().iter_mut(world) {
        // Update position based on velocity
        particle.position.x =
            (particle.position.x as f32 + particle.velocity.0 * delta_seconds) as i32;
        particle.position.y =
            (particle.position.y as f32 + particle.velocity.1 * delta_seconds) as i32;

        // Slow down velocity over time (friction)
        particle.velocity.0 *= 0.95;
        particle.velocity.1 *= 0.95;

        // Add some gravity
        particle.velocity.1 += delta_seconds * 1.0;
    }

    update_screen_shake(world, delta_seconds);
}

// Helper function to spawn a single particle
fn spawn_particle(
    world: &mut World,
    position: Position,
    velocity: (f32, f32),
    color: Color,
    lifetime: f32,
    size: f32,
) {
    world.spawn(Particle {
        position,
        velocity,
        color,
        lifetime,
        size,
    });
}

// Helper function to trigger screen shake
fn trigger_screen_shake(world: &mut World, intensity: f32, duration: f32) {
    let mut screen_shake = world.resource_mut::<ScreenShake>();
    screen_shake.intensity = intensity;
    screen_shake.duration = duration;
    trace!("Screen shake triggered with intensity {}", intensity);
}

// Helper function to update screen shake
fn update_screen_shake(world: &mut World, delta_seconds: f32) {
    let mut screen_shake = world.resource_mut::<ScreenShake>();
    if screen_shake.duration > 0.0 {
        screen_shake.duration -= delta_seconds;

        if screen_shake.duration <= 0.0 {
            // Reset shake when duration expires
            screen_shake.intensity = 0.0;
            screen_shake.current_offset = (0, 0);
        } else {
            // Calculate random shake offset based on intensity
            let intensity = screen_shake.intensity * (screen_shake.duration / 0.3); // Fade out
            let max_offset = (intensity * 2.0) as i16;

            screen_shake.current_offset = (
                (fastrand::i16(0..=max_offset) - max_offset / 2),
                (fastrand::i16(0..=max_offset) - max_offset / 2),
            );
        }
    }
}

// Helper function to clear coyote time particles
fn clear_coyote_time_particles(world: &mut World) {
    let particles_to_remove: Vec<Entity> = world
        .query::<(Entity, &Particle)>()
        .iter(world)
        .filter(|(_, p)| p.color == Color::White) // Coyote time particles are white
        .map(|(e, _)| e)
        .collect();

    for entity in particles_to_remove {
        world.despawn(entity);
    }
}
