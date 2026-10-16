# Agent Instructions

## Development

- Use Rust 2024 edition.
- Use eframe and egui for the GUI.
- Follow the project's Rust skills.
- Read the API documentation for the installed egui version.
- Use rust-analyzer MCP for semantic code analysis.
- Keep the code simple and idiomatic.
- Do not add dependencies without a clear need.

## Architecture

- Keep business logic separate from GUI code.
- Separate persistent data from temporary UI state.
- Keep each module focused on one responsibility.
- Do not perform blocking work in the UI frame.
- Do not reload resources or repeat expensive work each frame.
- Use background tasks for long operations.
- Request repaint only when needed.

## GUI

- Follow egui's immediate-mode design.
- Use stable widget IDs when needed.
- Support window resizing and display scaling.
- Check keyboard navigation and text input.
- Keep visual styles consistent.
- For UI implementation and review, follow `.claude/skills/smartisan-egui/SKILL.md` and the current desktop overrides in `docs/design.md`.

## Tools and Skills

- Use rg for searching file contents; do not use grep.
- Use Rust skills for implementation and review.
- Use rust-analyzer MCP for symbols and diagnostics.
- Use egui MCP to inspect and test the running app.
- Use egui_kittest for repeatable UI tests.
- Consult version-matched documentation before using unfamiliar APIs.

## Verification

- Run cargo fmt --all -- --check.
- Run cargo clippy --all-targets -- -D warnings.
- Run cargo test.
- Test changed UI behavior with egui MCP.
- Check screenshots after visual changes.
- Report failed checks and untested behavior.
