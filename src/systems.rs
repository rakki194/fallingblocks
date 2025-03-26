use bevy_ecs::prelude::*;
use log::{debug, info, trace};

use crate::components::{
    Board, CoyoteTime, GameState, Ghost, Input, Particle, Position, ScreenShake, Tetromino,
    TetrominoType,
};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::particles;

pub fn spawn_tetromino(world: &mut World) {
    // Make sure input state is clear when spawning a new tetromino
    // While preserving the hard_drop_released flag
    if let Some(mut input) = world.get_resource_mut::<Input>() {
        // Save the hard_drop_released status
        let was_hard_drop_released = input.hard_drop_released;

        // Reset input state
        *input = Input::default();

        // Restore the hard_drop_released status
        input.hard_drop_released = was_hard_drop_released;
    }

    let tetromino_type = TetrominoType::random();
    let tetromino = Tetromino::new(tetromino_type);

    // Start position at the top center of the board
    let position = Position {
        x: (BOARD_WIDTH / 2) as i32,
        y: 0,
    };

    // Check if spawn position is valid
    let board = world.resource::<Board>();
    if !board.is_valid_position(&position, &tetromino) {
        // Game over if we can't spawn a new tetromino
        let mut game_state = world.resource_mut::<GameState>();
        game_state.game_over = true;
        return;
    }

    // Create the ghost piece at the same initial position
    let ghost = Ghost {
        position: position.clone(),
    };

    // Spawn the tetromino entity with a ghost
    world.spawn((tetromino, position, ghost));
}

// Helper function to check if a tetromino can continue falling
fn can_continue_falling(world: &mut World, position: &Position, tetromino: &Tetromino) -> bool {
    let new_position = Position {
        x: position.x,
        y: position.y + 1,
    };
    let board = world.resource::<Board>();
    board.is_valid_position(&new_position, tetromino)
}

