use serde::{Deserialize, Serialize};

/// A chat message displayed in the transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// When true the assistant message is still being streamed/appended.
    #[serde(default)]
    pub streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// Side panel tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideTab {
    Skills,
    Tools,
}

impl std::fmt::Display for SideTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SideTab::Skills => write!(f, "Skills"),
            SideTab::Tools => write!(f, "Tools"),
        }
    }
}

/// Overlay currently displayed on top of the chat UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
    ModelPicker,
    #[allow(dead_code)]
    SessionPicker,
    Quit,
}

/// A slash command definition.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub args_hint: &'static str,
}

/// Work the async backend can request.
#[derive(Debug, Clone)]
pub enum BackendTask {
    Chat { prompt: String },
    Plan { prompt: String },
    Run { prompt: String },
    Status,
}

/// Result delivered from the async backend to the UI.
#[derive(Debug, Clone)]
pub enum BackendResult {
    Append {
        text: String,
    },
    Done,
    Failed {
        error: String,
    },
    SessionList {
        items: Vec<crate::vico::SessionSummary>,
    },
    SetMessages {
        messages: Vec<Message>,
    },
}
