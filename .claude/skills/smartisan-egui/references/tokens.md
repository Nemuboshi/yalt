# Token reference

Adapted from the pinned upstream `references/tokens.md`; provenance and licenses are in [ATTRIBUTION](../ATTRIBUTION.md). Colors below are original light design values, not egui defaults. Hex colors are sRGB; gradients run top to bottom.

| Semantic role | Light value |
|---|---|
| Page | `#f5f5f5` |
| Surface / field | `#ffffff` |
| Subtle surface | `#fafafa` |
| Body / list title | `#333333` |
| Control / heading text | `#636363` |
| Secondary text | `#7f7f7f` |
| Muted text | `#9d9d9d` |
| Disabled text | `#bababa` |
| Outline / field / group border | `#e2e2e2` |
| Button border | `#eeeeee` |
| Divider | `#f2f2f2` |
| Primary gradient | `#5b87eb` → `#5883eb` |
| Primary stroke | `#3f68e2` |
| Primary pressed fill / stroke | `#4772e6` / `#2f54d7` |
| Primary label | `#ffffff` |
| Selected segment gradient | `#4878f0` → `#6997f4` |
| Selected segment stroke / label | `#3d6fee` / `#ffffff` |
| Kit tab selected fill / stroke | `#b2b2b2` / `#a7a7a7` |
| Text action | `#5079d9` |
| Dialog primary text | `#5e80d0` |
| White control gradient | `#ffffff` → `#fcfcfc` |
| White control pressed | `#ebebeb` → `#f2f2f2` |
| List row pressed fill / stroke | `#5985eb` / `#3f68e2` |
| Toast gradient / text / stroke | `#fcf6e6` → `#fcf3dc` / `#ad8a67` / `#d2b89c` |

## Geometry and typography

Original mobile reference: width 432, margins 12; spacing vocabulary 4, 6, 8, 10, 12, 16, 18, 20, 24. Use egui logical points. Mobile dimensions are reference measurements, not desktop viewport constraints.

| Element | Original radius | Original height | Stroke |
|---|---:|---:|---:|
| Large primary button | 10 | 60 | 0.8 |
| Small button | 4 | 30 | 0.4 |
| Input | 6 | 34 | 0.8 |
| Group card / row | 10 / — | — / 60 | 0.8 |
| Selected segment | 3 | 30 | 0.8 |
| Tabs outer | 6 | 28.8 | 0.4 |
| Dialog | 12 | content-dependent | component-dependent |

Body/control text starts at 14–16 points, subtitles at 12, dialog title at 18. These are source size references, not font-weight guarantees. Use existing licensed fonts with CJK and other required glyph coverage. Larger or smaller desktop sizes must preserve readability and hit targets.

Original shadows include a primary drop shadow `(0, 4, blur 2, black 10%)` and a group shadow `(0, 10, blur 16, black 4%)`. Treat them as visual references: egui shadow parameters are not CSS parameters. Keep depth subtle and verify screenshots.

## Derived states and dark mode

The kit defines no dark palette or general hover/focus specification. Define these explicitly as adaptations; do not label egui defaults as source tokens. Map page, raised surface, field, text, border, accent, disabled, selection, and focus independently. Check readable contrast in each state, including white labels on blue. Document project-approved contrast adjustments as overrides. Keep focus visible beyond color alone and disabled controls noninteractive.
