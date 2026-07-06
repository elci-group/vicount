use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::types::SideTab;

/// Draw the skills/tools side panel checklist.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let focused = app.side_focused;

    let border_style = if focused {
        theme.accent()
    } else {
        theme.border()
    };
    let block = Block::default()
        .title(" Hub ")
        .title_style(if focused {
            theme.accent()
        } else {
            theme.primary()
        })
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(theme.base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 4 {
        return;
    }

    // Tabs.
    let titles = vec!["Skills", "Tools"];
    let tabs = Tabs::new(titles)
        .select(match app.side_tab {
            SideTab::Skills => 0,
            SideTab::Tools => 1,
        })
        .highlight_style(theme.selected())
        .divider(" │ ")
        .style(theme.base());

    let tab_height = 3;
    let tab_area = Rect {
        height: tab_height,
        ..inner
    };
    let list_area = Rect {
        y: inner.y + tab_height,
        height: inner.height.saturating_sub(tab_height),
        ..inner
    };

    frame.render_widget(tabs, tab_area);

    // Header hint.
    let header = Line::styled("↑↓ navigate · SPACE toggle · ENTER apply", theme.muted());
    let header_area = Rect {
        y: list_area.y,
        height: 1,
        ..list_area
    };
    frame.render_widget(Paragraph::new(header).style(theme.base()), header_area);

    // Items.
    let items_area = Rect {
        y: list_area.y + 1,
        height: list_area.height.saturating_sub(1),
        ..list_area
    };

    let items = app.side_items().to_vec();
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let selected = i == app.side_cursor;
        let mark = if item.selected { "✓" } else { " " };
        let arrow = if selected { "→" } else { " " };
        let label = format!(" {} [{}] {}", arrow, mark, item.name);
        let style = if selected {
            theme.selected()
        } else {
            Style::default().fg(theme.foreground).bg(theme.background)
        };
        lines.push(Line::styled(label, style));

        if !item.description.is_empty() && items_area.width > 6 {
            let desc = format!("   {}", item.description);
            lines.push(Line::styled(desc, theme.muted()));
        }
    }

    if lines.is_empty() {
        lines.push(Line::styled("No items available", theme.muted()));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(theme.base())
            .wrap(Wrap { trim: false }),
        items_area,
    );
}
