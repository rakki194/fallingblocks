#![warn(clippy::all, clippy::pedantic)]

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::Paragraph,
};

use super::renderer::centered_rect;
use crate::config::CONFIG;

// ASCII art letters for "FALLINGBLOCKS" title
pub const TITLE_LETTERS: [&str; 13] = [
    // F
    "████\n█   \n███ \n█   \n█   ",
    // A
    " ██ \n█  █\n████\n█  █\n█  █",
    // L
    "█   \n█   \n█   \n█   \n████",
    // L
    "█   \n█   \n█   \n█   \n████",
    // I
    "███\n █ \n █ \n █ \n███",
    // N
    "█  █\n██ █\n█ ██\n█  █\n█  █",
    // G
    " ███ \n█    \n█  ██\n█   █\n ███ ",
    // B
    "███ \n█  █\n███ \n█  █\n███ ",
    // L
    "█   \n█   \n█   \n█   \n████",
    // O
    " ██ \n█  █\n█  █\n█  █\n ██ ",
    // C
    " ███\n█   \n█   \n█   \n ███",
    // K
    "█  █\n█ █ \n██  \n█ █ \n█  █",
    // S
    " ███\n█   \n ██ \n   █\n███ ",
];

/// Renders the ASCII art title
pub fn render_ascii_title(f: &mut Frame, area: Rect, colors: &[Color]) {
    // Check for configuration updates
    crate::config::Config::check_and_reload();

    // Get the current configuration
    let config = CONFIG.read().unwrap();
    let kerning_pairs = &config.menu.title.kerning_adjustments;

    // Calculate base width (without kerning)
    let base_width: usize = TITLE_LETTERS
        .iter()
        .map(|letter| letter.lines().next().map_or(0, |l| l.len()))
        .sum();

    // Calculate total kerning adjustment
    let total_kerning = crate::config::menu::get_total_kerning_adjustment(kerning_pairs);

    // Adjusted width with kerning
    let title_width = base_width - total_kerning;
    let title_height = config.menu.title.title_height;

    // Calculate centered position
    let title_area = centered_rect(title_width as u16, title_height as u16, area);

    // Render each letter with cycling colors
    let mut current_x = title_area.x;

    for (i, letter) in TITLE_LETTERS.iter().enumerate() {
        let letter_width = letter.lines().next().map_or(0, |l| l.len()) as u16;
        let letter_area = Rect::new(current_x, title_area.y, letter_width, title_height as u16);

        // Use a different color for each letter
        let color_idx = i % colors.len();
        let style = Style::default().fg(colors[color_idx]);

        let paragraph = Paragraph::new(Text::from(letter.to_string())).style(style);

        f.render_widget(paragraph, letter_area);

        // Move to the next letter position
        current_x += letter_width;

        // Apply specific kerning adjustment for this letter pair
        if i < kerning_pairs.len() {
            let pair = &kerning_pairs[i];
            if pair.letter_index == i {
                current_x = current_x.saturating_add_signed(pair.adjustment);
            }
        }
    }
}

/// Gets the dimensions and position of the title
pub fn get_title_protection_zone(area: Rect) -> Rect {
    // Get the current configuration
    let config = CONFIG.read().unwrap();
    let kerning_pairs = &config.menu.title.kerning_adjustments;

    // Calculate base width (without kerning)
    let base_width: usize = TITLE_LETTERS
        .iter()
        .map(|letter| letter.lines().next().map_or(0, |l| l.len()))
        .sum();

    // Calculate total kerning adjustment
    let total_kerning = crate::config::menu::get_total_kerning_adjustment(kerning_pairs);

    // Adjusted width with kerning
    let title_width = base_width - total_kerning;
    let title_height = config.menu.title.title_height;

    // Calculate title position - top center of screen
    let title_area = centered_rect(
        title_width as u16,
        title_height as u16,
        Rect::new(0, 0, area.width, 10),
    );

    // Create a larger protection zone around the title
    Rect::new(
        0, // Protect the entire top
        0,
        area.width, // Full width
        title_area.y + title_area.height + config.menu.title.protection_margin as u16, // Title height plus configurable margin
    )
}
