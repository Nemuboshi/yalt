# Parity and Acceptance Checklist

Status of every feature the previous Go/Wails application provided, plus the
acceptance criteria from `docs/spec.md`.

Legend: **Verified** — exercised by an automated test or by running the
application. **Implemented** — code and tests exist, but the check needs a
platform or interaction not available here. **Not verified** — no evidence.

## Desktop features

| Feature | Status | Evidence |
| --- | --- | --- |
| Native window, resizing, minimum size | Verified | Running application; `theme` size tests |
| Tray icon with Open Translator / Settings / Quit | Implemented | Registered against a running build; menu *click* path not automated |
| Close to tray | Verified | Running application: window hidden, process alive |
| Start hidden | Verified | Running application: window created hidden |
| Tray failure leaves the window reachable | Verified | Running application with the tray disabled: window shown, close exits |
| Quit bypasses close-to-tray | Verified | `window_action` tests; quit path calls `App::shutdown` |
| Window icon and version metadata | Verified | Read back from the built executable |
| Clipboard copy of results | Implemented | `ctx.copy_text`; paste verified by hand during UI checks |
| Settings in a separate directory | Verified | Go settings byte-identical after a fresh Rust launch |
| Bounded shutdown | Verified | `shutdown_cancels_active_work_within_its_deadline` |

## Translation behavior

| Feature | Status | Evidence |
| --- | --- | --- |
| Chat Completions and Responses payloads | Verified | `translation::client` tests; both endpoints asserted |
| Streaming (SSE) | Verified | Lifecycle tests over local fixtures |
| Buffered responses | Verified | Lifecycle tests over local fixtures |
| SSE framing independent of chunk boundaries | Verified | Stream tests: split tags, split UTF-8, CRLF, comments, no-space fields |
| Reasoning filtering (`<think>`) | Verified | `think` tests, including split and unfinished blocks |
| Partial output preserved on Stop and on failure | Verified | `stopping_preserves_partial_output`, incomplete-status tests |
| Total request deadline | Verified | Lifecycle tests with a bounded deadline |
| Language detection and uncertainty | Verified | `languages` tests; threshold measured, see below |
| Automatic target rules | Verified | `select_target` tests |
| Manual source/target precedence | Verified | `resolved_source` tests; stale detection discarded |
| Prompt construction and glossary ordering | Verified | `prompt` tests, including exact output and normalization |
| Glossary import validation | Verified | Malformed JSON, non-string values, duplicates, failed-import preservation |
| Settings round trip, validation, recovery | Verified | `settings` tests, including corrupt-file retention and failed replacement |

## Interface

| Feature | Status | Evidence |
| --- | --- | --- |
| Visual identity: brown accent, both appearances | Verified | Theme tests; screenshots reviewed |
| CJK, kana, and Hangul rendering | Verified | Glyph-coverage test per script; screenshot with all three |
| Focus ring wraps the editor card | Verified | Screenshot |
| Interface labels not selectable; result selectable | Verified | Screenshot; read-only buffer is selectable but not editable |
| Copy feedback in the status bar | Verified | Screenshot |
| Keyboard submission (Ctrl/⌘+Enter) | Implemented | Suppressed during IME composition; not verified on a real IME |
| Glossary rows survive deletion (stable IDs) | Verified | Row-keyed IDs; `settings` view tests |
| Navigation preserves active translation | Verified | View switching does not touch request state |
| Small layout and display scaling | Not verified | Needs a real display; minimum size is enforced in code |

## Platform matrix

| Platform | Build | Run | Tray | Notes |
| --- | --- | --- | --- | --- |
| Windows x86_64 | Verified | Verified | Verified | All checks run here |
| macOS (arm64, x86_64) | CI | Not verified | Not verified | No macOS machine available |
| Linux x86_64 | CI | Not verified | No backend | No Linux machine available |

## Detector calibration

