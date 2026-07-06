use std::io;
use std::io::IsTerminal;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use vico_desktop_client::types::ContextMessage;

use crate::history;
use crate::theme::Theme;
use crate::types::{BackendResult, BackendTask, Message, Overlay, Role, SideTab, SlashCommand};
use crate::ui;
use crate::vico::SessionSummary;
use crate::vico::VicoClient;

/// A single selectable item in the side-panel checklist.
#[derive(Debug, Clone)]
pub struct CheckItem {
    pub name: String,
    pub description: String,
    pub selected: bool,
}

/// Main application state and event loop.
pub struct App {
    pub theme: Theme,
    pub vico: VicoClient,
    pub vico_url: String,
    pub should_quit: bool,

    // Chat transcript
    pub messages: Vec<Message>,
    pub scroll: usize,

    // Composer
    pub input: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub history_draft: String,

    // Side panel
    pub side_tab: SideTab,
    pub side_focused: bool,
    pub side_cursor: usize,
    pub skills: Vec<CheckItem>,
    pub tools: Vec<CheckItem>,

    // Overlays
    pub overlay: Overlay,
    pub model_picker_idx: usize,
    pub model_picker_providers: Vec<String>,
    pub session_picker_idx: usize,
    pub session_picker_items: Vec<SessionSummary>,
    pub quit_selected: bool,

    // Slash command autocomplete
    pub slash_open: bool,
    pub slash_cursor: usize,

    // Status / metadata
    pub busy: bool,
    pub status_message: String,
    pub status_until: Option<Instant>,
    pub session_name: String,
    pub session_id: Option<String>,
    pub model: String,
    pub turn_started: Option<Instant>,
    pub spinner_tick: usize,

    // Cancellation handle for the currently running backend turn.
    pub cancel_token: Option<CancellationToken>,

    // Channel used by async backend to push results back to the UI thread.
    pub result_tx: mpsc::Sender<BackendResult>,
    pub result_rx: mpsc::Receiver<BackendResult>,
}

/// The canonical slash command menu used by Vicount.
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "new",
        description: "Start a new session",
        args_hint: "[name]",
    },
    SlashCommand {
        name: "sessions",
        description: "Browse and resume sessions",
        args_hint: "",
    },
    SlashCommand {
        name: "model",
        description: "Switch model for this session",
        args_hint: "",
    },
    SlashCommand {
        name: "skills",
        description: "Focus/toggle skills checklist",
        args_hint: "",
    },
    SlashCommand {
        name: "tools",
        description: "Focus/toggle tools checklist",
        args_hint: "",
    },
    SlashCommand {
        name: "reset",
        description: "Clear session",
        args_hint: "",
    },
    SlashCommand {
        name: "plan",
        description: "Call ViCo /vico/atomise/plan",
        args_hint: "<prompt>",
    },
    SlashCommand {
        name: "run",
        description: "Call ViCo /orchestrate/submit",
        args_hint: "<prompt>",
    },
    SlashCommand {
        name: "status",
        description: "Show ViCo system status",
        args_hint: "",
    },
    SlashCommand {
        name: "help",
        description: "Show available commands",
        args_hint: "",
    },
    SlashCommand {
        name: "quit",
        description: "Exit Vicount",
        args_hint: "",
    },
];

