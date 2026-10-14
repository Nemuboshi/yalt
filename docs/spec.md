# Translation App Design

Date: 2026-10-14

Status: Proposed design. No application code has been changed.

## Script Review

The directory contains `call_api.py` and `AGENTS.md`. There is no Go application, test suite, or CodeGraph index.
The Python script is the source for the settings and request format in this design.
Its comments refer to `translate.py`, but that file is not present. Prompt equality with that file is not established.

### High: Stream errors can appear to be successful translations

At `call_api.py:265`, an event without `choices` is ignored.
A server error in such an event produces no error message.
At `call_api.py:250`, the stream can end without a completion marker.
The function then prints a newline and returns as if the response was successful.
The app must distinguish a completed translation from a broken connection.

Local tests reproduced both cases with a simulated response.
An error event produced empty output without an error.
An unfinished stream produced partial output without an error.

### Medium: Valid stream events can be lost

At `call_api.py:252`, the parser accepts only lines that start with `data: `.
Server-Sent Events (SSE), a format for streamed text events, also permits `data:` without a space.
The parser also processes each line separately. It does not assemble events with multiple data lines.
Malformed JSON is silently ignored at `call_api.py:259`.
These conditions can cause missing output without an error.

A local test with `data:` and no space reproduced the lost output.
The multiple-line case was examined in the code but was not separately exercised.

### Medium: Streaming bypasses reasoning-text removal

At `call_api.py:271`, content is written directly to the output.
Only the non-streaming path calls `strip_think`, at `call_api.py:277`.
Thus, the two paths do not produce the same result when the server returns reasoning tags.
A local test reproduced visible `<think>` text in streaming output.
The request disables reasoning, but the client cannot assume that each server obeys this field.

### Medium: The optional proxy lacks access and input limits

At `call_api.py:302`, the proxy permits requests from all origins.
At `call_api.py:337`, it accepts an unchecked body length.
There is no access token or request-size limit.
The default loopback address reduces exposure, but does not authenticate callers.
A LAN address exposes the service to other reachable machines.
The desktop app must not start this proxy by default.

This finding is from code inspection. No proxy or network attack test was done.
Browser access to loopback services also depends on browser network restrictions.

### Requirement gap: Automatic target selection is absent

At `call_api.py:441`, the target defaults to English.
The `auto` source value affects the prompt, not local language detection.
The Go app needs new language detection and target-selection logic.
This is a new requirement, not a defect in the existing command-line contract.

## Scope And Decisions

Build a Wails 2 desktop app with Go and TypeScript.
Use a small TypeScript frontend with Vite and ordinary HTML and CSS.
Do not add a frontend framework for two pages.
HTMX is not selected because this design uses Wails method calls and events, not HTML responses from a web server.

The first release provides text translation, streamed output, a settings page, local language detection, and a Windows tray icon.
Linux and macOS remain design targets. Their tray support requires separate native tests.
Do not substitute Wails 3 APIs for Wails 2 APIs.

Keep the Python script unchanged as a reference.
Image translation and the local proxy are existing script features, but the requested desktop scope does not explicitly include them.
Keep their settings in this design. Do not silently claim feature parity.
Obtain a scope decision before implementation of either feature.

Do not add translation history, clipboard monitoring, account access, automatic startup at sign-in, or a local HTTP server.
These are separate features and are not required for the requested interface.

## Translation Page

Use two large text areas in a single vertical column.
The input is above the output at every window width.
Start with a proposed window size of 760 by 860 logical pixels.
Permit window resizing. At small heights, let the page scroll instead of hiding controls.

```text
yalt                                         Settings

Source: Detect language       Target: Auto
Detected: Chinese             Auto target: English

Input
+--------------------------------------------------+
| Type or paste text                               |
|                                                  |
+--------------------------------------------------+
Clear                         Translate   Ctrl+Enter

Output                                  English
+--------------------------------------------------+
| Translation appears here as it arrives            |
|                                                  |
+--------------------------------------------------+
Status: Receiving translation        Stop      Copy
```

