# Vicount Roadmap — SOTA UX & Performance for Hermes in Rust

## Vision
Vicount is a Rust rewrite of Hermes that preserves the chat-first, slash-command-heavy Ink TUI UX while routing all inference, skills, tools, and orchestration through ViCo Desktop.

## Phase 1 — Faithful Core (Complete)
- [x] Ratatui/crossterm event loop with alternate-screen raw mode
- [x] Chat-centric layout: side panel (skills/tools checklist), main transcript, bottom input, status bar
- [x] Slash command autocomplete (`/new`, `/model`, `/skills`, `/tools`, `/reset`, `/plan`, `/run`, `/status`, `/help`, `/quit`)
- [x] Skills/Tools checklist panel with `↑/↓`, `Space` toggle, `Enter` apply
- [x] Modal overlays: model picker, session picker, help, quit
- [x] Hermes-like dark theme with accent colors and busy indicator
- [x] ViCo Desktop integration via `vico-desktop-client` (`chat`, `plan`, `orchestrate_submit`, `rag_search`, `system_status`)
- [x] Non-interactive `vicount -p "prompt"` mode
- [x] Offline/demo fallback when `VICO_DESKTOP_URL` is unset

## Phase 2 — UX Parity with Hermes
- [x] Persistent input history with `↑/↓` recall and draft preservation
- [x] True multi-line composer with `Shift+Enter` newline, word navigation, line-aware cursor movement
- [ ] Text selection and clipboard integration in composer
- [ ] Kaomoji / emoji / unicode / ascii busy indicator styles (`/indicator`)
- [x] Streaming assistant responses
- [ ] Session titles, resume by name/ID, and session browser
- [ ] Per-session model/provider override (`/model <provider>/<model>`)
- [ ] Background tasks HUD and `/agents` overlay
- [ ] Live tool progress display (`/verbose` levels)
- [ ] Sticky prompt tracker and transcript scrollbar
- [ ] Queued messages (`/queue`, `/steer`) while agent is busy

## Phase 3 — ViCo-Native Performance
- [x] Streaming `/vico/chat/stream` integration
- [ ] `/vico/atomise/execute` for plan execution
- [ ] `/orchestrate/submit` multi-agent task graphs with dependency visualization
- [ ] `/rag/search` and `/rag/ingest` for skills/project context
- [ ] `/user/history` and `/user/preferences` sync
- [ ] Cancellation and graceful interruption of in-flight turns
- [ ] Memory monitor and heap-dump guard (inspired by Hermes TUI)

## Phase 4 — SOTA Polish
- [ ] Mouse support, hyperlink click handling, OSC 52 clipboard
- [ ] Config file (`~/.config/vicount/config.toml`) for providers, skills dirs, keybindings
- [ ] Gateway integration for Telegram/Discord-style command surfaces
- [ ] Skill hub: install, inspect, and pin skills
- [ ] Kanban board integration (`/kanban`)
- [ ] Cron job management (`/cron`)
- [ ] Packaging: `.deb`, `.rpm`, Homebrew, nix flake, static musl binary
- [ ] E2E tests with event injection and golden transcript fixtures

## Performance Targets
- Startup < 100 ms on a modern laptop
- 60 FPS render budget with 10k-line transcripts
- Memory ceiling: 512 MB via virtualized transcript rows
- Sub-50 ms keystroke-to-screen latency
