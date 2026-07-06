use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, SLASH_COMMANDS};

/// Draw the help overlay.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .title_style(theme.accent())
        .borders(Borders::ALL)
        .border_style(theme.primary())
        .style(theme.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::styled("Vicount — ViCo Desktop TUI", theme.highlight()));
    lines.push(Line::styled("Chat-first, slash-command-heavy interface.", theme.muted()));
    lines.push(Line::from(""));

    for cmd in SLASH_COMMANDS {
        let name = format!("/{}", cmd.name);
        let hint = if cmd.args_hint.is_empty() {
            String::new()
        } else {
            format!(" {}", cmd.args_hint)
        };
        lines.push(Line::from(vec![
            Span::styled(name, theme.accent()),
            Span::styled(hint, theme.primary()),
            Span::styled(format!(" — {}", cmd.description), theme.base()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled("Global keys:", theme.highlight()));
    lines.push(Line::styled("  Enter        send message", theme.base()));
    lines.push(Line::styled("  ↑/↓          cycle input history", theme.base()));
    lines.push(Line::styled("  Tab          open slash menu", theme.base()));
    lines.push(Line::styled("  Ctrl+C / Ctrl+D  quit", theme.base()));
    lines.push(Line::from(""));
    lines.push(Line::styled("Press Esc, q, or Enter to close", theme.muted()));

    frame.render_widget(Paragraph::new(lines).style(theme.base()), inner);
}