Use warm off-white surfaces, dark ink text, and one brown accent.
Use a bundled, licensed font with CJK system-font fallbacks.
Do not download fonts at runtime.
Keep animation limited to a short page entrance and an activity indicator.
Obey reduced-motion settings.

The source selector provides automatic detection and the language choices from the script.
When a language is auto-detected, mark the selected option with the text
"(Auto-detected)".
Remove the mark when the user selects another option.
Keep the option value as the language code.
The target selector provides automatic selection and manual language choices.
A manual target remains in effect until the user selects automatic mode.
Do not overwrite that choice when the input changes.
If the source and target are the same, show a notice and permit an explicit translation request.

Translation starts with the button or Ctrl+Enter.
Do not send text on each keystroke.
Disable translation for empty or whitespace-only input.
Enter inserts a newline. Do not submit while an input method is composing text.

Keep the output as plain text. Do not interpret server text as HTML.
Provide labelled controls, visible keyboard focus, and accessible status messages.
Do not announce each output token to screen readers.
Show progress and final status separately.
Keep partial output after Stop or an error, with a clear incomplete label.
Copy remains available for partial output, but the incomplete label remains visible.

## Settings Page

Save persistent defaults here. Keep per-request source and target controls on the translation page.
Use Save, Cancel, and Restore Defaults.
Restoring defaults changes the form first; Save commits the change.
Apply saved settings to the next request, not to a request already in progress.
Desktop settings have the separate activation rules in the Desktop section.

### Connection

| Script setting | App field | Initial value |
| --- | --- | --- |
| `--api-base` | API base URL | `https://index-translate.bilibili.com/v1` |
| `--model` | Model | `Index-Translate-35B-A3B` |
| `timeout=120` | Request deadline in seconds | `120` |
| `User-Agent` | Fixed client identifier | App name and version |

The model field permits a custom identifier.
The script lists `Index-Translate-2B` and `Index-Translate-9B` as additional models.
Treat these as script-provided choices, not as evidence of current server availability.
The app appends `/chat/completions` to the base URL.
Show the resulting endpoint before Save.

Permit HTTPS endpoints. Permit HTTP only for explicit loopback endpoints in the first release.
Reject embedded credentials, query strings, fragments, and invalid URLs.
Do not send requests to another host through an automatic redirect.
The script has no API-key setting. Do not add credential storage unless another provider is required.
If credential support is added later, keep credentials in an OS credential store and out of frontend storage.

The script timeout is an I/O timeout. The proposed app field is a total request deadline.
This is an intentional behavior change that prevents a request from remaining active indefinitely.

### Translation

| Script setting | App field | Initial value |
| --- | --- | --- |
| `--source` | Default source | Auto |
| `--target` | Default target mode | Auto, changed from script default English |
| `--stream` | Stream output | On, changed from script default Off |
| `--max-tokens` | Maximum output tokens | `1024` |
| `--temperature` | Temperature | `0.0` |
| `--instruction` | Extra instructions | Empty |
| `--glossary` | Term pairs | Empty |
| `enable_thinking=False` | Disable model reasoning | On |

A token is a unit of model input or output, not a character.
Temperature controls variation in the model output.
Keep token and temperature fields under Advanced.
Require a positive token limit and a finite, nonnegative temperature.
Do not invent provider-specific upper limits. Show server validation errors clearly.
Use a proposed request-deadline range of 1 to 600 seconds.

Represent the glossary as source and target rows.
Reject empty source or target terms and duplicate source terms.
Preserve row order when the prompt is built.
Optional JSON import must report invalid files instead of silently changing their meaning.
The legacy delimiter format can remain an explicit import option if script compatibility is required.

Keep the script language identifiers in the request mapping, including identifiers such as `ukr_cyrl`.
Show readable language names in the interface.
The script language table is not proof of provider support.
The prompt builder also has a `genre` parameter, but the script never exposes it as a command-line setting.
Keep the existing text genre fixed instead of adding an unused control.

### Desktop

