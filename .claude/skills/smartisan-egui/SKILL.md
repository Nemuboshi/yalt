---
name: smartisan-egui
description: Use when designing, implementing, or reviewing Smartisan-inspired Rust egui or eframe interfaces, including restrained gradients, grouped surfaces, primary actions, segmented controls, and light or derived dark themes.
---

# Smartisan-inspired egui

Translate Smartisan’s restrained depth into native egui behavior: quiet gray surfaces, fine borders, subtle gradients, compact groups, and clear interaction states. This is an unofficial adaptation; desktop sizing and dark mode are derived, not original specifications. Read [attribution](ATTRIBUTION.md) before redistribution.

## Workflow

1. Inspect the installed egui version, theme application, widget helpers, layout bounds, and existing tests. Consult version-matched API documentation and available Rust/egui tools. Read project documentation for application-specific overrides.
2. Read [tokens](references/tokens.md) and [components](references/components.md). Define semantic roles for surfaces, text, borders, primary actions, selection, and focus. Compare actual applied colors against the tokens: egui light defaults are not the Smartisan palette. Define a separate, contrast-checked derived dark palette.
3. Use native `Button`, `TextEdit`, selection controls, menus, sliders, and dialogs wherever possible. Scope style changes to the intended controls. Keep persistent data separate from temporary UI state and business logic separate from painting.
4. Specify inactive, hover, pressed, disabled, selected, and keyboard-focus appearances. A constant `Button::fill()` overrides hover fill effects; `stroke()` likewise overrides state strokes. Use scoped widget visuals for solid states or response-driven decoration for gradients. Keep action colors separate from text-selection colors.
5. Allocate stable geometry within the available viewport. Bound editors and long results with scrolling; reserve toolbar/footer space. Keep the user’s window size stable across frames. Use stable widget IDs and native text editing for selection, clipboard, undo, accessibility, and IME. Respect composition through commit/cancel before triggering submit shortcuts.
6. Reserve a background paint slot **before** adding foreground widgets; replace that slot after their rectangle and response are known. Preserve focus outlines. Render gradients with rounded geometry or a rounded alpha mask: rectangular clipping alone does not round corners. Cache gradient textures or reusable geometry by size, scale, theme, and state; rebuild only when those inputs change. Load no resources per frame; move blocking work off the UI thread.
7. Verify light/dark, normal/hover/pressed/disabled/selected/focus, narrow/wide windows, scaling, keyboard traversal, long text, and CJK composition. Use egui inspection tools and screenshots when available, plus repeatable UI tests. Run project-required checks and report unavailable tools and untested behavior honestly.

## Quick reference

| Need | Starting point |
|---|---|
| Primary action | Blue vertical gradient, thin stroke, distinct pressed fill |
| Grouped content | White surface, gray outline, small radius |
| Blue selection | Segmented-control tokens; kit tabs are gray |
| Typography | Existing licensed fonts with verified glyph coverage |

## Common mistakes

Avoid oversized cards, vivid gradients, heavy shadows, content-driven window growth, and painted replacements for native text input. Do not claim font-weight fidelity without loading and selecting the actual font face. Download no proprietary fonts, icons, or screenshots as part of this skill.
