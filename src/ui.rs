#![warn(clippy::all, clippy::pedantic)]
#![allow(
    // Allow truncation when casting from usize to u16 since UI dimensions are always small enough to fit in u16
    clippy::cast_possible_truncation,
    // Allow sign loss when going from signed to unsigned types since we validate values are non-negative before casting
    clippy::cast_sign_loss,
    // Allow precision loss when casting between numeric types since exact precision isn't critical for UI rendering
    clippy::cast_precision_loss,
    // Allow potential wrapping when casting between types of same size as UI values are in reasonable ranges
    clippy::cast_possible_wrap,
    // Allow functions with many lines for complex UI rendering logic where splitting would reduce readability
    clippy::too_many_lines,
    // Allow underscore bindings that don't have side effects in UI code for consistency with naming patterns
    clippy::no_effect_underscore_binding
)]

use crate::app::App;
use crate::components::{GameState, Particle, ScreenShake};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::menu::MenuRenderer;
use crate::menu_types::{MenuState, OptionsOption};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &mut App) {
    if app.menu.state == MenuState::Game {
        render_game(f, app);
    } else {
        // Update the menu renderer first
        app.menu_renderer.update();

        // Clone the menu state to avoid borrow issues
        let menu_state = app.menu.clone();
        let menu_renderer = &app.menu_renderer;

        // Render the menu
        MenuRenderer::render_menu(f, app, &menu_state, menu_renderer);
    }
}

