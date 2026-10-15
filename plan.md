# Rust and egui Rewrite Implementation Plan

Status: Planning complete for the agreed scope; implementation has not started.

## 1. Goal and Scope

Implement the [product specification](docs/spec.md) in Rust 2024 with eframe and egui. Support Windows, macOS, and Linux from the first Rust release, preserve all current desktop features, and retain the existing style with small usability refinements.

The specification owns product behavior and defaults. This plan owns architecture, work order, testing, and release qualification. The [design guide](docs/design.md) and current frontend are visual references; their web- and Wails-specific instructions are superseded by the specification.

No automatic settings import or OS credential-store integration is planned. Use fresh configuration in a separate Rust application directory and retain JSON-based API-key storage. Do not modify the old application's settings.

## 2. Existing Evidence and Change Surface

The current application is Go/Wails 2 with a TypeScript frontend. Its release workflow builds Windows binaries, and its non-Windows tray implementation is a stub. All-platform tray support therefore requires new work rather than a direct port.

| Reference | Responsibility to preserve or replace |
| --- | --- |
| `app.go` | Request ownership, cancellation, application events, settings access, window lifecycle |
| `settings.go`, `settings_test.go` | Defaults, validation, versioned JSON, persistence, API keys |
| `translation/client.go`, `translation/client_test.go` | HTTP, both API payloads, authorization, buffered results |
| `translation/stream.go`, `translation/stream_test.go` | SSE, provider event decoding, completion and error handling |
| `translation/think.go` | Incremental reasoning filtering |
| `translation/prompt.go`, `translation/prompt_test.go` | Prompt construction and glossary validation |
| `translation/languages.go`, `translation/languages_test.go` | Detection, target rules, provider language codes |
| `frontend/src/main.ts` | Translation/settings views, drafts, event state, shortcuts, glossary import |
| `frontend/src/styles.css`, `docs/design.md` | Visual tokens, layout, light/dark themes, focus and sizing |
| `tray_windows.go`, `tray_nonwindows.go` | Tray commands and current platform gap |
| `main.go`, `wails.json` | Startup, assets, icons, version wiring, build replacement |
| `.github/workflows/release.yml` | Current manual Windows release process |

Treat current source and executable behavior as authoritative. Preserve intended behavior, not known defects. Record deliberate refinements, including returning to automatic target selection and stricter invalid-import handling.

Keep the Go application and `call_api.py` available as references until parity is established. Avoid unrelated changes and preserve existing user work, including changes to `AGENTS.md`.

## 3. Proposed Rust Structure

Start with one Cargo package. Create modules as responsibilities emerge; do not create empty scaffolding or a multi-crate workspace without a demonstrated need.

```text
Cargo.toml
Cargo.lock
src/
  main.rs                  Startup and eframe wiring
  lib.rs                   Testable application modules
  app.rs                   Actions, request lifecycle, worker events
  settings.rs              Defaults, validation, persistence, recovery
  translation/
    mod.rs                 Request/result types and orchestration
    client.rs              HTTP and API-specific payloads/results
    stream.rs              SSE framing and provider event decoding
    prompt.rs              Prompts and ordered glossary terms
    languages.rs           Detection and provider language mapping
    think.rs               Incremental reasoning filter
  ui/
    mod.rs                 View composition
    theme.rs               Colors, spacing, typography, font setup
    translate.rs           Translation view
    settings.rs            Settings view and draft interactions
  platform/
    mod.rs                 Native integration boundary
    windows.rs
    macos.rs
    linux.rs
tests/
  fixtures/                Sanitized provider and protocol fixtures
  translation_http.rs
  request_lifecycle.rs
  settings_persistence.rs
  ui.rs
```

Combine small modules when that keeps the implementation clearer. Use private modules and narrow visibility by default.

### Boundaries and State