impl App {
    pub fn new() -> Self {
        let (result_tx, result_rx) = mpsc::channel::<BackendResult>(64);
        let vico = VicoClient::new();
        let vico_url = vico.url();

        // Seed the side panels with demo items until the gateway can supply real lists.
        let skills = vec![
            CheckItem {
                name: "code-review".into(),
                description: "Review code changes".into(),
                selected: false,
            },
            CheckItem {
                name: "debug".into(),
                description: "Debug assistant".into(),
                selected: false,
            },
            CheckItem {
                name: "docs".into(),
                description: "Documentation writer".into(),
                selected: false,
            },
            CheckItem {
                name: "test-writer".into(),
                description: "Generate tests".into(),
                selected: false,
            },
            CheckItem {
                name: "planner".into(),
                description: "Task planner".into(),
                selected: false,
            },
        ];
        let tools = vec![
            CheckItem {
                name: "bash".into(),
                description: "Run shell commands".into(),
                selected: true,
            },
            CheckItem {
                name: "read_file".into(),
                description: "Read files".into(),
                selected: true,
            },
            CheckItem {
                name: "write_file".into(),
                description: "Write files".into(),
                selected: false,
            },
            CheckItem {
                name: "grep".into(),
                description: "Search files".into(),
                selected: false,
            },
            CheckItem {
                name: "web_search".into(),
                description: "Search the web".into(),
                selected: false,
            },
            CheckItem {
                name: "browser".into(),
                description: "Browser automation".into(),
                selected: false,
            },
        ];

        let mut app = Self {
            theme: Theme::default(),
            vico,
            vico_url,
            should_quit: false,
            messages: vec![],
            scroll: 0,
            input: String::new(),
            cursor: 0,
            history: history::load_history(history::MAX_HISTORY),
            history_idx: None,
            history_draft: String::new(),
            side_tab: SideTab::Skills,
            side_focused: false,
            side_cursor: 0,
            skills,
            tools,
            overlay: Overlay::None,
            model_picker_idx: 0,
            model_picker_providers: vec![
                "anthropic/claude-sonnet-4".into(),
                "openai/gpt-4o".into(),
                "google/gemini-2.5-pro".into(),
                "xai/grok-3".into(),
            ],
            session_picker_idx: 0,
            session_picker_items: vec![],
            quit_selected: false,
            slash_open: false,
            slash_cursor: 0,
            busy: false,
            status_message: String::new(),
            status_until: None,
            session_name: "new session".into(),
            session_id: None,
            model: "anthropic/claude-sonnet-4".into(),
            turn_started: None,
            spinner_tick: 0,
            cancel_token: None,
            result_tx,
            result_rx,
        };

        app.add_system_message("Welcome to Vicount. Type /help for commands.".into());
        if !app.vico.is_online() {
            app.add_system_message(
                "Offline mode — VICO_DESKTOP_URL is not set. Operating in echo/demo mode.".into(),
            );
        }
        app
    }

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(120);

        // Initial render.
        terminal.draw(|f| ui::draw(f, self))?;