pub fn input_system(world: &mut World) {
    // Clone resources to avoid borrowing issues
    let input = world.resource::<Input>().clone();
    let screen_shake = world.resource::<ScreenShake>().clone();

    // Check if screen shake is active
    if screen_shake.is_active {
        // If screen shake is active, ignore inputs
        return;
    }

    // Get coyote time status
    let coyote_time_active = {
        let game_state = world.resource::<GameState>();
        game_state.coyote_time_active
    };

    // First, check if there's an active tetromino
    let has_active_tetromino;
    {
        let mut query = world.query::<(&Tetromino, &Position)>();
        has_active_tetromino = !query.iter(world).collect::<Vec<_>>().is_empty();
    }

    if !has_active_tetromino {
        return;
    }

    // Handle hard drop separately
    if input.hard_drop {
        handle_hard_drop(world);
        return;
    }

    // Get the active tetromino
    let mut entity_id = None;
    let mut tetromino_clone = None;
    let mut position_clone = None;

    {
        let mut query = world.query::<(Entity, &Tetromino, &Position, &Ghost)>();
        for (entity, tetromino, position, _) in query.iter(world) {
            entity_id = Some(entity);
            tetromino_clone = Some(tetromino.clone());
            position_clone = Some(position.clone());
            break; // Only process the first tetromino
        }
    }

    // If no tetromino found, exit early
    let (entity, tetromino, position) = match (entity_id, tetromino_clone, position_clone) {
        (Some(e), Some(t), Some(p)) => (e, t, p),
        _ => return,
    };

    // Handle horizontal movement
    if input.left || input.right {
        let dx = if input.left { -1 } else { 1 };
        let new_position = Position {
            x: position.x + dx,
            y: position.y,
        };

        // Check if the move is valid
        let can_move = {
            let board = world.resource::<Board>();
            board.is_valid_position(&new_position, &tetromino)
        };

        if can_move {
            // Also check if piece can still move down
            let can_move_down = {
                let down_pos = Position {
                    x: new_position.x,
                    y: new_position.y + 1,
                };
                let board = world.resource::<Board>();
                board.is_valid_position(&down_pos, &tetromino)
            };

            // Update position
            world.entity_mut(entity).insert(new_position.clone());

            // Update ghost position
            if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                if let Some(mut ghost) = entity_mut.get_mut::<Ghost>() {
                    ghost.position.x += dx;
                }
            }

            // Only spawn coyote time particles if we can't move down
            if coyote_time_active && !can_move_down {
                debug!(
                    "Spawning coyote time particles due to horizontal movement during coyote time"
                );
                particles::spawn_coyote_time_particles(world, &new_position, &tetromino);
            }
        }
    }

    // Handle soft drop
    if input.down {
        let new_position = Position {
            x: position.x,
            y: position.y + 1,
        };

        // Check if the move is valid
        let can_move_down = {
            let board = world.resource::<Board>();
            board.is_valid_position(&new_position, &tetromino)
        };

        if can_move_down {
            // Update position
            world.entity_mut(entity).insert(new_position.clone());

            // Track soft drop distance for scoring
            let mut game_state = world.resource_mut::<GameState>();
            game_state.soft_drop_distance += 1;

            // Reset drop timer to avoid immediate auto-drop
            game_state.drop_timer = 0.0;

            // Clear any existing coyote time state since we can move down
            if game_state.coyote_time_active {
                game_state.coyote_time_active = false;
                game_state.coyote_time_timer = 0.0;

                // Also reset the coyote time resource
                let mut coyote_time = world.resource_mut::<CoyoteTime>();
                coyote_time.active = false;
                coyote_time.timer = 0.0;

                // Clear any existing coyote time particles
                let particles_to_remove: Vec<Entity> = world
                    .query::<(Entity, &Particle)>()
                    .iter(world)
                    .filter(|(_, p)| p.color == ratatui::style::Color::White)
                    .map(|(e, _)| e)
                    .collect();

                for entity in particles_to_remove {
                    world.despawn(entity);
                }
            }
        } else {
            // When we can't move down during soft drop, we should lock immediately
            // instead of activating coyote time
            handle_piece_lock(world, entity, &position, &tetromino);
            return; // Exit early to prevent any other movement processing
        }
    }

    // Handle rotation
    if input.rotate {
        let mut new_tetromino = tetromino.clone();
        new_tetromino.rotate();

        // Check if the rotation is valid
        let can_rotate = {
            let board = world.resource::<Board>();
            board.is_valid_position(&position, &new_tetromino)
        };

        if can_rotate {
            // Check if piece can still move down after rotation
            let can_move_down = {
                let down_pos = Position {
                    x: position.x,
                    y: position.y + 1,
                };
                let board = world.resource::<Board>();
                board.is_valid_position(&down_pos, &new_tetromino)
            };

            // Update tetromino
            world.entity_mut(entity).insert(new_tetromino.clone());

            // Add rotation effect
            if fastrand::f32() < 0.3 {
                // Only 30% chance to spawn particles for rotation
                particles::spawn_rotation_particles(world, &position, &new_tetromino);
            }

            // Only spawn coyote time particles if we can't move down
            if coyote_time_active && !can_move_down {
                debug!("Spawning coyote time particles due to rotation during coyote time");
                particles::spawn_coyote_time_particles(world, &position, &new_tetromino);
            }
        }
    }
}

// Separate function for hard drop to avoid borrow checker issues
fn handle_hard_drop(world: &mut World) {
    // Get the active tetromino
    let mut entity_id = None;
    let mut tetromino_clone = None;
    let mut position_clone = None;

    {
        let mut query = world.query::<(Entity, &Tetromino, &Position, &Ghost)>();
        for (entity, tetromino, position, _) in query.iter(world) {
            entity_id = Some(entity);
            tetromino_clone = Some(tetromino.clone());
            position_clone = Some(position.clone());
            break; // Only process the first tetromino
        }
    }

    // If no tetromino found, exit early
    let (entity, tetromino, position) = match (entity_id, tetromino_clone, position_clone) {
        (Some(e), Some(t), Some(p)) => (e, t, p),
        _ => return,
    };

    // Calculate drop distance
    let mut hard_drop_distance = 0;
    let mut test_y = position.y;

    {
        let board = world.resource::<Board>();

        // Find how far the tetromino can drop
        loop {
            test_y += 1;
            if !board.is_valid_position(
                &Position {
                    x: position.x,
                    y: test_y,
                },
                &tetromino,
            ) {
                break;
            }
            hard_drop_distance += 1;
        }
    }

    // Update the game state with the hard drop distance for scoring
    {
        let mut game_state = world.resource_mut::<GameState>();
        game_state.update_hard_drop_score(hard_drop_distance);
    }

    // Update the position to the final position
    let final_position = Position {
        x: position.x,
        y: position.y + hard_drop_distance as i32,
    };

    // Lock the tetromino at the final position
    {
        let mut board = world.resource_mut::<Board>();
        board.lock_tetromino(&final_position, &tetromino);
    }

    // Spawn particles for the locked tetromino
    particles::spawn_lock_particles(world, &final_position, &tetromino);

    // Remove the tetromino and spawn a new one
    world.despawn(entity);
    spawn_tetromino(world);
}

