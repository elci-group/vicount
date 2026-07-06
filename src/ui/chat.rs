use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;

use crate::app::App;
use crate::types::Overlay;
use crate::ui::{help, input, messages, model_picker, quit, session_picker, side_panel, slash_menu, status};

/// Draw the main chat layout.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let theme = app.theme;

    // Overall vertical layout: chat + input + status bar.
    // Input area grows with the composer content up to a max of ~40% of screen.
    let input_lines = app.input_line_count() as u16;
    let input_height = (input_lines + 2).clamp(3, (area.height as f32 * 0.4).max(5.0) as u16);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(input_height), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let input_area = vertical[1];
    let status_area = vertical[2];

    // Horizontal split: side panel + chat area.
    let side_width = 30.min(area.width / 4).max(20);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(side_width), Constraint::Min(20)])
        .split(main_area);

    let side_rect = horizontal[0];
    let chat_rect = horizontal[1];

    // Side panel.
    side_panel::draw(frame, app, side_rect);

    // Chat area (messages + composer share this rectangle; composer is separate below).
    let chat_block = Block::default()
        .borders(Borders::RIGHT | Borders::BOTTOM)
        .border_style(theme.border())
        .style(theme.base());
    frame.render_widget(chat_block.clone(), chat_rect);

    let inner = chat_block.inner(chat_rect);
    let chat_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(0)])
        .split(inner)[0];

    messages::draw(frame, app, chat_inner);

    // Input composer.
    input::draw(frame, app, input_area);

    // Status bar.
    status::draw(frame, app, status_area);

    // Slash autocomplete floats above the input area.
    if app.slash_open {
        let matches = app.slash_matches();
        if !matches.is_empty() {
            slash_menu::draw(frame, app, matches, input_area);
        }
    }

    // Centered overlays.
    match app.overlay {
        Overlay::Help => {
            let rect = centered_rect(70, 70, area);
            frame.render_widget(Clear, rect);
            help::draw(frame, app, rect);
        }
        Overlay::ModelPicker => {
            let rect = centered_rect(60, 60, area);
            frame.render_widget(Clear, rect);
            model_picker::draw(frame, app, rect);
        }
        Overlay::SessionPicker => {
            let rect = centered_rect(60, 60, area);
            frame.render_widget(Clear, rect);
            session_picker::draw(frame, app, rect);
        }
        Overlay::Quit => {
            let rect = centered_rect(40, 20, area);
            frame.render_widget(Clear, rect);
            quit::draw(frame, app, rect);
        }
        Overlay::None => {}
    }
}

/// Compute a centered rectangle with the given percentage sizes.
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
