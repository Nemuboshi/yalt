# Components in egui

Adapted from upstream `references/components.md` at the revision in [ATTRIBUTION](../ATTRIBUTION.md). Use [tokens](tokens.md) for exact light values. Desktop arrangements, hover/focus behavior, and dark mode are local derivations.

## Native implementation contract

Keep a native widget’s response, enabled state, keyboard navigation, and accessibility information when decorating it. A gradient is background decoration, not a second overlapping interactive widget. Native widgets remain responsible for labels and input behavior.

For a solid primary button, scope all relevant widget visuals rather than applying one constant fill. egui 0.36.2 documents that `Button::fill` overrides hover effects and `Button::stroke` overrides state strokes. Check the installed version before relying on this behavior.

For a gradient-backed native control, use this rendering order:

1. Reserve a background shape slot with the painter before creating the control.
2. Add the native widget with its built-in background suppressed through a version-appropriate scoped style. Retain its foreground and focus indication.
3. Select decoration from the actual enabled, selected, pointer-down, hover, and focus response. Resolve disabled first. Use egui’s native activation response for keyboard actions.
4. Replace the reserved slot with the rounded gradient and stroke using the allocated rectangle. The replacement keeps the original paint order, behind labels. Keep a visible focus ring above decoration.

For compound containers, reserve the slot before the children and replace it after measuring their bounds. Do not append an opaque background over text. A rectangular clip is insufficient for rounded corners: use rounded tessellation with interpolated vertex colors or a rounded alpha-masked texture. Cache textures or reusable meshes; invalidate on relevant size, pixels-per-point, theme, radius, or state changes. Avoid rebuilding/uploading resources every frame. Limit cache growth during resizing.

### Installed egui 0.36 pitfalls

In the installed egui 0.36.2, `RectShape::with_texture` combined with a nonempty stroke can panic during final tessellation in mesh `colored_vertex`. Paint the textured rounded fill with `Stroke::NONE`, then a separate untextured stroke shape. Put both shapes, fill first, in `Shape::Vec` when replacing the reserved background slot. Keep the stroke shape’s fill transparent.

Call `ctx.load_texture` **outside** any `ctx.data_mut` closure: loading a texture while holding that context-data lock can deadlock. Read the cache and determine misses under the lock, release it, load the texture, then reacquire the lock to store the handle. These failures were reproduced in the application implementation; check the installed version before generalizing them to other releases.

## Component mapping

| Source component | Native adaptation |
|---|---|
| Primary / small button | Native `Button`, primary tokens, separate pressed/disabled/focus states |
| Grouped list | Rounded white `Frame`, fine border, native row children, subtle dividers |
| Segmented selection | Native selectable widgets with stable selection data; blue selected gradient |
| Kit tabs | Native selection with gray selected fill; blue tabs are an explicit segment-derived variant |
| Input / search | Native `TextEdit`, stable IDs, bounded width, thin field border |
| Dialog | Native modal/dialog API for the installed version, bounded content, native action buttons |
| Menu | Native menu interactions and dismissal, restrained surface/borders |
| Switch / radio / slider | Native controls; adapt decoration without replacing keyboard semantics |
| Toast | Nonblocking notification, warm surface, bounded text and lifetime |

Original large button: height 60, radius 10, subtle shadow; small button: height 30, radius 4, no shadow. Large disabled button keeps its normal treatment at approximately 30% opacity in the source; implement noninteraction and verify readability. Hover and keyboard focus require derived treatments.

Original grouped lists use white cards, radius 10, gray outline, 60-high rows, title/subtitle separation of 4, and faint dividers. Desktop row heights may adapt to density, wrapping, and scale. Give long content a bounded scroll region.

Original segmented selection uses a 30-high blue block within a 60-high row. Source tabs use gray selection and a 28.8-high outer container. Do not confuse these components merely because both select one value.

## Desktop layout and input

Compute child bounds from the available viewport after reserving fixed controls. Keep size and IDs stable when status text changes. Wrap or scroll long output within allocated space; do not issue content-fit viewport resize commands each frame. Keep controls reachable in narrow windows and at high display scale.

Use native text selection, clipboard, undo, context menus, and IME. Composition spans frames; an empty preedit event list alone does not prove composition ended. Handle commit/cancel and submit shortcuts without discarding text or submitting prematurely. Test with an actual IME when available; simulated input does not establish platform IME correctness.

Keep blocking operations outside frame rendering and request repaint for state changes or active animation. Load existing licensed fonts once; do not infer real font weights from a styling flag.

## Acceptance exercise

On a small desktop editor with a primary action and segmented mode selector, check exact applied light colors, derived dark colors, every interaction state, paint order, rounded corners, and cache invalidation. Resize with long content, change display scale, traverse by keyboard, and compose CJK text. Use egui inspection/screenshot tools and repeatable UI tests where available. Report any checks that could not run.