Proposed defaults: tray enabled, close to tray enabled, start hidden disabled.
Automatic startup at sign-in is not included.
Close to tray is effective only after successful tray registration.
If the tray cannot start, keep the window accessible and use normal window-close behavior.
Changes to tray enablement and start-hidden behavior take effect after restart.
Show a restart notice after Save when these values change.
Close-to-tray changes take effect immediately, subject to the current tray-ready state.
Disabling the tray therefore does not strand an already hidden window.

### Existing Features Awaiting A Scope Decision

`--image` is request content, not a persistent setting.
If included, use an attachment control on the translation page.
An image-only request requires a manual target until image language detection is explicitly designed.
Do not claim that text language detection can classify an unread image.
Image format, size limits, and provider support need separate tests.

`--serve` and `--proxy` map to an optional Enable Local Proxy setting, initially Off.
`--host` maps to a bind address, initially `127.0.0.1`.
The proxy port defaults to `8080`.
If included, restrict the first release to loopback, add caller authentication, and limit request sizes.
Permit only the required origins and endpoints.
Test `/completions` and `/responses` independently before advertising support.
The script forwards these paths but does not establish that the upstream supports them.

## Language Detection

Detect language locally. Do not send a separate detection request.
Return a language identifier, an uncertainty indicator, and the proposed target.
The classification must distinguish Chinese from non-Chinese, not merely Han characters from other characters.
Japanese also uses Han characters.

Use `lingua-go` as the proposed detector dependency.
It documents Chinese and Japanese support and confidence values.
Its memory use and accuracy require measurements before the dependency is accepted.
Include non-Chinese candidates; a detector restricted to Chinese and English would misclassify other input.
Do not treat confidence values as calibrated probabilities.

Proposed selection rules:

1. A manual target always wins.
2. With an automatic target and a manual source, Chinese selects English; all other sources select Chinese.
3. With both controls automatic, detected Chinese selects English.
4. Detected non-Chinese selects Chinese.
5. Empty input has no detected language and cannot start a request.
6. Uncertain input shows "Language uncertain" and proposes Chinese, with a visible manual override.

The uncertainty fallback is a proposal, not a guaranteed correct classification.
Very short Chinese and Japanese text can be indistinguishable.
Include simplified Chinese, traditional Chinese, Japanese, mixed text, code, numbers, and emoji in the detection corpus.
For mixed text, classify the text as a whole. Do not switch targets during output.
Choose detector thresholds from the corpus results, not an arbitrary confidence number.

After a short typing pause, update the detection label locally.
Associate each result with an input revision. Discard results for old input.
At Translate, take a fixed copy of the input, settings, and target decision.
For uncertain input, retain automatic source handling in the prompt instead of asserting an unverified source language.

## Go Structure

Use a thin Wails adapter and one independent translation package.
The translation package has no Wails imports.
Keep settings and platform integration at the application root until a separate package has a clear reason.

```text
main.go                 Wails 2 setup and embedded frontend assets
app.go                  Bound methods, request ownership, runtime events
settings.go             Settings validation and persistence
tray_windows.go         Windows tray integration
tray_darwin.go          macOS tray integration
tray_linux.go           Linux tray integration
translation/
  client.go             HTTP calls, deadlines, response handling
  prompt.go             Script-compatible prompts and glossary formatting
  stream.go             SSE decoding and output filtering
  language.go           Local detection and target selection
  *_test.go             Unit and HTTP fixture tests
frontend/
  src/main.ts           App-lifetime state and event subscriptions
  src/translate.ts      Translation page and request state
  src/settings.ts       Settings page
  src/styles.css        Shared styles and layout
  wailsjs/              Generated Wails bindings; do not edit
```

Use the Go standard library for HTTP, JSON, contexts, and synchronization.
Do not add an OpenAI SDK, database, dependency-injection framework, or local web service.
The script request is small enough for a typed Go client.
Use consumer-defined interfaces only at actual test or platform boundaries.

Store non-secret settings under `os.UserConfigDir()` in an application-specific directory.
Include a schema version.
Validate settings in Go, even when the frontend has already validated them.
Write a temporary file in the same directory, then use an OS-tested replacement operation.
Preserve the previous valid file if saving fails.
Do not assume identical file-replacement behavior on every OS.
If the file is corrupt, show a recovery notice and retain it for diagnosis.
Do not silently overwrite it with defaults.
Do not persist translation text or raw provider responses.