        while !self.should_quit {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            // Process backend results first so the UI is responsive.
            while let Ok(result) = self.result_rx.try_recv() {
                self.apply_backend_result(result);
            }

            // Process input events.
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.on_key(key.code, key.modifiers).await;
                    }
                }
            }

            // Spinner tick.
            if last_tick.elapsed() >= tick_rate {
                if self.busy {
                    self.spinner_tick = self.spinner_tick.wrapping_add(1);
                }
                last_tick = Instant::now();
            }

            // Clear transient status messages after a few seconds.
            if let Some(until) = self.status_until {
                if Instant::now() >= until {
                    self.status_message.clear();
                    self.status_until = None;
                }
            }

            terminal.draw(|f| ui::draw(f, self))?;
        }

        terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        Ok(())
    }

    /// Handle a key press.
    async fn on_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Overlays consume keys first.
        if self.overlay != Overlay::None {
            self.handle_overlay_key(code, modifiers).await;
            return;
        }

        // Slash autocomplete.
        if self.slash_open {
            self.handle_slash_key(code, modifiers).await;
            return;
        }

        // Side panel focus.
        if self.side_focused {
            self.handle_side_panel_key(code, modifiers);
            return;
        }

        // Global shortcuts.
        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('c') => {
                    if self.busy {
                        self.cancel_turn();
                    } else {
                        self.overlay = Overlay::Quit;
                    }
                    return;
                }
                KeyCode::Char('d') => {
                    self.overlay = Overlay::Quit;
                    return;
                }
                KeyCode::Char('l') => return, // redraw handled by next frame
                _ => {}
            }
        }

        // Composer input (cursor is a char index).
        match code {
            KeyCode::Enter => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.insert_char('\n');
                } else {
                    self.submit_input().await;
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte = char_to_byte(&self.input, self.cursor);
                    let prev_byte = char_to_byte(&self.input, self.cursor - 1);
                    self.input.replace_range(prev_byte..byte, "");
                    self.cursor -= 1;
                    self.check_slash_open();
                }
            }
            KeyCode::Delete => {
                if self.cursor < char_len(&self.input) {
                    let byte = char_to_byte(&self.input, self.cursor);
                    let next_byte = char_to_byte(&self.input, self.cursor + 1);
                    self.input.replace_range(byte..next_byte, "");
                    self.check_slash_open();
                }
            }
            KeyCode::Left => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.cursor = word_left(&self.input, self.cursor);
                } else {
                    self.cursor = self.cursor.saturating_sub(1);
                }
            }
            KeyCode::Right => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.cursor = word_right(&self.input, self.cursor);
                } else {
                    self.cursor = (self.cursor + 1).min(char_len(&self.input));
                }
            }
            KeyCode::Home => self.cursor = self.current_line_start(),
            KeyCode::End => self.cursor = self.current_line_end(),
            KeyCode::Up => {
                if self.current_line() == 0 {
                    self.cycle_history(-1);
                } else {
                    self.cursor = self.cursor_up();
                }
            }
            KeyCode::Down => {
                if self.current_line() + 1 >= self.input_line_count() {
                    self.cycle_history(1);
                } else {
                    self.cursor = self.cursor_down();
                }
            }
            KeyCode::Tab => {
                if self.input.starts_with('/') {
                    self.slash_open = true;
                    self.slash_cursor = 0;
                }
            }
            KeyCode::Esc => {
                self.side_focused = false;
            }
            _ => {}
        }
    }

    fn insert_char(&mut self, c: char) {
        let byte = char_to_byte(&self.input, self.cursor);
        self.input.insert(byte, c);
        self.cursor += 1;
        self.check_slash_open();
    }

    fn check_slash_open(&mut self) {
        if self.input.starts_with('/') {
            self.slash_open = true;
            self.slash_cursor = 0;
        } else {
            self.slash_open = false;
        }
    }

    async fn handle_slash_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let matches = self.slash_matches();
        match code {
            KeyCode::Tab | KeyCode::Down => {
                if !matches.is_empty() {
                    self.slash_cursor = (self.slash_cursor + 1) % matches.len();
                }
            }
            KeyCode::Up => {
                if !matches.is_empty() {
                    self.slash_cursor = (self.slash_cursor + matches.len() - 1) % matches.len();
                }
            }
            KeyCode::Enter => {
                if let Some(cmd) = matches.get(self.slash_cursor) {
                    let name = cmd.name.to_string();
                    let rest = self
                        .input
                        .strip_prefix(&format!("/{}", name))
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    self.input.clear();
                    self.cursor = 0;
                    self.slash_open = false;
                    self.run_slash(&name, &rest).await;
                    return;
                }
                // No match: treat as ordinary message.
                self.slash_open = false;
                self.submit_input().await;
            }
            KeyCode::Esc => {
                self.slash_open = false;
            }
            KeyCode::Char(c) => {
                let byte = char_to_byte(&self.input, self.cursor);
                self.input.insert(byte, c);
                self.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte = char_to_byte(&self.input, self.cursor);
                    let prev_byte = char_to_byte(&self.input, self.cursor - 1);
                    self.input.replace_range(prev_byte..byte, "");
                    self.cursor -= 1;
                    if !self.input.starts_with('/') {
                        self.slash_open = false;
                    }
                }
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(char_len(&self.input)),
            _ => {}
        }
        // Suppress Ctrl+C while the menu is open.
        let _ = modifiers;
    }

    pub fn slash_matches(&self) -> Vec<&SlashCommand> {
        let prefix = self.input.to_lowercase();
        SLASH_COMMANDS
            .iter()
            .filter(|c| format!("/{}", c.name).starts_with(&prefix))
            .collect()
    }

    async fn handle_overlay_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match self.overlay {
            Overlay::Help => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter) {
                    self.overlay = Overlay::None;
                }
            }
            Overlay::ModelPicker => self.handle_model_picker_key(code),
            Overlay::SessionPicker => self.handle_session_picker_key(code),
            Overlay::Quit => self.handle_quit_key(code),
            Overlay::None => {}
        }
        let _ = modifiers;
    }

    fn handle_model_picker_key(&mut self, code: KeyCode) {
        let n = self.model_picker_providers.len().max(1);
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.model_picker_idx = self.model_picker_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.model_picker_idx = (self.model_picker_idx + 1).min(n - 1);
            }
            KeyCode::Enter => {
                if let Some(m) = self.model_picker_providers.get(self.model_picker_idx) {
                    self.model = m.clone();
                    self.set_status(format!("Model set to {}", m));
                }
                self.overlay = Overlay::None;
            }
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            _ => {}
        }
    }

    fn handle_session_picker_key(&mut self, code: KeyCode) {
        let n = self.session_picker_items.len().max(1);
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.session_picker_idx = self.session_picker_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.session_picker_idx = (self.session_picker_idx + 1).min(n - 1);
            }
            KeyCode::Enter => {
                let selected = self
                    .session_picker_items
                    .get(self.session_picker_idx)
                    .cloned();
                if let Some(s) = selected {
                    self.session_name = s.name.clone();
                    self.session_id = Some(s.session_id.clone());
                    self.vico.session_id = Some(s.session_id.clone());
                    self.load_session_history(&s.session_id);
                    self.set_status(format!("Resumed session {}", s.name));
                }
                self.overlay = Overlay::None;
            }
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            _ => {}
        }
    }

    fn load_session_history(&mut self, session_id: &str) {
        let tx = self.result_tx.clone();
        let vico = self.vico.clone();
        let id = session_id.to_string();
        tokio::spawn(async move {
            match vico.load_session_history(&id).await {
                Ok(messages) => {
                    let _ = tx.send(BackendResult::SetMessages { messages }).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(BackendResult::Failed {
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        });
    }

    fn handle_quit_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left | KeyCode::Right => self.quit_selected = !self.quit_selected,
            KeyCode::Enter => {
                if self.quit_selected {
                    self.should_quit = true;
                } else {
                    self.overlay = Overlay::None;
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            KeyCode::Char('y') => self.should_quit = true,
            KeyCode::Char('n') => self.overlay = Overlay::None,
            _ => {}
        }
    }

    fn handle_side_panel_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        let items = self.side_items();
        let n = items.len().max(1);
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.side_cursor = self.side_cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.side_cursor = (self.side_cursor + 1).min(n - 1);
            }
            KeyCode::Tab => {
                self.side_tab = match self.side_tab {
                    SideTab::Skills => SideTab::Tools,
                    SideTab::Tools => SideTab::Skills,
                };
                self.side_cursor = 0;
            }
            KeyCode::Char(' ') => {
                self.toggle_side_item();
            }
            KeyCode::Enter => {
                self.side_focused = false;
                let count = self.selected_skills_count() + self.selected_tools_count();
                self.set_status(format!(
                    "Applied {} selection{}",
                    count,
                    if count == 1 { "" } else { "s" }
                ));
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.side_focused = false;
            }
            _ => {}
        }
    }

    pub fn side_items(&self) -> &[CheckItem] {
        match self.side_tab {
            SideTab::Skills => &self.skills,
            SideTab::Tools => &self.tools,
        }
    }

    fn side_items_mut(&mut self) -> &mut Vec<CheckItem> {
        match self.side_tab {
            SideTab::Skills => &mut self.skills,
            SideTab::Tools => &mut self.tools,
        }
    }

    fn toggle_side_item(&mut self) {
        let cursor = self.side_cursor;
        let items = self.side_items_mut();
        if let Some(item) = items.get_mut(cursor) {
            item.selected = !item.selected;
        }
    }

    pub fn selected_skills_count(&self) -> usize {
        self.skills.iter().filter(|i| i.selected).count()
    }

    pub fn selected_tools_count(&self) -> usize {
        self.tools.iter().filter(|i| i.selected).count()
    }

    /// Submit the current input as a chat message.
    async fn submit_input(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.input.clear();
        self.cursor = 0;
        self.slash_open = false;
        self.history.push(text.clone());
        self.history_idx = None;
        self.history_draft.clear();
        if let Err(e) = history::append_history(&text) {
            warn!("failed to persist history: {e}");
        }

        if text.starts_with('/') {
            let trimmed = text.strip_prefix('/').unwrap_or("");
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").to_lowercase();
            let rest = parts.next().unwrap_or("").trim();
            self.run_slash(&name, rest).await;
            return;
        }

        self.add_user_message(text.clone());
        self.spawn_backend(BackendTask::Chat { prompt: text });
    }

    /// Execute a slash command.
    async fn run_slash(&mut self, name: &str, rest: &str) {
        debug!("slash /{name} {rest}");
        match name {
            "new" => {
                self.session_name = if rest.is_empty() {
                    "new session".into()
                } else {
                    rest.to_string()
                };
                self.messages.retain(|m| m.role == Role::System);
                self.scroll = 0;
                let name = self.session_name.clone();
                let mut vico = self.vico.clone();
                tokio::spawn(async move {
                    match vico.create_session(&name).await {
                        Ok(id) => info!("created session {id}"),
                        Err(e) => warn!("failed to create server session: {e}"),
                    }
                });
                self.set_status(format!("Started {}", self.session_name));
            }
            "sessions" => {
                self.overlay = Overlay::SessionPicker;
                self.session_picker_idx = 0;
                let tx = self.result_tx.clone();
                let vico = self.vico.clone();
                tokio::spawn(async move {
                    match vico.list_sessions(Some(50)).await {
                        Ok(items) => {
                            let _ = tx.send(BackendResult::SessionList { items }).await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(BackendResult::Failed {
                                    error: e.to_string(),
                                })
                                .await;
                        }
                    }
                });
            }
            "model" => {
                self.overlay = Overlay::ModelPicker;
                self.model_picker_idx = 0;
            }
            "skills" => {
                self.side_tab = SideTab::Skills;
                self.side_focused = !self.side_focused;
                self.side_cursor = 0;
            }
            "tools" => {
                self.side_tab = SideTab::Tools;
                self.side_focused = !self.side_focused;
                self.side_cursor = 0;
            }
            "reset" => {
                self.messages.retain(|m| m.role == Role::System);
                self.scroll = 0;
                self.set_status("Session cleared".into());
            }
            "plan" => {
                if rest.is_empty() {
                    self.set_status("Usage: /plan <prompt>".into());
                    return;
                }
                self.add_user_message(format!("/plan {}", rest));
                self.spawn_backend(BackendTask::Plan {
                    prompt: rest.to_string(),
                });
            }
            "run" => {
                if rest.is_empty() {
                    self.set_status("Usage: /run <prompt>".into());
                    return;
                }
                self.add_user_message(format!("/run {}", rest));
                self.spawn_backend(BackendTask::Run {
                    prompt: rest.to_string(),
                });
            }
            "status" => {
                self.spawn_backend(BackendTask::Status);
            }
            "help" => {
                self.overlay = Overlay::Help;
            }
            "quit" | "exit" => {
                self.overlay = Overlay::Quit;
            }
            _ => {
                self.set_status(format!("Unknown command /{name}"));
            }
        }
    }

    fn spawn_backend(&mut self, task: BackendTask) {
        self.busy = true;
        self.turn_started = Some(Instant::now());
        self.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            streaming: true,
        });
        let cancel = CancellationToken::new();
        self.cancel_token = Some(cancel.clone());
        let tx = self.result_tx.clone();
        let vico = self.vico.clone();
        let context = self.build_context();
        tokio::spawn(async move {
            run_backend_task(task, vico, context, tx, cancel).await;
        });
    }

    fn build_context(&self) -> Vec<ContextMessage> {
        self.messages
            .iter()
            .filter(|m| !m.streaming && m.role != Role::System)
            .map(|m| ContextMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
                agent: None,
            })
            .collect()
    }

    fn apply_backend_result(&mut self, result: BackendResult) {
        match result {
            BackendResult::Append { text } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == Role::Assistant {
                        last.content.push_str(&text);
                        return;
                    }
                }
                self.messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    streaming: true,
                });
            }
            BackendResult::Done => {
                if let Some(last) = self.messages.last_mut() {
                    last.streaming = false;
                }
                self.busy = false;
                self.turn_started = None;
                self.cancel_token = None;
            }
            BackendResult::Failed { error } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == Role::Assistant && last.streaming {
                        last.content.push_str(&format!("\n[error: {}]", error));
                        last.streaming = false;
                    } else {
                        self.messages.push(Message {
                            role: Role::Assistant,
                            content: format!("[error: {}]", error),
                            streaming: false,
                        });
                    }
                } else {
                    self.messages.push(Message {
                        role: Role::Assistant,
                        content: format!("[error: {}]", error),
                        streaming: false,
                    });
                }
                self.busy = false;
                self.turn_started = None;
                self.cancel_token = None;
            }
            BackendResult::SessionList { items } => {
                self.session_picker_items = items;
            }
            BackendResult::SetMessages { messages } => {
                self.messages.retain(|m| m.role == Role::System);
                self.messages.extend(messages);
                self.scroll_to_bottom();
            }
        }
    }

    fn add_user_message(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::User,
            content,
            streaming: false,
        });
        self.scroll_to_bottom();
    }

    fn add_system_message(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::System,
            content,
            streaming: false,
        });
    }

    fn set_status(&mut self, msg: String) {
        self.status_message = msg;
        self.status_until = Some(Instant::now() + Duration::from_secs(5));
    }

    fn cycle_history(&mut self, dir: i32) {
        if self.history.is_empty() {
            return;
        }
        match dir {
            -1 => {
                if self.history_idx.is_none() {
                    self.history_draft = self.input.clone();
                    self.history_idx = Some(self.history.len() - 1);
                } else if self.history_idx.unwrap() > 0 {
                    self.history_idx = Some(self.history_idx.unwrap() - 1);
                }
            }
            1 => {
                if let Some(idx) = self.history_idx {
                    if idx + 1 < self.history.len() {
                        self.history_idx = Some(idx + 1);
                    } else {
                        self.history_idx = None;
                    }
                }
            }
            _ => {}
        }
        if let Some(idx) = self.history_idx {
            self.input = self.history[idx].clone();
        } else {
            self.input = self.history_draft.clone();
        }
        self.cursor = char_len(&self.input);
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = usize::MAX;
    }

    /// Cancel the currently running backend turn, if any.
    fn cancel_turn(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
            self.set_status("Cancelling...".into());
        }
    }

    /// Number of logical lines in the composer input (\n-separated).
    pub fn input_line_count(&self) -> usize {
        self.input.lines().count().max(1)
    }

    /// Return the line index (0-based) that contains the cursor.
    fn current_line(&self) -> usize {
        self.input
            .chars()
            .take(self.cursor)
            .filter(|&c| c == '\n')
            .count()
    }

    /// Char index of the start of the line containing the cursor.
    fn current_line_start(&self) -> usize {
        self.input
            .chars()
            .enumerate()
            .take(self.cursor)
            .filter(|(_, c)| *c == '\n')
            .last()
            .map(|(i, _)| i + 1)
            .unwrap_or(0)
            .min(char_len(&self.input))
    }

    /// Char index just past the last character on the line containing the cursor.
    fn current_line_end(&self) -> usize {
        let after_cursor = self.input.chars().skip(self.cursor);
        let offset_to_eol = after_cursor.take_while(|&c| c != '\n').count();
        (self.cursor + offset_to_eol).min(char_len(&self.input))
    }

    /// Move the cursor up one line, preserving the visual column when possible.
    fn cursor_up(&self) -> usize {
        let line = self.current_line();
        if line == 0 {
            return self.cursor;
        }
        let col = self.cursor - self.current_line_start();
        self.line_col_to_cursor(line - 1, col)
    }

    /// Move the cursor down one line, preserving the visual column when possible.
    fn cursor_down(&self) -> usize {
        let line = self.current_line();
        if line + 1 >= self.input_line_count() {
            return self.cursor;
        }
        let col = self.cursor - self.current_line_start();
        self.line_col_to_cursor(line + 1, col)
    }

    /// Convert a (line, column) pair into a char cursor index.
    fn line_col_to_cursor(&self, line: usize, col: usize) -> usize {
        let mut current_line = 0;
        let mut current_col = 0;
        for (idx, c) in self.input.chars().enumerate() {
            if current_line == line {
                if current_col >= col || c == '\n' {
                    return idx;
                }
                current_col += 1;
            } else if c == '\n' {
                current_line += 1;
            }
        }
        char_len(&self.input)
    }
}

