use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::types::SlashCommand;

/// Draw the floating slash-command autocomplete menu.
pub fn draw(frame: &mut Frame, app: &App, matches: Vec<&SlashCommand>, anchor: Rect) {
    let theme = app.theme;
    let max_height = 12.min(matches.len() as u16 + 2).max(3);
    let width = anchor.width.clamp(30, 50);

    let area = Rect {
        x: anchor.x,
        y: anchor.y.saturating_sub(max_height),
        width,
        height: max_height,
    };

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" commands ")
        .title_style(theme.accent())
        .borders(Borders::ALL)
        .border_style(theme.primary())
        .style(theme.base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, cmd) in matches.iter().enumerate() {
        let selected = i == app.slash_cursor;
        let name = format!("/{}", cmd.name);
        let hint = if cmd.args_hint.is_empty() {
            String::new()
        } else {
            format!(" {}", cmd.args_hint)
        };
        let style = if selected {
            Style::default()
                .fg(theme.accent)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.base()
        };
        let desc_style = if selected {
            Style::default().fg(Color::White).bg(theme.selected_bg)
        } else {
            theme.muted()
        };
        lines.push(Line::from(vec![
            Span::styled(name, style),
            Span::styled(hint, desc_style),
        ]));
        lines.push(Line::styled(cmd.description.to_string(), desc_style));
    }

    frame.render_widget(Paragraph::new(lines).style(theme.base()), inner);
}