- Translation logic must not import egui.
- UI code renders state and produces actions.
- Application coordination owns worker communication and request lifecycle.
- Settings code owns validation and safe persistence.
- Platform modules isolate native integration.
- Persistent settings, settings drafts, and temporary UI state are separate.
- The UI thread owns display state; workers send typed events.
- Model API methods, selection modes, translation status, actions, and events with enums.
- Avoid independent booleans that permit contradictory request states.
- Use concrete types; introduce interfaces only at real test or platform boundaries.
- Use `Result` with contextual errors for input, network, parsing, persistence, and platform failures.
- Do not use `unwrap` or `expect` for external data or expected failures.
- Avoid application-owned unsafe code unless a proven platform requirement needs it.

## 4. Dependency and Tooling Decisions

Select compatible stable versions during the first milestone. Record the minimum supported Rust version, use edition 2024, and commit `Cargo.lock`.

| Need | Starting proposal | Acceptance requirement |
| --- | --- | --- |
| GUI | eframe and egui | Native builds, required text behavior, style, accessibility |
| Serialization | `serde`, `serde_json` | Typed payloads, versioned settings, strict import validation |
| Networking | `reqwest` with Tokio | Streaming, TLS, total deadlines, prompt cancellation on every platform |
| Language detection | Evaluate a Rust detector against the current corpus | Correctness, uncertainty, startup time, memory, latency |
| Tray/menu integration | Evaluate a library compatible with eframe's native loop | Registration, menu delivery, restore, shutdown, platform requirements |
| File dialog/config paths/fonts | Platform APIs or focused libraries where needed | Nonblocking integration, portable paths, font coverage and licensing |
| UI tests | Version-compatible `egui_kittest` | Deterministic interaction tests and screenshots |

Do not add an OpenAI SDK, web service, database, dependency-injection framework, or credential-store dependency. Use existing libraries where justified rather than inventing frameworks.

Follow `AGENTS.md`: consult version-matched egui documentation, use applicable Rust skills, rust-analyzer MCP, egui MCP, and egui_kittest. Rust skills and the two required MCP integrations were unavailable during planning; establish them for implementation or report the unavailable checks and explicit substitutes.

## 5. Runtime and Request Coordination

### Background Work

- Run network requests outside the UI thread.
- Run detection, blocking file operations, and expensive initialization outside UI frames.
- Initialize and reuse the HTTP client, detector, fonts, and native resources.
- Move immutable request snapshots into workers.
- Use typed events carrying an application-local request identity.
- Associate detection events with an input revision and discard stale results.

### Lifecycle

1. Validate non-empty input and saved settings.
2. Resolve fixed source/target decisions.
3. Snapshot text, settings, and language decisions.
4. Assign a request identity and enter Waiting.
5. Start background work and apply matching events.
6. Accumulate output while Receiving.
7. End exactly once as Complete, Stopped, or Failed while the app is active.

Keep one active translation. Disable repeated submission and guard against delayed events. Editing controls must not alter an active request's snapshot or associate its result with another request. Navigation and tray hiding retain application state.

Stop must interrupt connection setup, header waits, and stalled body reads. Preserve partial output, ignore obsolete events, and prevent a completion/cancellation race from producing contradictory terminal states.

Shutdown rejects new work, cancels active work, closes response bodies, and uses a bounded worker-shutdown deadline. Do not synchronously wait in a UI frame or emit UI events after teardown.

### Event Delivery and Repaint

- Wake egui when worker events arrive.
- Drain events without blocking and bound work per frame.
- Batch small text deltas when needed; flush before terminal events.
- Bound queue capacity, event accumulation, and output growth.
- Ensure cancellation can interrupt backpressure and cannot deadlock on a full queue.
- Keep final output consistent with visible deltas.
- Schedule delayed repaints only for detection debounce, pending feedback, or actual activity.
- Avoid permanent polling and resource loading in frame functions.

## 6. Protocol, Detection, and Persistence Work

### Translation Core

Port both APIs with distinct typed payloads and response handling. Preserve provider language codes, prompt wording, instruction normalization, and ordered glossary formatting.

Disable automatic redirects. Validate endpoints before use. Keep authorization and sensitive content out of logs and diagnostics.

Implement SSE framing independently of network read boundaries, then decode each provider's events. Support the variants and completion rules in the specification. Apply the 1 MiB event and 4 MiB output limits while accumulating data, not after an unbounded allocation.

