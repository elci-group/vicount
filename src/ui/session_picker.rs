use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Draw the session picker overlay.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let block = Block::default()
        .title(" Resume Session ")
        .title_alignment(Alignment::Center)
        .title_style(theme.accent())
        .borders(Borders::ALL)
        .border_style(theme.primary())
        .style(theme.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();

    if app.session_picker_items.is_empty() {
        lines.push(Line::styled("No previous sessions", theme.muted()));
    } else {
        for (i, session) in app.session_picker_items.iter().enumerate() {
            let selected = i == app.session_picker_idx;
            let prefix = if selected { "▸ " } else { "  " };
            let style = if selected {
                theme.selected()
            } else {
                theme.base()
            };
            let label = if session.message_count > 0 {
                format!(
                    "{} {} ({} messages)",
                    prefix, session.name, session.message_count
                )
            } else {
                format!("{} {}", prefix, session.name)
            };
            lines.push(Line::styled(label, style));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "↑/↓ select · Enter resume · Esc/q cancel",
        theme.muted(),
    ));

    frame.render_widget(Paragraph::new(lines).style(theme.base()), inner);
}
