use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Draw the quit confirmation overlay.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let block = Block::default()
        .title(" Quit? ")
        .title_alignment(Alignment::Center)
        .title_style(theme.accent())
        .borders(Borders::ALL)
        .border_style(theme.error())
        .style(theme.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::styled("Do you want to quit Vicount?", theme.base()));
    lines.push(Line::from(""));

    let yes_style = if app.quit_selected {
        theme.selected()
    } else {
        theme.base()
    };
    let no_style = if app.quit_selected {
        theme.base()
    } else {
        theme.selected()
    };

    lines.push(Line::from(vec![
        Span::styled("[ ", theme.base()),
        Span::styled(if app.quit_selected { "●" } else { "○" }, yes_style),
        Span::styled(" Yes ]  [ ", theme.base()),
        Span::styled(if app.quit_selected { "○" } else { "●" }, no_style),
        Span::styled(" No ]", theme.base()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::styled("←/→ · y/n · Enter · Esc", theme.muted()));

    frame.render_widget(Paragraph::new(lines).style(theme.base()), inner);
}