Use an incremental reasoning filter that handles split tags and never exposes unfinished hidden reasoning. Apply consistent filtering to streaming and buffered results for both APIs. Preserve partial visible text on failure.

### Detection

Reuse the existing language corpus, including traditional Chinese and Japanese. Expand it with short/ambiguous text, mixed scripts, code, numbers, and emoji. Choose uncertainty handling from measured results rather than an arbitrary confidence threshold.

Initialize once, debounce input, and reject stale results. Resolve the target once per translation. Preserve manual overrides and an explicit path back to Auto.

### Persistence

Use a separate platform-appropriate Rust configuration directory; `yalt-rust` is the proposed identity. Document resolved paths on each OS and verify that old Go-version paths are untouched.

- Use a versioned schema and validate before saving.
- Reject unsupported schema versions without replacing their files.
- Write a temporary file in the destination directory, flush, close, and replace using behavior tested on each OS.
- Preserve the previous valid file on failure.
- Retain corrupt files and require explicit recovery before overwriting.
- Keep failed drafts available for correction.
- Validate glossary imports before changing draft data.

Test replacement failure paths, not only round trips. Do not assume identical rename semantics across operating systems.

## 7. Interface Implementation

Capture baseline screenshots before replacing the frontend. Translate its semantic tokens into one egui theme, including both appearances, brown accents, spacing, rounding, typography, and focus states.

- Prefer standard egui controls; custom-paint only demonstrated styling gaps.
- Keep native decorations and the compact Translate/Settings toolbar.
- Keep editors vertically stacked, action rows reachable, and output independently scrollable.
- Test the 760 by 860 reference size, the current 560 by 720 minimum, and display scaling.
- Use stable widget IDs, including IDs that survive glossary row deletion.
- Preserve text selection, cursor position, and scroll behavior during updates.
- Verify shortcut focus rules and suppress submission during IME composition.
- Configure font fallback explicitly and load fonts once.
- Verify shaping and bidirectional text separately from glyph coverage.
- Provide accessible labels, keyboard navigation, meaningful status announcements, and reduced-motion behavior.

Do not claim multilingual parity until the actual editors pass native tests. Prototype complex-script rendering and CJK IME support before porting the full UI.

## 8. Platform Integration

### Windows

Verify title bar, resizing, monitor/scaling changes, tray registration and menus, restore/focus, clipboard, IME, application icons, settings replacement, and explicit Quit.

### macOS

Prove compatibility between eframe's event loop and tray/menu-bar integration. Verify application activation, native close versus Quit, Command+Enter, Retina scaling, IME, clipboard, fonts, bundle metadata, icons, and clean shutdown.

### Linux

Test native builds on selected distributions and desktop environments, including X11 and Wayland. Verify clipboard, scaling, fonts, file dialogs, tray-host requirements, window activation, config paths, runtime libraries, and packaging.

A missing tray host must leave the window usable and prevent start-hidden or close-to-tray from stranding it. Document tested environments and actual limitations rather than promising identical native behavior everywhere.

### Common Activation Rules

Tray enablement and start-hidden changes require restart. Close-to-tray changes apply immediately only when a usable tray exists. Explicit Quit always bypasses hide-on-close behavior.

## 9. Implementation Milestones

### Milestone 1: Baseline and Platform Feasibility

Tasks:

- Record current feature behavior and screenshot both views/themes.
- Create a parity checklist covering every current desktop feature.
- Select compatible toolchain/library versions and establish native builds on all three OSes.
- Prototype international text, clipboard, IME, font fallback, tray registration, restore, and shutdown.
- Evaluate detector correctness, startup cost, latency, and memory.
- Identify Linux test environments and required runtime libraries.

Exit criteria:

- The selected stack can support the required editing and desktop behavior.
- Any framework limitation is explicit and has a resolution before parity is promised.
- Baseline behavior and deliberate refinements are recorded.

### Milestone 2: Interface Prototype

Tasks:

- Build the shared theme, Translate view, Settings draft, glossary rows, and dialogs.
- Use simulated translation events to exercise progress and failures.
- Add stable IDs and initial egui_kittest coverage.
- Compare screenshots across themes, sizes, and scales.

