pub mod chat;
pub mod help;
pub mod input;
pub mod messages;
pub mod model_picker;
pub mod quit;
pub mod session_picker;
pub mod side_panel;
pub mod slash_menu;
pub mod status;

use ratatui::Frame;

use crate::app::App;

/// Render the full UI into the given frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    chat::draw(frame, app);
}