/// Render the main game UI
fn render_game(f: &mut Frame, app: &mut App) {
    // Get available area
    let available_area = f.area();

    // Calculate ideal game board dimensions based on available space
    let (board_width, board_height, cell_width, cell_height) =
        calculate_responsive_board_size(available_area);

    // Minimum info panel width
    let min_info_width = 20u16;

    // Check if the terminal is too small to render the game properly
    if available_area.width < (board_width + min_info_width) || available_area.height < board_height
    {
        // Pause the game by updating the game state
        let mut game_state = app.world.resource_mut::<GameState>();
        // Always set was_paused_for_resize to true regardless of game_over state
        game_state.was_paused_for_resize = true;

        // Create a warning message block
        let warning_text = Paragraph::new(
            "Terminal too small!\nPlease resize your terminal\nto continue playing.",
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Tetris - Paused"),
        );

        // Center the warning message in the available space
        let warning_area = centered_rect(50, 30, available_area);
        f.render_widget(warning_text, warning_area);
        return;
    } else if app.world.resource::<GameState>().was_paused_for_resize {
        let mut game_state = app.world.resource_mut::<GameState>();
        game_state.was_paused_for_resize = false;
    }

    // Get screen shake offset if active
    let (shake_x, shake_y) = {
        let screen_shake = app.world.resource::<ScreenShake>();
        (screen_shake.current_offset.0, screen_shake.current_offset.1)
    };

    // Apply screen shake to the entire frame
    let original_area = f.area();
    let shake_area = Rect {
        x: (original_area.x as i16 + shake_x).clamp(0, original_area.width as i16) as u16,
        y: (original_area.y as i16 + shake_y).clamp(0, original_area.height as i16) as u16,
        width: original_area.width,
        height: original_area.height,
    };

    // Calculate the best layout split between game area and info panel
    // Allocate approximately 70% to the game and 30% to the info when space allows
    let game_percentage = if shake_area.width > (board_width + 2 * min_info_width) {
        70 // Default to 70% when plenty of space
    } else {
        // Calculate minimum percentage needed for the game board
        let min_game_percent =
            (f32::from(board_width) / f32::from(shake_area.width) * 100.0) as u16;
        // Cap between 50% and 80% to ensure info panel is still usable
        min_game_percent.clamp(50, 80)
    };

    // Create the horizontal layout
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(game_percentage),
            Constraint::Percentage(100 - game_percentage),
        ])
        .split(shake_area);

    // Game area (left panel)
    let game_area = main_layout[0];

    // Calculate the vertical space needed for the board within the game area
    let title_height = 2u16; // Height of the title area
    let bottom_margin = 1u16; // Height of the bottom margin
    let available_board_height = game_area
        .height
        .saturating_sub(title_height + bottom_margin);

    // If board_height is greater than available_board_height, we need to recalculate
    let (final_board_width, final_board_height, final_cell_width, final_cell_height) =
        if board_height > available_board_height {
            // Recalculate with height constraint
            let height_constrained_width = (f32::from(available_board_height)
                * (BOARD_WIDTH as f32 / BOARD_HEIGHT as f32))
                as u16;
            let new_cell_width = (height_constrained_width / BOARD_WIDTH as u16).max(2);
            let new_cell_height = (new_cell_width / 2).max(1);

            (
                BOARD_WIDTH as u16 * new_cell_width + 2,
                BOARD_HEIGHT as u16 * new_cell_height + 2,
                new_cell_width,
                new_cell_height,
            )
        } else {
            (board_width, board_height, cell_width, cell_height)
        };

    // Create game layout with vertical constraints
    let game_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height),    // Title
            Constraint::Min(final_board_height), // Space for board (at least as tall as the board)
            Constraint::Length(bottom_margin),   // Bottom margin
        ])
        .split(game_area);

    // Board area - center horizontally within available space
    let board_area =
        centered_horizontal_rect(final_board_width, final_board_height, game_layout[1]);

    // Define the info panel layout
    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Title
            Constraint::Length(13), // Stats
            Constraint::Length(10), // Next piece preview
            Constraint::Min(5),     // Controls
            Constraint::Length(1),  // Bottom border
        ])
        .split(main_layout[1]);

    // Render game title
    let title = Paragraph::new("TETRIS")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, game_layout[0]);

    // Render game board with the calculated dimensions
    render_game_board(f, app, board_area, final_cell_width, final_cell_height);

    // Render score and info
    let info_title = Paragraph::new("INFO")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(info_title, info_layout[0]);

    // Create stats layout first
    let stats_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Basic stats
            Constraint::Length(5), // Achievement stats
            Constraint::Length(3), // Current status (fixed height)
        ])
        .split(info_layout[1]);

    // Basic stats
    let game_state = app.world.resource::<GameState>();

    let basic_stats = format!(
        "Score: {}\nLevel: {}\nLines: {}",
        game_state.score, game_state.level, game_state.lines_cleared,
    );

    let basic_info = Paragraph::new(basic_stats)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    f.render_widget(basic_info, stats_layout[0]);

    // Achievement stats
    let achievement_stats = format!(
        "Tetris: {}\nT-Spins: {}\nPerfect Clears: {}",
        game_state.tetris_count, game_state.t_spin_count, game_state.perfect_clear_count,
    );

    let achievement_info = Paragraph::new(achievement_stats)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    f.render_widget(achievement_info, stats_layout[1]);

    // Current combo and back-to-back status
    let combo_count = game_state.combo_count;
    let back_to_back = game_state.back_to_back;

    let combo_color = if combo_count > 1 {
        if combo_count > 5 {
            Color::LightMagenta
        } else if combo_count > 3 {
            Color::LightCyan
        } else {
            Color::LightGreen
        }
    } else {
        Color::White
    };

    let combo_text = if combo_count > 1 {
        format!("Combo: {combo_count}")
    } else {
        String::new()
    };

    let back_to_back_text = if back_to_back { "Back-to-Back" } else { "" };

    let status_text = if game_state.game_over {
        "GAME OVER!\nPress Enter to restart".to_string()
    } else {
        let mut text = String::new();
        if !combo_text.is_empty() {
            text.push_str(&combo_text);
        }
        if !back_to_back_text.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(back_to_back_text);
        }
        text
    };

    let current_status = Paragraph::new(status_text)
        .style(Style::default().fg(if game_state.game_over {
            Color::Red
        } else {
            combo_color
        }))
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(current_status, stats_layout[2]);

    // Render next tetromino preview
    render_next_tetromino(f, app, info_layout[2]);

    // Render controls with updated key bindings
    let controls = Paragraph::new(
        "Controls:\n\
        ←/→: Move left/right\n\
        ↓: Soft drop\n\
        E: Hard drop\n\
        ↑/Space: Rotate\n\
        Q: Quit\n\
        ",
    )
    .block(Block::default().borders(Borders::TOP))
    .wrap(Wrap { trim: true });
    f.render_widget(controls, info_layout[3]);
}