Detection uses `whatlang`. The uncertainty floor (`UNCERTAINTY_FLOOR`,
`src/translation/languages.rs`) was chosen from measurements rather than a
conventional confidence threshold. No threshold separated correct from
incorrect detections: a confident kanji-only Japanese sample can be read as
Chinese, while a correct short sample scores near zero.

The floor therefore only flags short or weakly-attested input as uncertain. It
never overrides a manual selection. It is documented in the source as a
heuristic, not a calibrated probability.

Two guards beyond the floor make a result usable as a provider code:

- The detector is restricted to the offered languages, and a result with no
  provider code is reported as uncertain. The restriction is not sufficient on
  its own: scripts that identify a single language (Bengali, Thai, Greek,
  Hangul, kana) are resolved without consulting the candidate list, so the
  explicit check in `detect` is the backstop.
- The floor rejects text whose script alone decided the language. A bare
  `"12345"` is not a language, but a Latin-script classifier can only answer
  with a Latin-script language.

### Detector choice

`lingua` was the first detector. Its per-language models dominated the binary:
292 MB of a 295 MB build, and 146 MB of the 165 MB build after its cargo
features were restricted to the 30 offered languages. Restricting the features
also changed detector behavior, because the candidate set changes.

`whatlang` replaced it: 0.6 MB linked, a 19.5 MB release binary, and better
handling of short input — `"привет"` scores 0.10 and is correctly uncertain,
where `lingua` scored 0.502 and asserted Russian. On the 32-sentence corpus
covering every offered language, `whatlang` scores 30/32 and `lingua` 31/32;
each misses one hard pair (`whatlang`: `ru` → `bg`; `lingua`: `nb` → `da`).

`whichlang` was measured and rejected. It supports 16 languages, so 14 of the
30 offered languages — Ukrainian, Persian, Czech, Greek, Danish, Hungarian,
Finnish, Norwegian, Slovak, Bulgarian, Polish, Romanian, Thai, and Indonesian
— have no code at all; on the corpus it scored 10/32. It also returns a bare
`Lang` with no confidence, and answers `Eng`/`Ita`/`Fra` for empty, whitespace,
and emoji input, so it cannot express "uncertain".

## Known limitations

- **Tray menu clicks are not automated.** The behavior behind each command is
  covered, and tray registration is confirmed on a running build, but the click
  itself needs a human. Windows 11 keeps new tray icons in its overflow
  flyout, where they are not reachable by UI Automation.
- **Linux has no tray backend.** Both candidates need facilities that cannot be
  assumed. Window behavior is unaffected.
- **macOS and Linux have never been run.** CI proves they compile and their
  tests pass; it does not establish native behavior.
- **No live provider request has been made.** Fixtures prove protocol handling,
  not that a named model is currently served.
- **The detector misses one hard pair.** Russian prose in the corpus is read as
  Bulgarian (confidence 0.65, so not flagged uncertain). Ukrainian is read
  correctly, so this is not a general Cyrillic problem; the previous detector
  missed a different pair instead. A manual source selection is the remedy.
- **The API key is stored in plaintext**, as before.

## Definition of done

From `plan.md` §13:

- [x] Core logic is independent of the UI; no blocking work runs in UI frames.
- [x] Persistent settings and temporary UI state are separate.
- [x] The HTTP client, detector, fonts, and platform resources are reused.
- [x] Both APIs, partial results, cancellation, detection, glossary, and
      settings pass deterministic tests.
- [x] Screenshots reviewed after visual changes.
- [x] Existing settings remain untouched; fresh configuration is safe.
- [x] Formatting, Clippy, tests, and release builds pass.
- [x] Documentation covers builds, paths, environments, and known limits.
- [x] Obsolete build code is not removed before parity is demonstrated; it is
      retained for reference and excluded from all workflows.
- [ ] Product acceptance criteria pass on all three platforms.
- [ ] Native tray, clipboard, IME, accessibility, and scaling checks pass on
      all three platforms.
- [ ] Packaged startup is verified on a clean environment.

The unchecked items all require a macOS or Linux machine, or a clean
environment, and cannot be established from this one.
