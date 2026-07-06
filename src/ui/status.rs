use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;

/// Draw the status bar at the bottom.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;

    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
        .split(area);

    // Left: session name / source.
    let left = Span::styled(
        format!("─ {} ", app.session_name),
        Style::default().fg(theme.border).bg(theme.background),
    );
    frame.render_widget(Paragraph::new(Line::from(left)).style(theme.base()), parts[0]);

    // Middle: transient status message or busy indicator.
    let middle_text = if !app.status_message.is_empty() {
        app.status_message.clone()
    } else if app.busy {
        format!("{} working...", spinner_frame(app.spinner_tick))
    } else {
        "ready".to_string()
    };
    let middle = Span::styled(middle_text, theme.muted().add_modifier(Modifier::DIM));
    frame.render_widget(Paragraph::new(Line::from(middle)).style(theme.base()), parts[1]);

    // Right: model, selected tools/skills, ViCo URL.
    let url = &app.vico_url;
    let prefix = format!(
        "{} │ skills:{} tools:{} │ ",
        app.model, app.selected_skills_count(), app.selected_tools_count()
    );
    let suffix = " ─";
    let available = area.width as usize;
    let fixed_width = prefix.width() + suffix.width();
    let max_url = available.saturating_sub(fixed_width);
    let display_url = if url.width() > max_url {
        if max_url > 3 {
            let take = max_url.saturating_sub(3);
            format!("{}...", url.chars().take(take).collect::<String>())
        } else {
            url.chars().take(max_url).collect()
        }
    } else {
        url.clone()
    };
    let right_text = format!("{}{}{}", prefix, display_url, suffix);
    let right_style = if app.vico.is_online() {
        theme.success()
    } else {
        theme.warning()
    };
    let right = Span::styled(right_text, right_style);
    frame.render_widget(Paragraph::new(Line::from(right)).style(theme.base()), parts[2]);
}

fn spinner_frame(tick: usize) -> &'static str {
    const FRAMES: &[&str] = &["◜", "◠", "◝", "◞", "◡", "◟"];
    FRAMES[tick % FRAMES.len()]
}
