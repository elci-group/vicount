use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

use crate::app::App;
use crate::theme::Theme;
use crate::types::{Message, Role};

/// Render the chat transcript.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;

    // Measure how many lines the content would occupy.
    let text = build_message_text(&app.messages, theme, app.busy, app.spinner_tick);
    let line_count = text.lines.len();

    // Keep scroll pinned to bottom unless user scrolled up.
    let visible_height = area.height as usize;
    let max_scroll = line_count.saturating_sub(visible_height);
    if app.scroll == usize::MAX {
        app.scroll = max_scroll;
    }
    app.scroll = app.scroll.min(max_scroll);

    let block = Block::default().borders(Borders::NONE).style(theme.base());
    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll as u16, 0));

    frame.render_widget(para, area);

    // Vertical scrollbar.
    if max_scroll > 0 {
        let mut state = ScrollbarState::new(max_scroll).position(app.scroll);
        let sb = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        frame.render_stateful_widget(sb, area, &mut state);
    }
}

fn build_message_text(
    messages: &[Message],
    theme: Theme,
    busy: bool,
    tick: usize,
) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (idx, msg) in messages.iter().enumerate() {
        let is_last = idx == messages.len() - 1;
        match msg.role {
            Role::User => {
                let prefix = Span::styled("▸ ", theme.user());
                let content = Span::styled(msg.content.clone(), theme.user());
                lines.push(Line::from(vec![prefix, content]));
                lines.push(Line::from(""));
            }
            Role::Assistant => {
                let prefix = Span::styled("◆ ", theme.assistant());
                let mut content = msg.content.clone();
                if is_last && busy && msg.streaming {
                    content.push(' ');
                    content.push_str(spinner_frame(tick));
                }
                let text_spans = vec![prefix, Span::styled(content, theme.assistant())];
                lines.push(Line::from(text_spans));
                lines.push(Line::from(""));
            }
            Role::System => {
                let style = Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::ITALIC);
                for line in msg.content.lines() {
                    lines.push(Line::styled(line.to_string(), style));
                }
                lines.push(Line::from(""));
            }
        }
    }

    Text::from(lines)
}

fn spinner_frame(tick: usize) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[tick % FRAMES.len()]
}