async fn run_backend_task(
    task: BackendTask,
    vico: VicoClient,
    context: Vec<ContextMessage>,
    tx: mpsc::Sender<BackendResult>,
    cancel: CancellationToken,
) {
    match task {
        BackendTask::Chat { prompt } => match vico.chat_stream(&prompt, context, cancel).await {
            Ok(mut rx) => {
                while let Some(chunk) = rx.recv().await {
                    match chunk {
                        Ok(text) => {
                            if tx.send(BackendResult::Append { text }).await.is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(BackendResult::Failed {
                                    error: e.to_string(),
                                })
                                .await;
                            let _ = tx.send(BackendResult::Done).await;
                            return;
                        }
                    }
                }
                let _ = tx.send(BackendResult::Done).await;
            }
            Err(e) => {
                let _ = tx
                    .send(BackendResult::Failed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = tx.send(BackendResult::Done).await;
            }
        },
        BackendTask::Plan { prompt } => match vico.plan(&prompt, context).await {
            Ok(text) => {
                let _ = tx.send(BackendResult::Append { text }).await;
                let _ = tx.send(BackendResult::Done).await;
            }
            Err(e) => {
                let _ = tx
                    .send(BackendResult::Failed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = tx.send(BackendResult::Done).await;
            }
        },
        BackendTask::Run { prompt } => match vico.orchestrate_submit(&prompt).await {
            Ok(text) => {
                let _ = tx.send(BackendResult::Append { text }).await;
                let _ = tx.send(BackendResult::Done).await;
            }
            Err(e) => {
                let _ = tx
                    .send(BackendResult::Failed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = tx.send(BackendResult::Done).await;
            }
        },
        BackendTask::Status => match vico.system_status().await {
            Ok(text) => {
                let _ = tx.send(BackendResult::Append { text }).await;
                let _ = tx.send(BackendResult::Done).await;
            }
            Err(e) => {
                let _ = tx
                    .send(BackendResult::Failed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = tx.send(BackendResult::Done).await;
            }
        },
    }
}

/// Convert a char index to a byte index in `s`.
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Number of chars in `s`.
fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Move a char cursor one word to the left.
fn word_left(s: &str, char_pos: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut i = char_pos.saturating_sub(1);
    while i > 0 && chars.get(i) == Some(&' ') {
        i -= 1;
    }
    while i > 0 && chars.get(i.saturating_sub(1)) != Some(&' ') {
        i -= 1;
    }
    i
}

/// Move a char cursor one word to the right.
fn word_right(s: &str, char_pos: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut i = char_pos;
    while i < chars.len() && chars.get(i) == Some(&' ') {
        i += 1;
    }
    while i < chars.len() && chars.get(i) != Some(&' ') {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_input(input: &str, cursor: usize) -> App {
        let mut app = App::new();
        app.input = input.to_string();
        app.cursor = cursor;
        app
    }

    #[test]
    fn cursor_line_helpers_single_line() {
        let app = app_with_input("hello", 0);
        assert_eq!(app.current_line(), 0);
        assert_eq!(app.current_line_start(), 0);
        assert_eq!(app.current_line_end(), 5);

        let app = app_with_input("hello", 5);
        assert_eq!(app.current_line(), 0);
        assert_eq!(app.current_line_start(), 0);
        assert_eq!(app.current_line_end(), 5);
    }

    #[test]
    fn cursor_line_helpers_multi_line() {
        let app = app_with_input("hello\nworld", 2);
        assert_eq!(app.current_line(), 0);
        assert_eq!(app.current_line_start(), 0);
        assert_eq!(app.current_line_end(), 5);

        let app = app_with_input("hello\nworld", 6);
        assert_eq!(app.current_line(), 1);
        assert_eq!(app.current_line_start(), 6);
        assert_eq!(app.current_line_end(), 11);
    }

    #[test]
    fn cursor_up_down_preserves_column() {
        let app = app_with_input("hello\nhi", 7); // 'i' on second line, col 1
        assert_eq!(app.cursor_down(), 7); // already on last line
        assert_eq!(app.cursor_up(), 1); // 'e' on first line, col 1

        let app = app_with_input("hi\nhello", 1); // col 1 on first line
        assert_eq!(app.cursor_down(), 4); // second line col 1 ('e')
    }

    #[test]
    fn cursor_up_down_clamps_to_shorter_line() {
        let app = app_with_input("hi\nhello", 6); // col 4 on second line ('o')
        assert_eq!(app.cursor_up(), 2); // first line only has cols 0-1, clamp to end
    }

    #[test]
    fn input_line_count() {
        let app = app_with_input("one\ntwo\nthree", 0);
        assert_eq!(app.input_line_count(), 3);

        let app = app_with_input("", 0);
        assert_eq!(app.input_line_count(), 1);
    }

    #[test]
    fn slash_matches_filters_by_prefix() {
        let mut app = App::new();
        app.input = "/ne".into();
        let matches: Vec<_> = app.slash_matches().into_iter().map(|c| c.name).collect();
        assert!(matches.contains(&"new"));
        assert!(!matches.contains(&"help"));
    }

    #[test]
    fn slash_matches_empty_prefix_lists_all() {
        let mut app = App::new();
        app.input = "/".into();
        assert_eq!(app.slash_matches().len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn insert_char_updates_input_and_cursor() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('b');
        assert_eq!(app.input, "ab");
        assert_eq!(app.cursor, 2);
    }

    #[test]
    fn char_to_byte_roundtrip() {
        assert_eq!(char_to_byte("hello", 0), 0);
        assert_eq!(char_to_byte("hello", 5), 5);
        assert_eq!(char_to_byte("héllo", 4), 5); // é is two bytes; last char is byte 5
        assert_eq!(char_to_byte("héllo", 99), 6);
    }

    #[test]
    fn word_left_skips_spaces_and_word() {
        assert_eq!(word_left("hello world", 11), 6);
        assert_eq!(word_left("hello world", 5), 0);
        assert_eq!(word_left("", 0), 0);
    }

    #[test]
    fn word_right_skips_spaces_and_word() {
        assert_eq!(word_right("hello world", 0), 5);
        assert_eq!(word_right("hello world", 5), 11);
        assert_eq!(word_right("hello world", 6), 11);
        assert_eq!(word_right("", 0), 0);
    }
}

/// Run the TUI application.
pub async fn run_app() -> Result<()> {
    info!("starting Vicount TUI");
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("Vicount TUI requires an interactive terminal (try `vicount -p \"hello\"` for non-interactive mode)");
    }
    let mut app = App::new();
    app.run().await
}
