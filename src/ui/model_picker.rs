use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Draw the model picker overlay.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let block = Block::default()
        .title(" Select Model ")
        .title_alignment(Alignment::Center)
        .title_style(theme.accent())
        .borders(Borders::ALL)
        .border_style(theme.primary())
        .style(theme.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::styled("Current model:", theme.muted()));
    lines.push(Line::styled(app.model.clone(), theme.highlight()));
    lines.push(Line::from(""));

    for (i, model) in app.model_picker_providers.iter().enumerate() {
        let selected = i == app.model_picker_idx;
        let prefix = if selected { "▸ " } else { "  " };
        let style = if selected {
            theme.selected()
        } else {
            theme.base()
        };
        lines.push(Line::styled(format!("{}{} {}", prefix, i + 1, model), style));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled("↑/↓ select · Enter confirm · Esc/q cancel", theme.muted()));

    frame.render_widget(Paragraph::new(lines).style(theme.base()), inner);
}
