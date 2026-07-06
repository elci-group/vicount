:# Vicount

Vicount is a chat-first terminal UI for [ViCo Desktop](https://github.com/scotia). It is a Rust rewrite of the Hermes-style Ink interface: slash commands, side-panel skill/tool checklists, streaming responses, and session management.

## Features

- **Ratatui/crossterm** event loop with alternate-screen raw mode
- **Slash commands**: `/new`, `/sessions`, `/model`, `/skills`, `/tools`, `/reset`, `/plan`, `/run`, `/status`, `/help`, `/quit`
- **Multi-line composer** with `Shift+Enter` for newlines
- **Persistent input history** stored in `~/.local/share/vicount/history.jsonl`
- **True streaming** assistant responses via `/vico/chat/stream` WebSocket
- **Session browser** (`/sessions`) with server-side create/resume/load
- **Cancellation** of in-flight turns with `Ctrl+C`
- **Offline/demo fallback** when `VICO_DESKTOP_URL` is not set

## Requirements

- Rust 1.78+ (2021 edition)
- A running ViCo Desktop instance for full functionality (optional)

## Install

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/elci-group/vicount/main/scripts/install.sh | sh
```

Or build from source:

```bash
git clone https://github.com/YOURUSER/vicount
cd vicount
cargo build --release
# Binary is at target/release/vicount
```

## Usage

```bash
# Interactive TUI
vicount

# Non-interactive prompt
vicount -p "Explain this codebase to me"
```

## Environment

- `VICO_DESKTOP_URL` — URL of the ViCo Desktop gateway (default: offline mode)

## Development

```bash
cargo check
cargo test
cargo build --release
```

## License

MIT