/// Calculate the responsive board size based on available area
#[must_use]
pub fn calculate_responsive_board_size(area: Rect) -> (u16, u16, u16, u16) {
    // Calculate the available space
    let available_width = area.width.saturating_sub(4); // Subtract margin
    let available_height = area.height.saturating_sub(4); // Subtract margin

    // Calculate the aspect ratio of the original game board
    let _board_aspect_ratio = BOARD_WIDTH as f32 / BOARD_HEIGHT as f32;

    // Base cell size calculations
    // Determine maximum possible cell dimensions while maintaining proper aspect ratio
    let max_cell_width_by_width = available_width / BOARD_WIDTH as u16;
    let max_cell_height_by_height = available_height / BOARD_HEIGHT as u16;

    // Enforce 2:1 width-to-height ratio for visual "square" appearance in terminal
    // Terminal characters are typically about twice as tall as they are wide
    let max_cell_width_by_height = max_cell_height_by_height * 2;

    // Choose the smaller dimension to ensure board fits
    let cell_width = max_cell_width_by_width.min(max_cell_width_by_height);
    // Ensure cell width is at least 2 and even (for better appearance)
    let cell_width = (cell_width.max(2) / 2) * 2;

    // Calculate cell height based on the 2:1 ratio (half the width, minimum 1)
    let cell_height = (cell_width / 2).max(1);

    // Calculate final board dimensions including borders
    let board_width = BOARD_WIDTH as u16 * cell_width + 2; // +2 for borders
    let board_height = BOARD_HEIGHT as u16 * cell_height + 2; // +2 for borders

    (board_width, board_height, cell_width, cell_height)
}

#[must_use]
pub fn centered_horizontal_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width), // Ensure it doesn't exceed available width
        height: height.min(r.height), // Ensure it doesn't exceed available height
    }
}

fn render_game_board(f: &mut Frame, app: &mut App, area: Rect, cell_width: u16, cell_height: u16) {
    // Calculate the inner area (inside the borders)
    let inner_area = Block::default().borders(Borders::ALL).inner(area);

    // Render the game board border
    f.render_widget(Block::default().borders(Borders::ALL), area);

    // Get blocks to render using the app's helper method
    let blocks = app.get_render_blocks();

    // Render each block
    for (position, tetromino_type) in blocks {
        let x = position.x.clamp(0, (BOARD_WIDTH - 1) as i32) as u16;
        let y = position.y.clamp(0, (BOARD_HEIGHT - 1) as i32) as u16;

        // Each cell is sized according to calculated dimensions
        if x < BOARD_WIDTH as u16 && y < BOARD_HEIGHT as u16 {
            let block_x = inner_area
                .left()
                .saturating_add(x.saturating_mul(cell_width));

            // Fix: Invert Y coordinate to start from the bottom instead of the top
            let block_y = inner_area.bottom().saturating_sub(1).saturating_sub(
                ((BOARD_HEIGHT as u16).saturating_sub(1).saturating_sub(y))
                    .saturating_mul(cell_height),
            );

            if block_x < inner_area.right() && block_y < inner_area.bottom() {
                let color = tetromino_type.get_color();

                // Draw a single block with proper proportional size
                // For cell_width=2 and cell_height=1, this matches the original rendering
                let block_char = if cell_width >= 2 && cell_height >= 1 {
                    "█"
                } else {
                    "■"
                };

                for dx in 0..cell_width {
                    for dy in 0..cell_height {
                        if let Some(cell) = f.buffer_mut().cell_mut((block_x + dx, block_y - dy)) {
                            cell.set_symbol(block_char);
                            cell.set_fg(color);
                            cell.set_bg(Color::Black);
                        }
                    }
                }
            }
        }
    }

    // Render particles
    render_particles(f, app, inner_area, cell_width, cell_height);

    // If game is over, overlay "GAME OVER" text
    let game_state = app.world.resource::<GameState>();
    if game_state.game_over {
        let game_over = Paragraph::new("GAME OVER")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

        let game_over_area = Rect {
            x: inner_area.x,
            y: inner_area.y.saturating_add(inner_area.height / 2),
            width: inner_area.width,
            height: 1,
        };

        f.render_widget(game_over, game_over_area);
    }
}