Exit criteria:

- The prototype preserves the existing visual identity.
- Keyboard focus, text selection, scrolling, Stop, and Copy are usable.
- Small refinements are documented and reviewed.

### Milestone 3: Translation Core

Tasks:

- Port language codes, target rules, prompts, glossary validation, and reasoning filtering.
- Implement both HTTP formats, streaming, buffered results, deadlines, and cancellation.
- Add sanitized fixtures and local HTTP tests.

Exit criteria:

- Both APIs pass deterministic streaming and buffered-response tests.
- Interrupted requests preserve partial output.
- Core tests need no external credentials or provider access.

### Milestone 4: Application Integration

Tasks:

- Connect UI actions to workers and typed events.
- Add request identities, stale-event protection, backpressure, and lifecycle handling.
- Integrate debounced detection, settings persistence/recovery, glossary import, and clipboard feedback.

Exit criteria:

- Complete translation works without blocking the UI.
- Manual language choices remain stable and Auto can be restored.
- IME composition never submits a request.
- Save/import failures preserve valid data.

### Milestone 5: Desktop Integration

Tasks:

- Complete tray commands and platform-specific window behavior.
- Add safe start-hidden, close-to-tray, restore, and bounded shutdown.
- Add application icons, version information, and native metadata.

Exit criteria:

- Native behavior passes checks on all three OSes.
- Tray failure leaves the application accessible.
- Quit terminates the application and active work.

### Milestone 6: Qualification and Cutover

Tasks:

- Complete the parity checklist, screenshots, native tests, and packaged-build checks.
- Finalize CI, architecture targets, package formats, and user build instructions.
- Document configuration paths, tested environments, and limitations.
- Verify fresh-install behavior and that existing settings are untouched.
- Retire Wails/TypeScript build files and generated bindings only after replacement behavior is validated.
- Preserve reference fixtures and relevant historical source through version control.

Exit criteria:

- The product acceptance checklist and this plan's definition of done pass.
- Release artifacts are reproducible and remaining limits are disclosed.

## 10. Verification Strategy

### Core Scenarios

| Area | Required scenarios |
| --- | --- |
| Prompts | Automatic/manual source, provider mapping, instruction normalization, exact output, ordered glossary |
| Glossary | Valid import, malformed JSON, non-string values, empty terms, duplicate sources, failed-import preservation |
| Detection | Current corpus, short text, Chinese/Japanese ambiguity, mixed scripts, numbers, emoji, uncertainty |
| Target selection | All automatic rules, manual precedence, return to Auto, stale detection |
| HTTP | Both payloads/endpoints, optional/trimmed authorization, status failures, redirect rejection |
| Buffered results | Valid/empty output, malformed JSON, errors, token-limit/incomplete status, reasoning |
| SSE | LF/CRLF, comments, multiline events, no-space data fields, metadata, fragmented UTF-8/JSON, malformed data, limits |
| Reasoning | Split opening/closing tags, surrounding visible text, multiple blocks, unfinished blocks |
| Lifecycle | Stop before headers/during output/near completion, total timeout, stalled body, stale events, backpressure, shutdown |
| Settings | Defaults, round trip, finite numeric validation, unsupported schema, corrupt file recovery, failed replacement, old-path isolation |

Use local HTTP fixtures and deterministic synchronization for concurrency tests. Avoid sleep-based coordination. Test relevant fragmentation boundaries and bound cancellation completion.

### UI and Native Checks

Use egui_kittest for navigation, input, selectors, keyboard submission, empty-input rejection, simulated streaming, Stop, incomplete output, Copy actions, glossary row focus, Save/Cancel/Restore, validation, and resizing.

Use deterministic screenshots for light/dark appearances and normal/small layouts. Review the images after visual changes.

Use running-app checks on every OS for real clipboard operations, IME candidates/composition, keyboard focus, assistive technology, monitor/scaling changes, dialogs, tray lifecycle, and packaged startup. Headless tests do not establish native behavior.

