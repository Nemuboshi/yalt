# yalt Product Specification

Status: Approved scope for a planned Rust rewrite; implementation has not started.

Implementation sequence and verification details are in [plan.md](../plan.md).
The existing [design guide](design.md) remains the visual reference. This specification supersedes its HTML-, CSS-, and Wails-specific implementation directions.

## 1. Objective and Decisions

Rewrite yalt using Rust 2024, eframe, and egui while preserving the current desktop application's features and visual identity.

| Area | Decision |
| --- | --- |
| Platforms | Windows, macOS, and Linux from the first Rust release |
| Interface | Preserve the current style with small usability refinements |
| Feature scope | Preserve all current desktop features |
| Settings migration | No automatic import or migration |
| API-key storage | Retain JSON-based storage; do not introduce an OS credential store |
| Existing application | Retain as a reference until the Rust replacement is validated |

The working interpretation of no migration is fresh defaults in a separate configuration directory. The Rust version must not automatically read, modify, or overwrite the Go application's settings.

## 2. Scope

### Included

- Text translation with manual submission.
- Local source-language detection and manual source selection.
- Automatic and manual target selection.
- Chat Completions and Responses APIs.
- Streaming and non-streaming responses.
- Custom API base URLs, custom model identifiers, and optional bearer API keys.
- Deadlines, cancellation, partial results, and copy feedback.
- Extra instructions, glossary editing, and JSON glossary import.
- Model-reasoning preference and reasoning-text filtering.
- Settings validation, drafts, persistence, and restore-defaults confirmation.
- Light and dark appearance, keyboard navigation, and international text input.
- Native windows, resizing, tray commands, close-to-tray, and start-hidden on all three platforms.

### Excluded

Image translation, a local proxy or HTTP server, translation history, clipboard monitoring, global translation shortcuts, automatic startup at sign-in, accounts, cloud synchronization, automatic updates, a plugin system, and a database are outside this rewrite.

Features of `call_api.py` that are absent from the desktop application are not implied by desktop feature parity.

## 3. Window and Navigation

- Keep native window decorations and operating-system window controls.
- Keep a fixed, non-resizable bottom status bar in both views. Status text starts at its far left; a Settings toggle stays at its far right.
- The Settings toggle remains selected while Settings is open. Clicking it again returns to Translate and discards the unsaved draft, like Cancel.
- Focused settings editors retain a visible outline, including the bounded multiline viewport.
- Use the project-local Smartisan-inspired egui style: light gray page, white grouped surfaces, fine borders, subtle gradients and inset fields, muted blue primary actions and selection. Dark mode is a documented charcoal/blue adaptation. Preserve the compact desktop geometry and native interactions; `docs/design.md` defines the current overrides and attribution.
- Prefer a locally installed LXGW Neo XiHei (Plus when available) for all interface text. Resolve missing characters through installed system fonts, then egui defaults. Never embed or download XiHei during builds or at runtime.
- Keep the application name in the native title bar.
- Keep the input above the output at every supported window width.
- Use 760 by 860 logical pixels as the initial window-size reference.
- Evaluate the current 560 by 720 minimum during the prototype and establish a usable supported minimum before release.
- At narrow widths, wrap or stack language controls as needed.
- At short heights, allow page scrolling and keep actions reachable.
- Component sizes are determined by the layout and window size, never by user text, provider messages, or item counts.
- Text editors and collection controls have bounded viewports. Overflow scrolls inside the component; multiline text and additional glossary entries must not enlarge it.
- Single-line fields retain one row and scroll horizontally with the cursor. Multiline editors and lists show scrollbars when their content exceeds the viewport.
- Dynamic informational text stays within its allocated area, using internal scrolling or truncation with the full text available on hover. It must not push adjacent controls outside the window.
- Long output scrolls within its own surface and does not expand the layout indefinitely.
- Navigation to Settings and hiding the window to the tray do not cancel translation.
- Returning to Translate shows retained input, output, and current request status.

## 4. Translation View

### Controls

The view provides:

- Source selection, including Detect language.
- Target selection, including Auto.
- A multiline input editor and character count.
- Clear and Translate actions.
- A selectable, read-only, plain-text result.
- Stop while a request is active.
- Copy for any non-empty complete or partial result.
- Progress, completion, error, and incomplete-result feedback.

Show readable language names while preserving provider language identifiers, including identifiers such as `ukr_cyrl`, `fas_arab`, and `ces_latn`. The existing table in `translation/languages.go` is the language-list baseline; a listed language does not prove that every provider supports it.

When a source is detected, mark it as auto-detected. A manual source selection removes that mark. Both selectors must allow returning to automatic mode after a manual choice.

### Submission

- Start translation only through Translate or the platform shortcut.
- Use Command+Enter on macOS and Ctrl+Enter on Windows and Linux.
- The shortcut applies while the translation input has focus.
- Enter inserts a newline.
- Never submit while an input method is composing text.
- Disable submission for empty or whitespace-only input and while a translation is active.
- Do not submit text automatically while typing or detecting language.
- If source and target are the same, show a notice and permit an explicit request.