// Render all particles
fn render_particles(f: &mut Frame, app: &mut App, area: Rect, cell_width: u16, cell_height: u16) {
    // Collect all particles
    let particles_data = app
        .world
        .query::<&Particle>()
        .iter(&app.world)
        .collect::<Vec<_>>();

    for particle in particles_data {
        // Convert position to u16, clamping to board boundaries
        let x = particle.position.x.clamp(0, (BOARD_WIDTH - 1) as i32) as u16;
        let y = particle.position.y.clamp(0, (BOARD_HEIGHT - 1) as i32) as u16;

        // Calculate screen position
        let particle_x = area.left().saturating_add(x.saturating_mul(cell_width));
        let particle_y = area.bottom().saturating_sub(1).saturating_sub(
            ((BOARD_HEIGHT as u16).saturating_sub(1).saturating_sub(y)).saturating_mul(cell_height),
        );

        // Only render if within screen bounds
        if particle_x < area.right() && particle_y < area.bottom() {
            let color = particle.color;

            // Different particle size based on the size attribute
            let particle_size = if particle.size > 0.85 {
                "█" // Full block for largest particles
            } else if particle.size > 0.7 {
                "▇" // Nearly full block
            } else if particle.size > 0.55 {
                "▆" // Mostly full block
            } else if particle.size > 0.4 {
                "▓" // Medium density
            } else if particle.size > 0.25 {
                "▒" // Low-medium density
            } else {
                "░" // Very low density
            };

            // Draw particle (applying the same cell size as game blocks)
            for dx in 0..cell_width {
                for dy in 0..cell_height.min(1) {
                    // Limit particle height to 1 for better aesthetics
                    if let Some(cell) = f.buffer_mut().cell_mut((particle_x + dx, particle_y - dy))
                    {
                        cell.set_symbol(particle_size);
                        cell.set_fg(color);
                    }
                }
            }
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect
#[must_use]
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Function to render the next tetromino preview
pub fn render_next_tetromino(f: &mut Frame, app: &mut App, area: Rect) {
    // Create a preview box with a title
    let next_block = Block::default().title("NEXT").borders(Borders::ALL);

    // Get inner area for the preview before rendering the block
    let inner_area = next_block.inner(area);

    f.render_widget(next_block, area);

    // Get the next tetromino type from game state
    if let Some(game_state) = app.world.get_resource::<GameState>() {
        if let Some(next_type) = game_state.next_tetromino {
            // Get blocks for the tetromino type
            let blocks = next_type.get_blocks();

            // Calculate size needed for the tetromino
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;

            for &(x, y) in &blocks {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }

            // Calculate cell dimensions to match board aspect ratio
            let width = (max_x - min_x + 1) as u16;
            let height = (max_y - min_y + 1) as u16;

            // Calculate available space in the preview area
            let available_width = inner_area.width;
            let available_height = inner_area.height;

            // Calculate block size that fits within the preview area
            // Accounting for terminal character aspect ratio (2:1 width-to-height)
            let max_block_width = available_width / width;
            let max_block_height = available_height / height;

            // Ensure proper aspect ratio (terminal characters are typically twice as tall as they are wide)
            let block_width = max_block_width.min(max_block_height * 2);
            // Make sure width is even for better appearance
            let block_width = (block_width / 2) * 2;
            let block_height = (block_width / 2).max(1);

            // Center the tetromino in the preview area
            let total_width = width.saturating_mul(block_width);
            let total_height = height.saturating_mul(block_height);

            let start_x = inner_area
                .left()
                .saturating_add((available_width.saturating_sub(total_width)) / 2);
            let start_y = inner_area
                .top()
                .saturating_add((available_height.saturating_sub(total_height)) / 2);

            let color = next_type.get_color();

            // Draw the tetromino blocks
            for &(x, y) in &blocks {
                let block_x = start_x.saturating_add((x - min_x) as u16 * block_width);
                let block_y = start_y.saturating_add((y - min_y) as u16 * block_height);

                if block_x < inner_area.right() && block_y < inner_area.bottom() {
                    // Draw a single block with proper proportional size
                    let block_char = if block_width >= 2 && block_height >= 1 {
                        "█"
                    } else {
                        "■"
                    };

                    for dx in 0..block_width {
                        for dy in 0..block_height {
                            if let Some(cell) =
                                f.buffer_mut().cell_mut((block_x + dx, block_y + dy))
                            {
                                cell.set_symbol(block_char);
                                cell.set_fg(color);
                                cell.set_bg(Color::Black);
                            }
                        }
                    }
                }
            }
        }
    }
}