pub fn game_tick_system(world: &mut World, delta_seconds: f32) {
    // Log delta seconds for debugging
    trace!("Game tick with delta: {}", delta_seconds);

    // Update particles first, regardless of game state
    particles::update_particles(world, delta_seconds);

    // Check if game is over
    let game_over = {
        let game_state = world.resource::<GameState>();
        game_state.game_over
    };

    if game_over {
        return;
    }

    // Update coyote time
    let coyote_time_expired = {
        // First get the current state
        let is_active = {
            let game_state = world.resource::<GameState>();
            game_state.coyote_time_active
        };

        if is_active {
            // Update coyote time first
            let mut coyote_time = world.resource_mut::<CoyoteTime>();
            coyote_time.active = true;
            coyote_time.timer += delta_seconds;
            let timer_value = coyote_time.timer;
            let should_expire = timer_value >= crate::game::COYOTE_TIME_DURATION;

            if should_expire {
                coyote_time.active = false;
                coyote_time.timer = 0.0;
            }

            // Then update game state
            let mut game_state = world.resource_mut::<GameState>();
            game_state.coyote_time_timer = timer_value;

            if should_expire {
                debug!("Coyote time expired");
                game_state.coyote_time_active = false;
                game_state.coyote_time_timer = 0.0;
                true
            } else {
                false
            }
        } else {
            // Reset both states
            {
                let mut coyote_time = world.resource_mut::<CoyoteTime>();
                coyote_time.active = false;
                coyote_time.timer = 0.0;
            }
            false
        }
    };

    // If coyote time just expired, handle tetromino locking
    if coyote_time_expired {
        let mut tetromino_entity = None;
        let mut tetromino_data = None;
        let mut position_data = None;

        // Collect the active tetromino data
        {
            let mut entities = world.query::<(Entity, &Tetromino, &Position)>();
            for (entity, tetromino, position) in entities.iter(world) {
                tetromino_entity = Some(entity);
                tetromino_data = Some(*tetromino);
                position_data = Some(*position);
                break;
            }
        }

        if let (Some(entity), Some(tetromino), Some(position)) =
            (tetromino_entity, tetromino_data, position_data)
        {
            // Before locking, check once more if the piece can continue falling
            if can_continue_falling(world, &position, &tetromino) {
                // If the piece can move down, update its position and continue
                debug!("Piece can continue falling after coyote time expired");
                let new_position = Position {
                    x: position.x,
                    y: position.y + 1,
                };
                world.entity_mut(entity).insert(new_position);

                // Reset drop timer to give player a moment before next drop
                let mut game_state = world.resource_mut::<GameState>();
                game_state.drop_timer = 0.0;
            } else {
                // Handle locking the piece
                handle_piece_lock(world, entity, &position, &tetromino);
            }
        }
        return;
    }

    // Update drop timer
    let should_drop = {
        let mut game_state = world.resource_mut::<GameState>();

        // Don't update drop timer if coyote time is active
        if game_state.coyote_time_active {
            false
        } else {
            // Add the elapsed time to our drop timer
            game_state.drop_timer += delta_seconds;

            // Get the drop delay based on level
            let drop_delay = game_state.get_drop_delay();

            // Debug log
            trace!(
                "Drop timer: {}, Drop delay: {}",
                game_state.drop_timer, drop_delay
            );

            // Check if it's time to drop the tetromino
            let should_drop = game_state.drop_timer >= drop_delay;

            // Reset timer if dropping
            if should_drop {
                game_state.drop_timer = 0.0;
                debug!("Dropping tetromino!");
            }

            should_drop
        }
    };

    // Handle automatic falling
    if should_drop {
        // Collect the active tetromino data
        let mut tetromino_entity = None;
        let mut tetromino_data = None;
        let mut position_data = None;

        {
            let mut entities = world.query::<(Entity, &Tetromino, &Position)>();

            // Log how many entities we have for debugging
            let count = entities.iter(world).count();
            debug!("Found {} tetromino entities", count);

            for (entity, tetromino, position) in entities.iter(world) {
                tetromino_entity = Some(entity);
                tetromino_data = Some(*tetromino);
                position_data = Some(*position);
                break;
            }
        }

        // No active tetromino, spawn one
        if tetromino_entity.is_none() {
            debug!("No active tetromino, spawning a new one");
            spawn_tetromino(world);
            return;
        }

        let entity = tetromino_entity.unwrap();
        let tetromino = tetromino_data.unwrap();
        let position = position_data.unwrap();

        trace!(
            "Moving tetromino at position ({}, {})",
            position.x, position.y
        );

        // Try to move tetromino down
        let new_position = Position {
            x: position.x,
            y: position.y + 1,
        };

        let can_move_down = {
            let board = world.resource::<Board>();
            board.is_valid_position(&new_position, &tetromino)
        };

        if can_move_down {
            // Update position
            debug!("Moving tetromino down");
            world.entity_mut(entity).insert(new_position);
        } else {
            // Instead of locking immediately, activate coyote time if not already active
            let mut game_state = world.resource_mut::<GameState>();
            if !game_state.coyote_time_active {
                debug!("Activating coyote time");
                game_state.coyote_time_active = true;
                game_state.coyote_time_timer = 0.0;

                // Spawn initial coyote time particles to give visual feedback
                particles::spawn_coyote_time_particles(world, &position, &tetromino);
            }
        }
    }
}

