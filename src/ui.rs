use crate::app::App;
use crate::components::{GameState, Particle, ScreenShake};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &mut App) {
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
        if !game_state.game_over {
            game_state.was_paused_for_resize = true;
        }

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
        x: (original_area.x as i16 + shake_x) as u16,
        y: (original_area.y as i16 + shake_y) as u16,
        width: original_area.width,
        height: original_area.height,
    };

    // Calculate the best layout split between game area and info panel
    // Allocate approximately 70% to the game and 30% to the info when space allows
    let game_percentage = if shake_area.width > (board_width + 2 * min_info_width) {
        70 // Default to 70% when plenty of space
    } else {
        // Calculate minimum percentage needed for the game board
        let min_game_percent = (board_width as f32 / shake_area.width as f32 * 100.0) as u16;
        // Cap between 50% and 80% to ensure info panel is still usable
        min_game_percent.min(80).max(50)
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
            let height_constrained_width =
                (available_board_height as f32 * (BOARD_WIDTH as f32 / BOARD_HEIGHT as f32)) as u16;
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
    let board_area = Rect {
        x: game_layout[1].x + (game_layout[1].width.saturating_sub(final_board_width)) / 2,
        y: game_layout[1].y + (game_layout[1].height.saturating_sub(final_board_height)) / 2,
        width: final_board_width,
        height: final_board_height,
    };

    // Define the info panel layout
    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Title
            Constraint::Length(10), // Score and next piece
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
            Constraint::Min(3),    // Current status
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
        format!("Combo: {}", combo_count)
    } else {
        "".to_string()
    };

    let back_to_back_text = if back_to_back { "Back-to-Back" } else { "" };

    let current_status = if game_state.game_over {
        Paragraph::new("GAME OVER!\nPress Enter to restart")
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
    } else {
        Paragraph::new(format!("{}\n{}", combo_text, back_to_back_text))
            .style(Style::default().fg(combo_color))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
    };

    f.render_widget(current_status, stats_layout[2]);

    // Render controls with updated key bindings
    let controls = Paragraph::new(
        "Controls:\n\
        ←/→: Move left/right\n\
        ↓: Soft drop\n\
        Enter: Hard drop\n\
        ↑/Space: Rotate\n\
        Q: Quit\n\
        ",
    )
    .block(Block::default().borders(Borders::TOP))
    .wrap(Wrap { trim: true });
    f.render_widget(controls, info_layout[2]);
}

/// Calculate the responsive board size based on available area
pub fn calculate_responsive_board_size(area: Rect) -> (u16, u16, u16, u16) {
    // Calculate available area (accounting for some minimal borders/margins)
    let available_width = area.width.saturating_sub(4); // Minimal horizontal margin
    let available_height = area.height.saturating_sub(4); // Minimal vertical margin

    // Calculate the aspect ratio of the original game board
    let board_aspect_ratio = BOARD_WIDTH as f32 / BOARD_HEIGHT as f32;

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

/// Center a rectangle horizontally within a larger rectangle
pub fn centered_horizontal_rect(width: u16, r: Rect) -> Rect {
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    Rect {
        x,
        y: r.y,
        width: width.min(r.width), // Ensure it doesn't exceed available width
        height: r.height,
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
        let x = position.x as u16;
        let y = position.y as u16;

        // Each cell is sized according to calculated dimensions
        if x < BOARD_WIDTH as u16 && y < BOARD_HEIGHT as u16 {
            let block_x = inner_area.left() + x * cell_width;

            // Fix: Invert Y coordinate to start from the bottom instead of the top
            let block_y = inner_area.bottom() - 1 - ((BOARD_HEIGHT as u16 - 1) - y) * cell_height;

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
            y: inner_area.y + (inner_area.height / 2),
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
        let x = particle.position.x as u16;
        let y = particle.position.y as u16;

        // Check if particle is inside the board area
        if x < BOARD_WIDTH as u16 && y < BOARD_HEIGHT as u16 {
            let particle_x = area.left() + x * cell_width;

            // Use the same Y-coordinate calculation as the game board
            let particle_y = area.bottom() - 1 - ((BOARD_HEIGHT as u16 - 1) - y) * cell_height;

            if particle_x < area.right() && particle_y < area.bottom() {
                let color = particle.color;

                // Different particle size based on the size attribute
                let particle_size = if particle.size > 0.7 {
                    "█" // Full block for larger particles
                } else if particle.size > 0.4 {
                    "▓" // Medium density for medium particles
                } else {
                    "▒" // Low density for small particles
                };

                // Draw particle (applying the same cell size as game blocks)
                for dx in 0..cell_width {
                    for dy in 0..cell_height.min(1) {
                        // Limit particle height to 1 for better aesthetics
                        if let Some(cell) =
                            f.buffer_mut().cell_mut((particle_x + dx, particle_y - dy))
                        {
                            cell.set_symbol(particle_size);
                            cell.set_fg(color);
                        }
                    }
                }
            }
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect
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
