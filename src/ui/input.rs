use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Draw the composer input area. Supports multi-line content.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.border())
        .style(theme.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = "▸ ";
    let prompt_width = 2;

    let input_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(prompt_width), Constraint::Min(1)])
        .split(inner);

    let prompt_span = Span::styled(prompt, theme.user());
    let prompt_line = Line::from(vec![prompt_span]);
    frame.render_widget(Paragraph::new(prompt_line).style(theme.base()), input_layout[0]);

    // Compose display value with cursor indicator.
    let display = render_with_cursor(&app.input, app.cursor, theme);
    let placeholder = if app.input.is_empty() && !app.busy {
        "Type /help for commands or a message..."
    } else if app.busy {
        "Ctrl+C to interrupt..."
    } else {
        ""
    };

    let para = if app.input.is_empty() && !app.busy {
        Paragraph::new(Line::styled(placeholder.to_string(), theme.muted())).style(theme.base())
    } else {
        Paragraph::new(display).style(theme.base())
    };

    frame.render_widget(para, input_layout[1]);
}

/// Render the input string with the cursor highlighted.
/// `cursor` is a char index. Newlines split the input into multiple lines.
fn render_with_cursor(value: &str, cursor: usize, theme: crate::theme::Theme) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut line_chars: Vec<char> = Vec::new();
    let mut line_cursor: Option<usize> = None;
    let total_chars = value.chars().count();

    for (idx, c) in value.chars().enumerate() {
        if c == '\n' {
            lines.push(build_line(std::mem::take(&mut line_chars), line_cursor, theme));
            line_cursor = None;
        } else {
            if idx == cursor {
                line_cursor = Some(line_chars.len());
            }
            line_chars.push(c);
        }
    }

    // Cursor at the very end of the last non-empty line.
    if cursor == total_chars && !line_chars.is_empty() {
        line_cursor = Some(line_chars.len());
    }

    // Emit the final line only if it has content or a cursor.
    if !line_chars.is_empty() || line_cursor.is_some() {
        lines.push(build_line(line_chars, line_cursor, theme));
    }

    // If the input ends with a newline and the cursor is at the end, the cursor
    // belongs on a new empty line.
    if value.ends_with('\n') && cursor == total_chars {
        lines.push(build_line(vec![], Some(0), theme));
    }

    Text::from(lines)
}

/// Build a single rendered line, highlighting the cursor character if present.
fn build_line(chars: Vec<char>, cursor_col: Option<usize>, theme: crate::theme::Theme) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    let Some(pos) = cursor_col else {
        spans.push(Span::styled(chars.iter().collect::<String>(), theme.base()));
        return Line::from(spans);
    };
    let pos = pos.min(chars.len());

    let before: String = chars.iter().take(pos).collect();
    spans.push(Span::styled(before, theme.base()));

    if pos == chars.len() {
        spans.push(Span::styled(" ", Style::default().bg(theme.accent).fg(Color::Black)));
    } else {
        let cursor_char = chars.get(pos).copied().unwrap_or(' ').to_string();
        let rest: String = chars.iter().skip(pos + 1).collect();
        spans.push(Span::styled(
            cursor_char,
            Style::default().bg(theme.accent).fg(Color::Black).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(rest, theme.base()));
    }

    Line::from(spans)
}
