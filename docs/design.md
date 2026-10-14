# Visual Design System

## Purpose

This document defines the visual language for the yalt desktop app.
The frontend uses HTML and CSS to simulate the design language of macOS.
The native Wails window frame remains responsible for window controls and window
behavior.

This document covers:

- The app shell
- Typography
- Color
- Spacing
- Controls
- Content surfaces
- Interaction states
- Accessibility
- Responsive behavior

The product scope and behavior are defined in `docs/spec.md`.

## Design direction

The interface must feel like a small macOS utility.
It must not feel like a web dashboard.

Use these principles:

1. Keep the layout compact.
2. Use the system font and system-like controls.
3. Use one clear primary action.
4. Use quiet surfaces and thin separators.
5. Keep content more important than decoration.
6. Use the accent color for action and focus.
7. Show state with text and shape, not with color alone.
8. Keep the interface usable with the keyboard.

Do not use:

- Large decorative gradients
- Heavy card shadows
- Full-page hero sections
- All-caps labels with wide letter spacing
- Custom window buttons
- Decorative glass behind the main content
- Color as the only error or status signal

## Platform boundary

The frontend simulates the macOS design language.
It does not replace native window behavior.

The app must keep the native window frame when the platform provides one.
The HTML layer must not draw fake traffic-light buttons.
The HTML layer must not draw a fake title bar that looks active but cannot move
or resize the window.

The HTML layer owns:

- The toolbar content
- Navigation
- Form controls
- Translation content
- Status messages
- Settings content

The native layer owns:

- Window movement
- Window resizing
- Window close, minimize, and maximize behavior
- The system menu bar when the platform supports it
- Tray behavior

## App shell

The main window contains three visual layers:

1. Toolbar
2. Content
3. Status feedback

The toolbar contains:

- The current view control

The toolbar does not show the application name or a connection state.
The native window title bar shows the application name.

The translation view is the main view.
Settings remains a separate view in the first implementation.
The Settings command must also be available from the application menu when the
platform provides an application menu.

Use a segmented control for two peer views.
Do not use a large tab bar.
Do not use a sidebar for only two views.

The translation view contains:

- Source and target language controls
- The input editor
- The Translate and Clear actions
- The output editor
- The Copy action
- The streaming or completion state

Keep the input above the output at all window sizes.
If the window height is too small, let the content scroll.
Do not hide actions to preserve a fixed height.

## Typography

Use the system font on macOS.
Use platform fallbacks on Windows and Linux.
Keep the CJK fallback list after the system font.

```css
font-family:
  -apple-system,
  BlinkMacSystemFont,
  "SF Pro Text",
  "PingFang SC",
  "Hiragino Sans GB",
  "Noto Sans CJK SC",
  sans-serif;
```

Use these text roles:

| Role | Size | Weight | Use |
| --- | ---: | ---: | --- |
| Window title | 13 px | 600 | Toolbar title |
| Section title | 15 px | 600 | Settings groups |
| Body | 13 px | 400 | Labels and normal content |
| Editor text | 15 px | 400 | Input and output text |
| Secondary | 12 px | 400 | Help text and secondary state |
| Caption | 11 px | 500 | Small status details |

Use normal letter spacing.
Do not use letter spacing to make a label look more important.

Keep text line height between 1.35 and 1.6.
Use a larger line height for CJK text when the font needs it.

Do not bundle a font for the macOS visual layer.
The system font provides the closest platform match.
Use the fallback list when the app runs on another platform.

## Color system

Use semantic tokens.
Components must not use raw color values.

The default light theme uses these starting values:

```css
:root {
  color-scheme: light dark;

  --window-background: #f5f5f7;
  --content-background: rgba(255, 255, 255, 0.78);
  --control-background: rgba(255, 255, 255, 0.9);
  --control-background-hover: rgba(255, 255, 255, 1);
  --separator: rgba(60, 60, 67, 0.18);

  --label: #1d1d1f;
  --secondary-label: #6e6e73;
  --tertiary-label: #86868b;
  --placeholder: #8e8e93;

  --accent: #8a5a3b;
  --accent-hover: #74492e;
  --accent-pressed: #5f3b24;
  --accent-tint: rgba(138, 90, 59, 0.12);
  --accent-label: #ffffff;

  --success: #248a3d;
  --warning: #9a6700;
  --destructive: #ff3b30;
  --focus-ring: rgba(138, 90, 59, 0.45);

  --shadow-popover: 0 8px 24px rgba(0, 0, 0, 0.16);
}
```

Use a dark theme with the same semantic names:

