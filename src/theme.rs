use ratatui::style::{Color, Modifier, Style};

/// Hermes-like dark theme with accent colors for the Vicount TUI.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub primary: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub user_prompt: Color,
    pub assistant_prompt: Color,
    pub highlight: Color,
    pub selected_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Black,
            foreground: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(120, 120, 120),
            accent: Color::Rgb(255, 95, 162),
            primary: Color::Rgb(100, 200, 255),
            success: Color::Rgb(80, 230, 140),
            warning: Color::Rgb(255, 200, 80),
            error: Color::Rgb(255, 90, 90),
            border: Color::Rgb(80, 80, 80),
            user_prompt: Color::Rgb(255, 200, 100),
            assistant_prompt: Color::Rgb(100, 200, 255),
            highlight: Color::Rgb(255, 255, 100),
            selected_bg: Color::Rgb(40, 40, 60),
        }
    }
}

impl Theme {
    pub fn base(self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }

    pub fn muted(self) -> Style {
        Style::default().fg(self.muted).bg(self.background)
    }

    pub fn accent(self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    pub fn primary(self) -> Style {
        Style::default().fg(self.primary).bg(self.background)
    }

    pub fn success(self) -> Style {
        Style::default().fg(self.success).bg(self.background)
    }

    pub fn warning(self) -> Style {
        Style::default().fg(self.warning).bg(self.background)
    }

    pub fn error(self) -> Style {
        Style::default().fg(self.error).bg(self.background)
    }

    pub fn border(self) -> Style {
        Style::default().fg(self.border).bg(self.background)
    }

    pub fn user(self) -> Style {
        Style::default()
            .fg(self.user_prompt)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    pub fn assistant(self) -> Style {
        Style::default()
            .fg(self.assistant_prompt)
            .bg(self.background)
    }

    pub fn selected(self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlight(self) -> Style {
        Style::default()
            .fg(self.highlight)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_has_expected_colors() {
        let t = Theme::default();
        assert_eq!(t.background, Color::Black);
        assert_eq!(t.foreground, Color::Rgb(220, 220, 220));
        assert_eq!(t.accent, Color::Rgb(255, 95, 162));
    }

    #[test]
    fn base_style_uses_foreground_and_background() {
        let t = Theme::default();
        let style = t.base();
        assert_eq!(style.fg, Some(t.foreground));
        assert_eq!(style.bg, Some(t.background));
    }

    #[test]
    fn selected_style_is_bold() {
        let t = Theme::default();
        let style = t.selected();
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }
}