## Streaming And Request Ownership

Go sends `POST /chat/completions` with the script request fields.
Use the same prompt structure and `chat_template_kwargs.enable_thinking=false` by default.
A saved non-streaming setting uses the same request lifecycle, but sends one final text result.

The frontend subscribes to Wails events before it starts a request.
It creates a request identifier and passes it with the input.
`StartTranslation` validates the request and returns promptly after acceptance.
Go owns the HTTP request and its cancellation context.
`CancelTranslation` cancels only the matching request.

Use one active translation at a time.
Serialize start, cancellation, and completion state under a small coordinator.
A new accepted request cancels its predecessor.
The predecessor must release its response body before the replacement starts network work.
Every worker has a cancellation path and participates in shutdown.
Do not hold a state lock during network I/O or event delivery.

Events contain `requestID`, `sequence`, and `kind`.
Kinds are `started`, `delta`, `done`, `cancelled`, and `error`.
Delta events contain text. Terminal events contain final accumulated text and status.
An accepted request produces exactly one terminal event while the app remains active.
The frontend ignores old identifiers and already-applied sequence numbers.
Keep request state and event subscriptions for the lifetime of the frontend application.
Page disposal detaches only the page view, not the event subscriptions.
Navigation to Settings does not cancel translation.
Returning to Translation shows the retained text and current request status.
Hiding the window to the tray also leaves the active request and subscriptions intact.
Remove subscriptions only when the frontend application is disposed.
The terminal text replaces the visible text to reconcile a missed update.
There is no automatic replay after app exit or window reload.

Parse SSE at event boundaries, not network read boundaries.
Support LF and CRLF, comments, optional spaces after field names, and multiple data lines.
Network reads can split JSON, UTF-8 characters, or reasoning tags.
Set explicit limits on event size and accumulated output; report exceeded limits.
Do not silently ignore malformed data or provider error events.

Handle empty choices and metadata events without assuming that they contain translation text.
Recognize `[DONE]` and the provider finish reason.
For this provider contract, require `[DONE]` for normal stream completion.
Keep a length-limited finish reason as an incomplete result, even when `[DONE]` follows.
Treat an unexpected end of stream as an error and preserve partial text.
If the live provider uses a different terminal contract, change the parser only with a recorded fixture and a test.

Discard separate reasoning fields.
For legacy inline `<think>` blocks, use one incremental filter for streaming and non-streaming results.
Buffer partial opening tags so reasoning text cannot escape before the full tag arrives.
Define this filter for the leading reasoning block, not arbitrary tags inside translated documents.
An unterminated leading block produces an error rather than exposing its contents.

Batch output events briefly to reduce frontend work, with a proposed maximum delay of 40 milliseconds.
Flush pending text before the terminal event.
Show distinct messages for network errors, deadlines, invalid data, provider limits, and user cancellation.
Do not automatically retry after partial output.
An explicit retry starts a fresh request and clears or replaces the old result.

If input changes during translation, cancel the old request and mark its output as belonging to earlier input.
On application exit, reject new requests, cancel active work, close bodies, and wait for workers within a bounded shutdown period.
Do not emit frontend events after the runtime shuts down.
Logs contain status and request identifiers, not user text, instructions, glossary values, or credentials.

## Tray And Platform Plan

The Wails 2 source examined provides lifecycle hooks and window controls.
It does not provide the Wails 3 `app.SystemTray` API.
Use a separate tray implementation behind OS-specific files.
`getlantern/systray` is a candidate, not a proven Wails 2 integration.

First, build a small Windows integration experiment.
It must show the tray, open the window, hide the window, and exit without a remaining process.
Menu items are Open Translator, Settings, and Quit.
Quit must bypass close-to-tray logic.
The close hook hides the window only when the tray is ready and the user has not requested Quit.
Use Wails runtime functions only after startup supplies the runtime context.
Do not enable unconditional hide-on-close behavior that prevents the tray-failure fallback.