```css
@media (prefers-color-scheme: dark) {
  :root {
    --window-background: #1e1e1e;
    --content-background: rgba(38, 38, 40, 0.84);
    --control-background: rgba(50, 50, 52, 0.9);
    --control-background-hover: rgba(62, 62, 64, 0.96);
    --separator: rgba(235, 235, 245, 0.18);

    --label: #f5f5f7;
    --secondary-label: #98989d;
    --tertiary-label: #8e8e93;
    --placeholder: #8e8e93;

    --accent: #c69c7b;
    --accent-hover: #d6b192;
    --accent-pressed: #e3c6ac;
    --accent-tint: rgba(198, 156, 123, 0.18);
    --accent-label: #1d1d1f;

    --success: #30d158;
    --warning: #ffd60a;
    --destructive: #ff453a;
    --focus-ring: rgba(198, 156, 123, 0.55);

    --shadow-popover: 0 8px 24px rgba(0, 0, 0, 0.36);
  }
}
```

The accent color is a warm brown.
It identifies the primary action and keyboard focus.
The app must not use blue or teal as a primary or secondary color.
Use `--accent-label` for text on an accent-filled surface.
Use a separate status color only when the status needs a meaning that the
accent color does not provide.

## Spacing and shape

Use a 4 px spacing unit:

```css
:root {
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-7: 32px;
}
```

Use these shape values:

```css
:root {
  --radius-control: 6px;
  --radius-group: 10px;
  --radius-popover: 10px;
}
```

Use a one-pixel separator for grouped content.
Do not add a shadow to normal content.
Use `--shadow-popover` only for menus, popovers, and floating notices.

## Toolbar

The toolbar must remain compact.
It must use a quiet background and a bottom separator.

Recommended structure:

```html
<header class="toolbar">
  <nav class="segmented-control" aria-label="View">
    <button class="segment is-selected" type="button">Translate</button>
    <button class="segment" type="button">Settings</button>
  </nav>
</header>
```

The toolbar holds only the view control.
The native title bar provides the window title.

## Segmented control

Use the segmented control only for two or three peer views.

The control must provide:

- A selected state
- A hover state
- A pressed state
- A visible keyboard focus state
- An accessible label

The selected segment uses the control background.
The unselected segment uses the window background.
Keep the contrast between these states small.

## Form controls

Use native HTML elements where they provide the required behavior:

- Use `button` for actions.
- Use `select` for the language lists.
- Use `textarea` for translation input and output.
- Use `input` for settings values.
- Use `label` for every form control.

Do not replace a native control with a `div` unless the native control cannot
provide the required behavior.

Labels use normal sentence case.
Keep the label close to its control.
Place help text below the control.

Controls use:

- A minimum height of 30 px
- A 6 px corner radius
- A one-pixel separator
- A clear focus ring
- A system-like background

A `select` must hide the native appearance.
Use one chevron from a CSS custom property.
Style the option list with the control tokens.

When the source language is auto-detected, append "(Auto-detected)" to the
selected option text.
Keep the option value as the language code.
Remove the text when the user selects another option.

The input editor and result editor use a larger 10 px group radius.
The result editor remains read-only.

## Buttons

Use three button levels:

1. Primary button
2. Secondary button
3. Text button

Use the primary button for Translate.
Use the secondary button for Stop when a request is active.
Use a text button for Clear and Copy.

Buttons must use sentence case.
Buttons must show a disabled state without hiding their label.
Buttons must not show a pointer cursor.
Keep the default cursor for every clickable element.
Destructive buttons must use the destructive color only when the action is
destructive.

The Translate action must remain available with `Cmd+Enter` on macOS.
Keep the equivalent Windows and Linux shortcut for those platforms.

## Translation content

The translation page must use one vertical content column.
The source editor appears first.
The result editor appears second.

The editor group contains:

- A short section label
- The editor surface
- A compact action row

The result group contains:

- A short section label
- The target language name
- The result text
- A Copy action
- A completion or incomplete state

Streaming output must not create a new layout for each token.
Keep the result position stable while the text changes.

The result editor scrolls inside its own surface when the text is long.
Keep the action row visible.
The editor and result surfaces must not grow without limit.

If the user stops a request or a request fails, keep the partial result.
Show an explicit incomplete state.
Keep Copy available for partial output.

## Settings

Settings use grouped sections.
Each group contains related fields.

Use this structure:

```html
<section class="settings-group">
  <h2>Connection</h2>
  <label class="setting-field">
    <span>Base URL</span>
    <input type="url">
    <small>Use HTTPS for remote endpoints.</small>
  </label>
</section>
```

