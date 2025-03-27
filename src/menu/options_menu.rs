#![warn(clippy::all, clippy::pedantic)]

use crate::app::App;
use crate::menu_types::{Menu, OptionsOption};
use crate::sound::AudioState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

/// Renders the options menu
pub fn render_options_menu(f: &mut Frame, area: Rect, menu: &Menu, app: &App) {
    let mut options = Vec::new();

    // Get audio state from the world
    if let Some(audio_state) = app.world.get_resource::<AudioState>() {
        options.push(format!(
            "Music: {}",
            if audio_state.is_music_enabled() {
                "ON"
            } else {
                "OFF"
            }
        ));

        options.push(format!(
            "Sound: {}",
            if audio_state.is_sound_enabled() {
                "ON"
            } else {
                "OFF"
            }
        ));

        options.push(format!("Volume: {:.1}", audio_state.get_volume()));
    } else {
        // Fallback if audio state isn't available
        options.push("Music: N/A".to_string());
        options.push("Sound: N/A".to_string());
        options.push("Volume: N/A".to_string());
    }

    options.push("Back".to_string());

    let mut lines = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let style = if i
            == match menu.options_selected {
                OptionsOption::MusicToggle => 0,
                OptionsOption::SoundToggle => 1,
                OptionsOption::VolumeUp => 2,
                OptionsOption::VolumeDown => 2,
                OptionsOption::Back => 3,
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