Each request uses a fixed snapshot of input, source, target, and saved settings. Later edits must not change an active request or relabel its result as belonging to another request.

### Results and Status

Distinguish Ready, Waiting, Receiving, Complete, Stopping, Stopped, and Failed states where relevant. A stopped or failed request with partial output must show an explicit incomplete label.

- Preserve accumulated output after Stop, timeout, provider error, connection failure, invalid later stream data, or output-token exhaustion.
- Keep partial results copyable.
- Show clipboard success or failure near Copy.
- Keep output stable while text arrives; do not animate the layout per token.
- Default to Plain output, with a Plain / Markdown switch at the far left of the result action row and Copy result alone at the far right. The choice is temporary UI state and does not alter requests or output data.
- Buttons retain their dimensions across idle, hovered, pressed, and selected states.
- Markdown renders the accumulated result as streaming chunks arrive, inside the same bounded viewport. Incomplete Markdown may reformat as later chunks complete it.
- Markdown strong emphasis has visibly heavier glyphs. With egui's color-only strong styling, synthesize weight using the preferred font without changing text size or layout.
- Copy result always copies the original output text, including Markdown source when present. Markdown mode does not fetch remote images.
- Do not announce every streamed token to screen readers.
- Do not automatically retry a request after partial output.
- An explicit new request starts a fresh result.

## 5. Language Detection and Target Selection

Detect language locally after a short typing pause. Detection must not make a separate provider request or block typing.

Associate detection with the current input revision. Discard stale results and never replace a manual selection.

Automatic target rules:

1. A manual target always wins.
2. With an automatic target and manual source, Chinese selects English; all other sources select Chinese.
3. With both controls automatic, detected Chinese selects English.
4. Detected non-Chinese selects Chinese.
5. Empty input has no detected language and cannot start a request.
6. Uncertain input shows "Language uncertain", proposes Chinese, and permits manual correction.

For uncertain input, retain automatic-source wording in the prompt rather than asserting an unverified language.

Detection must distinguish Chinese from non-Chinese rather than treating every Han character as Chinese. Evaluate simplified Chinese, traditional Chinese, Japanese, mixed text, short text, code, numbers, and emoji. Very short Chinese and Japanese samples can be ambiguous; confidence values are not calibrated probabilities.

Resolve the target once per request. Do not change it while receiving output.

## 6. Settings

### Defaults

| Field | Default |
| --- | --- |
| API base URL | `https://index-translate.bilibili.com/v1` |
| API method | Chat Completions |
| API key | Empty |
| Model | `Index-Translate-35B-A3B` |
| Request deadline | 120 seconds |
| Source selection | Auto |
| Target selection | Auto |
| Stream output | On |
| Maximum output tokens | 1024 |
| Temperature | 0.0 |
| Extra instructions | Empty |
| Glossary | Empty |
| Disable model reasoning | On |
| Tray enabled | On |
| Close to tray | On |
| Start hidden | Off |

Retain an application/version client identifier and any persistent fields needed for current desktop behavior. Do not add new controls solely because an internal field exists.

### Editing

- Opening Settings creates a draft from saved settings.
- Cancel discards draft changes.
- Restore Defaults requires confirmation and changes only the draft.
- Save validates and persists the draft before applying it.
- A failed save preserves the draft and the previous saved configuration.
- Successful changes apply to future translation requests.
- Show the final request endpoint near the connection fields.
- Permit custom model identifiers; do not claim current provider availability from a built-in name.

### Validation

- Accept HTTPS endpoints.
- Accept HTTP only for explicit loopback endpoints.
- Reject malformed URLs, embedded credentials, query strings, and fragments.
- Require a supported API method and a non-empty model.
- Require a deadline between 1 and 600 seconds.
- Require positive maximum output tokens, with at least 16 for Responses to retain current behavior.
- Require finite, nonnegative temperature.
- Require valid glossary entries and a supported configuration schema version.
- Show actionable errors without exposing API keys.
- Do not invent provider-specific limits beyond established behavior; report provider validation errors clearly.

The request deadline covers the total request, including streaming.

### Glossary

- Represent terms as ordered source/target rows.
- Support adding, editing, removing, and importing terms.
- Import a JSON object containing string source/target pairs.
- Reject invalid JSON, invalid value types, empty terms, and duplicate source terms.
- Compare duplicate sources using the existing case-insensitive validation intent.
- Preserve row order when constructing a prompt.
- Validate imports before replacing the current draft glossary.
- Preserve the existing glossary if import fails. Show JSON parsing and validation failures in a centered Info dialog with an OK action, rather than in the bottom feedback label.

## 7. Provider Contract

| API | Endpoint suffix | Principal input and token fields |
| --- | --- | --- |
| Chat Completions | `/chat/completions` | `messages`, `max_tokens` |
| Responses | `/responses` | `input`, `max_output_tokens` |