// Update handle_piece_lock to use the new particle module
fn handle_piece_lock(
    world: &mut World,
    entity: Entity,
    position: &Position,
    tetromino: &Tetromino,
) {
    info!("Locking tetromino in place");

    // Check for T-spin before locking
    let is_t_spin = {
        let board = world.resource::<Board>();
        let game_state = world.resource::<GameState>();
        game_state.is_t_spin(&board, position, tetromino)
    };

    // First lock the tetromino
    {
        let mut board = world.resource_mut::<Board>();
        board.lock_tetromino(position, tetromino);
    }

    // Then spawn the lock particles
    particles::spawn_lock_particles(world, position, tetromino);

    // Then clear lines and check for perfect clear
    let (lines_cleared, is_perfect_clear) = {
        let mut board = world.resource_mut::<Board>();

        // Clear completed lines
        let lines_cleared = board.clear_lines();

        // Check for perfect clear
        let is_perfect_clear = if lines_cleared > 0 {
            let cells = board.cells.clone();
            let is_empty = cells
                .iter()
                .all(|row| row.iter().all(|cell| cell.is_none()));
            is_empty
        } else {
            false
        };

        (lines_cleared, is_perfect_clear)
    };

    // Update score if needed
    if lines_cleared > 0 {
        info!(
            "Cleared {} lines (T-spin: {}, Perfect clear: {})",
            lines_cleared, is_t_spin, is_perfect_clear
        );

        let mut game_state = world.resource_mut::<GameState>();
        game_state.update_score(lines_cleared, is_t_spin, is_perfect_clear);

        // Spawn special particles for perfect clears
        if is_perfect_clear {
            particles::spawn_perfect_clear_particles(world, BOARD_WIDTH, BOARD_HEIGHT);
        }
    } else {
        // Reset combo counter if no lines were cleared
        let mut game_state = world.resource_mut::<GameState>();
        game_state.combo_count = 0;
    }

    // Remove the old tetromino entity
    world.despawn(entity);

    // Spawn a new tetromino
    spawn_tetromino(world);
}
