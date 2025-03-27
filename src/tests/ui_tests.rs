#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::components::{GameState, TetrominoType};
    use crate::ui::{self, calculate_responsive_board_size, centered_rect, render_next_tetromino};
    use ratatui::{backend::TestBackend, layout::Rect, prelude::*};

    // Helper function to create a test terminal
    fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_centered_rect() {
        // Test centered_rect function
        let area = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(50, 40, area);

        // Check that it's centered and has the right proportions
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 40);
        assert_eq!(centered.x, 25); // (100 - 50) / 2
        assert_eq!(centered.y, 30); // (100 - 40) / 2
    }

    #[test]
    fn test_calculate_responsive_board_size() {
        // Test with a standard 80x24 terminal size
        let area = Rect::new(0, 0, 80, 24);
        let (board_width, board_height, cell_width, cell_height) =
            calculate_responsive_board_size(area);

        // Board should fit within the area
        assert!(board_width <= area.width);
        assert!(board_height <= area.height);

        // Cell size should be reasonable (at least 2x1)
        assert!(cell_width >= 2);
        assert!(cell_height >= 1);

        // Cell width should be even for better appearance
        assert_eq!(cell_width % 2, 0);

        // Test with a very small terminal - increase the height to 20 to ensure it's big enough
        let small_area = Rect::new(0, 0, 30, 20);
        let (small_board_width, _small_board_height, small_cell_width, small_cell_height) =
            calculate_responsive_board_size(small_area);

        // Board should still fit
        assert!(small_board_width <= small_area.width);

        // Even with small terminal, cells should be at least minimal size
        assert!(small_cell_width >= 2);
        assert!(small_cell_height >= 1);

        // Test with a very large terminal
        let large_area = Rect::new(0, 0, 200, 100);
        let (large_board_width, large_board_height, large_cell_width, large_cell_height) =
            calculate_responsive_board_size(large_area);

        // Board should be larger with a bigger terminal
        assert!(large_board_width > board_width);
        assert!(large_board_height > board_height);

        // Cell size should be larger
        assert!(large_cell_width > cell_width);
        assert!(large_cell_height >= cell_height);
    }

    #[test]
    fn test_game_render_with_small_terminal() {
        // Create a very small terminal that can't fit the game
        let mut terminal = create_test_terminal(20, 10);
        let mut app = App::new();

        // Set the menu state to Game so render_game gets called
        app.menu.state = crate::menu_types::MenuState::Game;

        // This should show the warning screen and not crash
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();

        // Game should be paused
        let game_state = app.world.resource::<GameState>();
        assert!(game_state.was_paused_for_resize);
    }

    #[test]
    fn test_game_over_rendering() {
        // Setup app with game over state
        let mut app = App::new();
        {
            let mut game_state = app.world.resource_mut::<GameState>();
            game_state.game_over = true;
        }

        // Create a terminal and render
        let mut terminal = create_test_terminal(80, 30);

        // This should render the game over text without crashing
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    }

    #[test]
    fn test_next_tetromino_rendering() {
        // Create a test app
        let mut app = App::new();

        // Set a known next tetromino
        {
            let mut game_state = app.world.resource_mut::<GameState>();
            game_state.next_tetromino = Some(TetrominoType::I);
        }

        // Create a test terminal
        let mut terminal = create_test_terminal(40, 20);

        // Create a test area for the next tetromino preview
        let preview_area = Rect::new(5, 5, 10, 10);

        // Test that rendering doesn't panic
        terminal
            .draw(|f| {
                render_next_tetromino(f, &mut app, preview_area);
            })
            .unwrap();

        // Verify that something was rendered (buffer is not empty)
        let buffer = terminal.backend().buffer();
        let mut has_content = false;

        for x in preview_area.left()..preview_area.right() {
            for y in preview_area.top()..preview_area.bottom() {
                if let Some(cell) = buffer.cell((x, y)) {
                    // Check if the cell has content (foreground color changed from default)
                    if cell.fg != Color::Reset && cell.fg != Color::White {
                        has_content = true;
                        break;
                    }
                }
            }
            if has_content {
                break;
            }
        }

        assert!(has_content, "Next tetromino preview should render content");
    }
}