Keep one field per row unless two fields form one clear pair.
Do not place unrelated settings in the same group.
Show the resulting endpoint near the base URL and model fields.
Show save feedback near the Save button.

Restore Defaults must open a confirmation dialog before it changes the form.
Use a native `dialog` element with `showModal`.
Use the content tokens, the group radius, and the popover shadow.
Do not use the browser default dialog style.
Do not use `window.confirm`.

## Materials and effects

The toolbar can use a translucent material when the platform supports it.
The toolbar must have an opaque fallback.

The editor and result surfaces must remain readable when the background behind
the window changes.
Do not apply blur to the full page.
Do not use blur as a replacement for contrast.

Use `backdrop-filter` only as progressive enhancement:

```css
.toolbar {
  background: var(--content-background);
  border-bottom: 1px solid var(--separator);
}

@supports (backdrop-filter: blur(20px)) {
  .toolbar {
    backdrop-filter: blur(20px);
  }
}
```

## Interaction states

Every interactive component must define these states:

- Default
- Hover
- Pressed
- Keyboard focus
- Disabled
- Error

The translation controls must also define:

- Waiting
- Streaming
- Complete
- Incomplete

Use a visible two-pixel focus ring.
Use `:focus-visible`.
Do not remove the browser focus outline without replacing it.

Use text for errors.
Use an icon or shape with the text when it improves recognition.
Do not use a color change as the only state change.

## Selection and cursor

Keep the page text not selectable.
Permit selection only in the text controls:

- The input editor
- The result text
- The settings fields
- The endpoint preview

Do not use a pointer cursor for buttons, tabs, and labels.
Use the default cursor for these elements.
Permit the pointer cursor only where the platform requires it.

## Motion

Keep motion short and useful.

Use motion for:

- A short view transition
- A streaming activity indicator
- A status change when the status is not already visible

Do not animate the full layout during streaming.
Do not animate each output token.
Do not use motion to communicate information that is not also available as text.

Respect reduced motion:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms;
    animation-iteration-count: 1;
    scroll-behavior: auto;
    transition-duration: 0.01ms;
  }
}
```

## Accessibility

The interface must:

- Provide a label for every form control.
- Keep keyboard focus visible.
- Preserve keyboard access to every action.
- Use status text for connection, progress, completion, and error states.
- Keep normal text readable against its background.
- Respect reduced motion.
- Keep the result available to assistive technology.
- Avoid announcing every streamed token.

Use `aria-live="polite"` for final status changes.
Do not use a live region for the full streaming result.

Use `aria-busy="true"` on the result group while a request streams.
Remove it when the request completes, stops, or fails.

## Responsive behavior

The first window size can remain 760 by 860 logical pixels.
The design must work at smaller sizes.

At narrow widths:

- Keep the language controls in one row when they fit.
- Move the language controls into a vertical stack when they do not fit.
- Keep the primary action visible.
- Let the content scroll.

At narrow heights:

- Keep the toolbar visible.
- Keep the editor actions reachable.
- Let the result content scroll.
- Do not hide the Stop or Copy action.

## CSS organization

Keep the visual system in `frontend/src/styles.css`.
Organize the file in this order:

1. Tokens
2. Reset and base rules
3. App shell
4. Toolbar and navigation
5. Form controls
6. Translation view
7. Settings view
8. Feedback states
9. Responsive rules
10. Reduced-motion rules

Components must use semantic token names.
Do not add raw color values to component rules.
Do not add a new spacing value when an existing token fits.

## Acceptance checklist

The implementation is ready when:

- The window reads as a compact macOS utility.
- The page does not look like a dashboard.
- The toolbar and controls use system-like proportions.
- The app supports light and dark appearance.
- The system font is used on macOS.
- The main action is clear.
- The input remains above the output.
- Keyboard focus is visible.
- `Cmd+Enter` starts translation on macOS.
- Stop and Copy remain reachable during streaming.
- Partial output remains available after Stop or error.
- Reduced motion removes non-essential animation.
- No component uses a raw color outside the token block.
- The accent color is brown, not blue.
- The toolbar shows no application name and no connection badge.
- The result editor scrolls inside its own surface.
- Page text is not selectable outside the text controls.
- Buttons, tabs, and labels use the default cursor.
- Restore Defaults opens the styled confirmation dialog.
- The language and settings select controls match the control style.
- The design works at the proposed window size and at smaller sizes.

## Source guidance

Use the Apple Human Interface Guidelines as the source for macOS patterns.
Use the guidelines for windows, layout, controls, materials, menus, and
accessibility.

The app uses HTML and CSS, so these patterns are visual references.
Native behavior still belongs to Wails and the operating system.
