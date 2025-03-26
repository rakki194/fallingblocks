use crate::app::App;
use crate::components::{GameState, Particle, ScreenShake};
use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &mut App) {
    // Define minimum sizes required for the game to be playable
    let cell_width = 2; // Each cell is 2 characters wide
    let cell_height = 1; // Each cell is 1 character tall
    let board_width = BOARD_WIDTH as u16 * cell_width + 2; // +2 for borders
    let board_height = BOARD_HEIGHT as u16 * cell_height + 2; // +2 for borders
    let min_info_width = 20u16;
    let min_total_width = board_width + min_info_width;
    let min_total_height = board_height + 5; // Adding space for title and borders

    // Check if the terminal is too small to render the game properly
    if f.area().width < min_total_width || f.area().height < min_total_height {
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
        let warning_area = centered_rect(50, 30, f.area());
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

    // Calculate required board space
    let cell_width = 2; // Each cell is 2 characters wide
    let cell_height = 1; // Each cell is 1 character tall
    let board_width = BOARD_WIDTH as u16 * cell_width + 2; // +2 for borders
    let board_height = BOARD_HEIGHT as u16 * cell_height + 2; // +2 for borders

    // Define minimum required width for the info panel
    let min_info_width = 20u16;

    // Calculate total minimum width needed
    let min_total_width = board_width + min_info_width;

    // Define layout with minimum width consideration
    let available_width = shake_area.width;
    let board_percentage = if available_width > min_total_width {
        (board_width as f64 / available_width as f64 * 100.0) as u16
    } else {
        70 // Default if screen is too small
    };

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(board_percentage),
            Constraint::Percentage(100 - board_percentage),
        ])
        .split(shake_area);

    let game_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),            // Title
            Constraint::Fill(1),              // Flexible spacing above game board
            Constraint::Length(board_height), // Game board (fixed height)
            Constraint::Length(1),            // Bottom border
        ])
        .split(main_layout[0]);

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

    // Render game board - now using index 2 since we added a fill constraint
    render_game_board(f, app, game_layout[2]);

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

fn render_game_board(f: &mut Frame, app: &mut App, area: Rect) {
    // Calculate fixed cell size
    let cell_width = 2; // Each cell is 2 characters wide
    let _cell_height = 1; // Each cell is 1 character tall

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

        // Each cell is 2x1 characters to make it more square-like
        if x < BOARD_WIDTH as u16 && y < BOARD_HEIGHT as u16 {
            let block_x = inner_area.left() + x * cell_width;

            // Fix: Invert Y coordinate to start from the bottom instead of the top
            // This ensures blocks appear at the bottom of the board with no extra space
            let block_y = inner_area.bottom() - 1 - ((BOARD_HEIGHT as u16 - 1) - y);

            if block_x < inner_area.right() && block_y < inner_area.bottom() {
                let color = tetromino_type.get_color();

                // Use the current Ratatui API for setting cells
                if let Some(cell) = f.buffer_mut().cell_mut((block_x, block_y)) {
                    cell.set_symbol("█");
                    cell.set_fg(color);
                    cell.set_bg(Color::Black);
                }

                // Make the block two cells wide for better proportions
                if let Some(cell) = f.buffer_mut().cell_mut((block_x + 1, block_y)) {
                    cell.set_symbol("█");
                    cell.set_fg(color);
                    cell.set_bg(Color::Black);
                }
            }
        }
    }

    // Render particles
    render_particles(f, app, inner_area);

    // If game is over, overlay "GAME OVER" text
    let game_state = app.world.resource::<GameState>();
    if game_state.game_over {
        let game_over = Paragraph::new("GAME OVER")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

        let game_over_area = Rect {
            x: inner_area.x + (inner_area.width / 2) - 5,
            y: inner_area.y + (inner_area.height / 2),
            width: 10,
            height: 1,
        };

        f.render_widget(game_over, game_over_area);
    }
}

// Render all particles
fn render_particles(f: &mut Frame, app: &mut App, area: Rect) {
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
            let particle_x = area.left() + x * 2;

            // Use the same Y-coordinate calculation as the game board
            let particle_y = area.bottom() - 1 - ((BOARD_HEIGHT as u16 - 1) - y);

            if particle_x < area.right() && particle_y < area.bottom() {
                let _opacity = (particle.lifetime * 255.0) as u8;
                let color = particle.color;

                // Different particle size based on the size attribute
                let particle_size = if particle.size > 0.7 {
                    "█" // Full block for larger particles
                } else if particle.size > 0.4 {
                    "▓" // Medium density for medium particles
                } else {
                    "▒" // Low density for small particles
                };

                // Draw particle
                if let Some(cell) = f.buffer_mut().cell_mut((particle_x, particle_y)) {
                    cell.set_symbol(particle_size);
                    cell.set_fg(color);
                }
            }
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