On macOS, the tray library documents main-thread and native-event-loop requirements.
Do not assume that `go systray.Run(...)` is portable.
Its external-loop registration API is only a candidate for integration with Wails.
Prove native-loop ownership and shutdown on macOS before support is promised.

On Linux, test the required tray library packages and the target desktop environment.
Some environments do not provide a tray host.
The app must remain usable as a normal window when the tray is unavailable.
Test both X11 and Wayland where they are release targets.

Build and test on each target OS.
The first supported release target is Windows.
Linux and macOS release support remains conditional on native build and tray results.
Pin an exact Wails 2 version and accepted dependency versions during implementation.
The source version used for this design is evidence of API shape, not a claim about the latest release.

## Verification Plan

Preserve the current prompt behavior with fixture tests for plain text, explicit source, instructions, glossary terms, and JSON constraints.
Add tests for deliberate changes, including automatic target selection and streaming enabled by default.
Do not copy the Python streaming defects into the Go implementation.

Use `httptest.Server` for normal JSON, SSE, delayed responses, malformed events, error events, rate limits, and broken connections.
Split fixtures at every relevant byte boundary.
Exercise cancellation before headers, during output, and immediately before completion.
Test settings snapshots and fast repeated requests.
Do not use sleep-based synchronization in concurrency tests.

Measure detector accuracy, startup time, memory use, and short-input latency.
Exercise traditional Chinese and Japanese-only Han text explicitly.
Record uncertain cases rather than claiming perfect detection.
Add benchmarks for SSE processing and the chosen output batch size before tuning performance.

Frontend tests cover vertical layout, input-method composition, stale events, Stop, Copy, keyboard access, and settings errors.
Use a simulated Wails bridge for component logic.
Use native application tests for the real bridge and tray.
A browser-only test cannot establish native tray behavior.

Required implementation checks are `go test ./...`, `go vet ./...`, and `go test -race ./...` for concurrent code.
Also require TypeScript type checks, frontend tests, a production frontend build, and a Wails build.
Native acceptance tests cover close-to-tray, tray failure, restore, explicit Quit, and shutdown during streaming.
Live provider tests require a deliberate request with non-sensitive sample text.

## Work Completed And Limits

The script passed a Python syntax parse.
Four local simulated-response tests reproduced the stream defects described in this review.
No public API request was sent.
The Python script was not changed.
There is no application implementation to compile, race-test, or package.
The design review identified a settings-navigation event gap.
The revised design keeps stream state and subscriptions at application scope.
The final file diff was examined. The document has no trailing whitespace or non-ASCII characters.

No CodeGraph or gopls query was needed because there is no indexed repository or Go source.
Repository status was inspected with RTK.
The proposed native tray behavior, detector choice, live model support, and provider limits remain unverified.

This document uses short technical sentences.
Full ASD-STE100 compliance requires the official dictionary; no tool guarantees compliance.

## Decisions Before Implementation

Does the first release need image translation and the local proxy, or text translation only?
Is manual Translate sufficient, or is translation after a typing pause also required?
Is simplified Chinese the intended output, or must the app offer traditional Chinese as a separate choice?
Is close-to-tray on Windows acceptable as the default, with the initial window visible?

The proposals in this document permit planning.
Feature scope and uncertain-language behavior need agreement before the acceptance criteria are final.

## Primary Source Record

The following source files were retrieved on 2026-10-14.
The Wails documentation website returned HTTP 403 through the available fetch path.
The versioned source files were used instead.

Wails 2 application options and lifecycle hooks:
`https://raw.githubusercontent.com/wailsapp/wails/v2.10.2/v2/pkg/options/options.go`

Wails 2 event emission:
`https://raw.githubusercontent.com/wailsapp/wails/v2.10.2/v2/pkg/runtime/events.go`

Tray lifecycle and main-thread requirements:
`https://raw.githubusercontent.com/getlantern/systray/master/systray.go`

Tray platform support:
`https://raw.githubusercontent.com/getlantern/systray/master/README.md`

Language detection, candidate languages, and memory considerations:
`https://raw.githubusercontent.com/pemistahl/lingua-go/main/README.md`

SSE format reference:
`https://html.spec.whatwg.org/multipage/server-sent-events.html`