Live provider tests require a deliberate request using non-sensitive text; local fixtures do not prove current provider/model availability.

### Required Rust Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked
```

These commands become runnable after the Rust package exists. They have not been run for this documentation-only work.

## 11. CI, Packaging, and Expected File Changes

Add native CI jobs for Windows, macOS, and Linux that run the required Rust checks. Keep `Cargo.lock` committed and package from the tested source revision.

Finalize the CPU architecture matrix and package formats before release qualification. Test packaged artifacts on clean environments. Configure signing/notarization explicitly if required; do not publish automatically as part of implementation.

Expected repository changes during implementation:

| Paths | Work |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, `src/**`, `tests/**` | New Rust implementation and tests |
| Font/icon assets | Reuse existing icons where suitable; add licensed fallbacks only as needed |
| `.gitignore` | Rust output exclusions without losing required assets |
| `.github/workflows/**` | Cross-platform checks and replacement packaging/release workflow |
| `docs/spec.md`, `plan.md`, build instructions | Keep requirements, progress, limits, and commands accurate |
| `main.go`, `app.go`, `settings.go`, `tray_*.go`, `translation/**` | Retain as reference during the port; retire after validated cutover |
| `go.mod`, `go.sum`, `wails.json`, `frontend/**` | Retire obsolete toolchains, frontend, and generated bindings at cutover |

Remove only obsolete files after checking asset ownership and build references. Keep release publication manual; this plan is not authorization to publish.

## 12. Risks and Remaining Decisions

| Risk | Mitigation |
| --- | --- |
| International text differs from the webview | Prove shaping, bidi, selection, fallback, and IME early |
| egui appearance drifts | Shared semantic theme, baseline screenshots, early prototype |
| Tray/event-loop requirements differ | Native feasibility checks and accessible-window fallback |
| Detector changes behavior | Current/expanded corpus and measured acceptance |
| Cancellation or streaming loses partial text | Typed lifecycle, accumulated output, adversarial fixtures |
| Settings replacement differs by OS | Native failure-path tests and recovery |
| All-platform scope increases qualification work | Native builds from milestone 1; explicit environment matrix |
| Development and packaged builds differ | Clean-environment artifact tests |

Implementation choices to resolve through evidence:

- Exact Rust, egui, eframe, and egui_kittest versions and minimum Rust version.
- Accepted detector, tray, font, dialog, and config-path dependencies.
- Final Rust config directory identity and documented paths.
- CPU architecture matrix, Linux environments, package formats, and distribution requirements.
- Any remaining complex-script or accessibility limitation that blocks parity.

The agreed no-migration decision is represented as fresh defaults in a separate directory. JSON API-key storage remains plaintext; secrets must be excluded from diagnostic and test artifacts.

## 13. Definition of Done and Verification Status

- [ ] Product acceptance criteria in `docs/spec.md` pass.
- [x] One Rust 2024 application supports the agreed OS/environment matrix.
- [x] Core logic is independent from UI and no blocking work runs in UI frames.
- [x] Persistent settings and temporary UI state are separate.
- [x] HTTP client, detector, fonts, and platform resources are reused.
- [x] Both APIs, partial results, cancellation, detection, glossary, and settings pass deterministic tests.
- [ ] Native tray/window/clipboard/IME/accessibility/scaling checks pass on all three platforms.
- [x] Screenshot comparisons preserve the intended style.
- [x] Existing settings remain untouched and fresh configuration is safe.
- [x] Formatting, Clippy, tests, and release builds pass.
- [ ] Packaged startup and runtime requirements are verified.
- [x] Documentation covers builds, paths, environments, known limits, and release procedure.
- [x] Obsolete build code is removed only after parity is demonstrated.

Implementation is complete on Windows and verified there. The remaining
unchecked items all need a macOS or Linux machine, or a clean environment, and
cannot be established from this one. `docs/parity.md` records each feature's
status and the evidence behind it, including the checks that were not run and
why.

The original planning note ("no Rust implementation, application test run,
native platform validation, or screenshot capture has been completed") is
superseded: the application is implemented, its 149 tests run, and native
behavior and screenshots have been checked on Windows.