Both support configured model, temperature, streaming, and optional bearer authorization. Disable automatic redirects so requests and credentials are not silently sent to another endpoint.

Preserve current prompt behavior for language names, provider codes, automatic-source wording, whitespace handling, instructions, and ordered glossary constraints.

Use provider-specific reasoning request fields only where supported. Filter inline reasoning consistently from streamed and buffered output for both APIs, retaining the current display-filtering behavior independently of whether the provider honors its reasoning preference.

### Streaming

Support LF and CRLF, comments, `data:` with or without a following space, multiline data events, metadata, and arbitrary network chunk boundaries. Network reads may split UTF-8, JSON, and reasoning tags.

- Recognize each API's completion and failure semantics.
- Require the Chat Completions completion marker for the established contract.
- Require a Responses completion event for successful Responses streaming.
- Do not report success merely because the connection ended.
- Report malformed events, provider errors, token-limit termination, and premature disconnection.
- Handle empty choices and ignorable metadata without treating them as text.
- Preserve bounded parsing and output: use the current 1 MiB SSE-event and 4 MiB output limits as the starting contract.
- Enforce limits during accumulation, before excessive allocation.
- Do not expose hidden reasoning from fragmented or unfinished reasoning blocks.

## 8. Persistence and Privacy

Use versioned JSON settings in a platform-appropriate configuration directory under a separate Rust application identity. `yalt-rust` is the proposed directory name; final paths must be documented for each platform.

- Start with fresh defaults when no Rust configuration exists.
- Do not automatically import the old application's settings.
- Retain API-key persistence in JSON; OS credential-store integration is excluded.
- Keep credentials out of logs, screenshots, test fixtures, and error messages.
- Do not persist translation input, output, instructions, glossary values, or raw provider responses outside the settings fields explicitly intended to be saved.
- Use restrictive file permissions where supported.
- Preserve the previous valid settings when saving fails.
- Retain corrupt or unsupported files and show recovery feedback.
- Do not overwrite an unreadable configuration through ordinary Save until the user explicitly chooses recovery.

API keys remain plaintext in JSON under this storage decision.

## 9. Desktop Behavior

Provide Open Translator, Settings, and Quit tray commands on all three operating systems. Quit must bypass close-to-tray and cancel active work.

- Hide on close only when close-to-tray is enabled and tray registration has succeeded.
- If a tray cannot start, keep the window accessible and use normal close behavior.
- Start hidden only after successful tray registration.
- Restoring from the tray shows and activates the appropriate view.
- Tray enablement and start-hidden changes take effect after restart, with a notice after Save.
- Close-to-tray changes take effect immediately, subject to actual tray availability.
- Application shutdown cancels active work and finishes within a bounded period.

Linux support must document tested desktop environments, tray-host requirements, and X11/Wayland coverage. An environment without a tray host must still support normal window-based use.

## 10. Appearance, International Text, and Accessibility

Use a compact Smartisan-inspired desktop utility style across platforms:

- Muted blue accent, quiet gray and white surfaces, subtle depth, fine separators, restrained typography.
- Light and dark appearance.
- Consistent spacing based on a 4-point scale.
- Approximately 6-point control rounding and 10-point editor/group rounding.
- System-appropriate fonts with explicit international-text fallbacks.
- Visible keyboard focus, disabled states, and textual status feedback.
- No fake title bar or replacement window buttons.

Small refinements may improve spacing, alignment, automatic-language controls, validation, focus, and resizing. Avoid unnecessary animation and respect reduced-motion preferences.

Text support must cover the scripts represented by the current language list. Verify font coverage, shaping, bidirectional text, selection, cursor movement, and IME composition separately. Font downloads at runtime are excluded; bundled fallback fonts must have suitable licenses.

Every control must have an accessible label and keyboard access. Keep output available to assistive technology and announce meaningful status changes without announcing each token. Validate accessibility on each operating system.

## 11. Acceptance Criteria

- [ ] Windows, macOS, and Linux builds preserve all current desktop features.
- [x] Both APIs work with streaming and buffered responses.
- [x] Manual selection, local detection, uncertainty, and automatic target rules behave as specified.
- [x] Stop and failures preserve copyable partial results with an incomplete label.
- [x] Navigation and tray hiding retain active translation state.
- [x] Settings drafts, validation, glossary import, and recovery preserve user data.
- [x] Existing Go-version settings remain untouched; fresh Rust settings use a separate directory.
- [x] The interface preserves the current visual identity with documented small refinements.
- [ ] Both themes, supported window sizes, and display scaling are usable.
- [ ] International text, IME, keyboard navigation, clipboard, and accessibility are tested.
- [x] Tray failure cannot strand the application in a hidden state.
- [ ] Packaged applications pass native checks on all three platforms.

Every unchecked item needs a macOS or Linux machine, or a clean environment.
`docs/parity.md` records each one's status and the evidence behind it, so a
partial result is not mistaken for a complete one.

Implementation details, test scenarios, tooling requirements, and remaining release decisions are maintained in [plan.md](../plan.md).
