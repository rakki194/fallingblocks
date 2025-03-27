#![warn(clippy::all, clippy::pedantic)]

use crate::menu_types::{Menu, MenuOption};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

/// Renders the main menu options
pub fn render_main_menu_options(f: &mut Frame, area: Rect, menu: &Menu) {
    let options = vec!["New Game", "Options", "Quit"];
    let mut lines = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let style = if i
            == match menu.selected_option {
                MenuOption::NewGame => 0,
                MenuOption::Options => 1,
                MenuOption::Quit => 2,
            } {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![Span::styled(option.to_string(), style)]));
    }
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}
