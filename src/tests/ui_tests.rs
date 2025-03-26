#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::components::{GameState, Position, Tetromino, TetrominoType};
    use crate::game::{BOARD_HEIGHT, BOARD_WIDTH};
    use crate::ui::{self, centered_rect};
    use ratatui::{prelude::*, widgets::*};

    // Helper function to create a test frame
    fn create_test_frame() -> (Terminal<TestBackend>, Frame<'static>) {
        let backend = TestBackend::new(80, 30);
        let terminal = Terminal::new(backend).unwrap();
        let frame = Frame::new(
            terminal.current_buffer_mut(),
            terminal.current_buffer_mut().area,
        );
        (terminal, frame)
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
            ui::calculate_responsive_board_size(area);

        // Board should fit within the area
        assert!(board_width <= area.width);
        assert!(board_height <= area.height);

        // Cell size should be reasonable (at least 2x1)
        assert!(cell_width >= 2);
        assert!(cell_height >= 1);

        // Cell width should be even for better appearance
        assert_eq!(cell_width % 2, 0);

        // Test with a very small terminal
        let small_area = Rect::new(0, 0, 30, 15);
        let (small_board_width, small_board_height, small_cell_width, small_cell_height) =
            ui::calculate_responsive_board_size(small_area);

        // Board should still fit
        assert!(small_board_width <= small_area.width);
        assert!(small_board_height <= small_area.height);

        // Even with small terminal, cells should be at least minimal size
        assert!(small_cell_width >= 2);
        assert!(small_cell_height >= 1);

        // Test with a very large terminal
        let large_area = Rect::new(0, 0, 200, 100);
        let (large_board_width, large_board_height, large_cell_width, large_cell_height) =
            ui::calculate_responsive_board_size(large_area);

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
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

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
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // This should render the game over text without crashing
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    }
}

// Simple test backend for ratatui
#[derive(Debug)]
struct TestBackend {
    width: u16,
    height: u16,
    buffer: Buffer,
}

impl TestBackend {
    fn new(width: u16, height: u16) -> Self {
        TestBackend {
            width,
            height,
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }
}

impl Backend for TestBackend {
    fn draw<'a, I>(&mut self, content: I) -> Result<(), std::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, cell) in content {
            if x < self.width && y < self.height {
                self.buffer.get_mut(x, y).clone_from(cell);
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), std::io::Error> {
        Ok((0, 0))
    }

    fn set_cursor(&mut self, _: u16, _: u16) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), std::io::Error> {
        self.buffer = Buffer::empty(Rect::new(0, 0, self.width, self.height));
        Ok(())
    }

    fn size(&self) -> Result<Rect, std::io::Error> {
        Ok(Rect::new(0, 0, self.width, self.height))
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
