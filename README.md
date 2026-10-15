# yalt

A desktop translator for OpenAI-compatible Chat Completions and Responses
APIs. Rust 2024 with eframe/egui; runs on Windows, macOS, and Linux.

This is a rewrite of an earlier Go/Wails application. It is a separate
application: it has its own configuration directory and does not read, migrate,
or modify the previous version's settings.

## Building

Requires a Rust toolchain at or above the version in `rust-version` in
`Cargo.toml`.

```sh
cargo build --release --locked
```

The binary is `target/release/yalt` (`yalt.exe` on Windows). It is
self-contained: the icon and, on Windows, the version metadata are embedded at
build time.

### Linux build dependencies

eframe links against the system windowing and font libraries:

```sh
sudo apt install libgl1-mesa-dev libxkbcommon-dev libwayland-dev \
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev libfontconfig1-dev
```

These are build-time only. The tray needs no libraries, because it has no Linux
backend (see [Platform behavior](#platform-behavior)).

## Configuration

Settings are stored as JSON. Each platform's path follows that platform's
convention:

| Platform | Path |
| --- | --- |
| Windows | `%APPDATA%\yalt-rust\settings.json` |
| macOS | `$HOME/Library/Application Support/yalt-rust/settings.json` |
| Linux | `$XDG_CONFIG_HOME/yalt-rust/settings.json` (or `$HOME/.config/...`) |

The `yalt-rust` directory name is deliberate, so the previous Go application's
`yalt` directory is never read or written.

Writes are atomic: settings are written to a temporary file and then replace
the previous file, so an interrupted save cannot leave a half-written file. A
file that is missing, corrupt, or carries an unsupported schema version is
reported and left untouched rather than overwritten. Saving is blocked until
that is resolved, so a recoverable mistake cannot destroy the file.

The API key is stored in plaintext in this file. Keep it out of shared
screenshots, logs, and issue reports.

## Platform behavior

Tray icon, close-to-tray, and start-hidden behave differently by platform,
because a tray icon depends on facilities that are not universally present.

| Platform | Tray | Notes |
| --- | --- | --- |
| Windows | Yes | Verified against a running build. |
| macOS | Yes | Implemented; not verified on a macOS machine. |
| Linux | No | No backend is built. |

When no tray is available, the application stays fully usable from its window:
close-to-tray is ignored, and a start-hidden launch reveals the window instead
of hiding it. A tray that fails to register can therefore never strand the
window out of reach.

On Linux, both candidate tray backends need facilities that cannot be assumed —
GTK with `libappindicator`, or a session D-Bus service — so neither is used.
Tray-host requirements and X11/Wayland coverage are therefore not claimed.

Start-hidden and tray enablement take effect after a restart; the interface
says so after saving. Close-to-tray applies immediately.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked
```

CI runs these on Windows, macOS, and Linux; see
`.github/workflows/check.yml`.

The test suite needs no network access or provider credentials: HTTP behavior
is exercised against local fixtures.

### What the tests do not cover

These need a human on a real desktop, and are not established by the automated
suite:

- Tray menu interaction — clicking **Open Translator**, **Settings**, or
  **Quit**. The behavior behind them is covered, but the click path itself is
  not automated.
- IME composition, clipboard integration, and assistive-technology behavior.
- Display scaling and monitor changes.
- macOS and Linux at all. Neither platform has been run.
- Live provider requests. The fixtures prove protocol handling, not that a
  given model is currently available.

## Releasing

Releases are published by `.github/workflows/release.yml`, triggered manually
with a version. It builds Windows (x86_64), macOS (Apple Silicon and Intel),
and Linux (x86_64) archives and attaches them to a GitHub release.

The requested version must match `version` in `Cargo.toml`, so the release tag
and the version metadata compiled into the executable cannot disagree.

## Repository layout

| Path | Contents |
| --- | --- |
| `src/translation/` | Provider protocol, prompts, detection, streaming |
| `src/settings.rs` | Settings model, validation, persistence |
| `src/app.rs` | Request lifecycle and background work |
| `src/ui/` | eframe application, views, theme, fonts |
| `src/tray.rs` | Tray icon and its commands |
| `tests/` | Integration tests, including the local HTTP fixtures |
| `docs/spec.md` | Product specification |
| `plan.md` | Implementation plan and milestones |
| `call_api.py` | Provider reference script, kept for protocol checks |
| `*.go`, `frontend/`, `wails.json` | Previous implementation, kept for reference |

The Go sources and the TypeScript frontend are retained for reference during
the port. They are not built, and no workflow uses them.
