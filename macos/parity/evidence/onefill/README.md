# One fill (`one_background`)

What the setting paints, which colour it uses and how that was checked. Screenshots are not in the tree (see `../../README.md`);
the scripts and the numbers are.

## Which colour: measured

`contrast.py` (run `python3 -I contrast.py [-v]`, output in `contrast.txt` and `contrast-verbose.txt`) puts each candidate in place of the
three colours the main window painted before (`surface.window` under the terminal, `sidebar.background` under the sidebar and the
status bar, `surface.panel` under the right panel) and measures, over the 55 bundled themes (`DesignSystem/Resources/themes.json`):

1. core's `CONTRAST_PAIRS` (parsed from `contrast.rs`) as core measures them, normal and high-contrast sets;
2. the same at the default look (opacity 0.8, blur), with the text colours `ThemeTokens::translucent` lifts, on the worst desktop it models;
3. the terminal's card (`terminal.background`) against the fill around it, opaque and at 0.8.

| | `surface.window` | `sidebar.background` |
|---|---|---|
| 1. measurements that fell and are under their minimum | 71, in 25 dark themes (0 light); none failed before | 0 |
| of the rest, lower than today but at or above the minimum | 2169 (dark themes) | 418 (19 light themes; tightest `wormhole_volt.light` `text.link` 4.946 to 4.53, min 4.5) |
| 2. default look, lower than today and under the minimum | 152 (text.faint 35 themes, git.branch 33, selected muted 31, text.muted 28, selected text 25), dark themes only | 0 |
| 3. card against the fill, opaque | equal to today in all 55 | higher in 54, equal in `void` (its card is the fill) |
| 3. card against the fill at 0.8 | equal to today in all 55 | higher in 53, equal in `void`, lower in `matrix` (1.0059 to 1.0016) |

`sidebar.background` is the fill (`ChromeColours.oneFill`, the one place that names it). No theme colour was changed.

## What the window does: read from window-shots

Background instances (`--background`, `WORMHOLE_DEVCTL`), the debug build of this branch against a build of the same tree with the Swift files of the commit before the
change, each `window-shot` read with `pix.py` and compared with `cmp.py` (both need NumPy and Pillow; `cmp.py` reads `shots/base-*.png`
and `shots/new-*-off-*.png`). A shot is 2640 x 1640 (the window
at 2x); the blur material is captured as a flat grey, the same in every part, so equal colours compare exactly.

| State | Sidebar, terminal column outside the card, panel, status bar | Lines |
|---|---|---|
| on, dark, opacity 0.8 and 1; light, 0.8 and 1 | one colour in each (dark 0.8: `(50, 55, 60)` over the material; opaque: `sidebar.background`), the card another | none between the columns (colour changes along a row across the window: the card's two edges only), none above the status bar |
| off | as before: the column differs from the sidebar, the panel is `surface.panel` | the dividers at the columns' edges and the rule above the status bar |
| on, Increase Contrast | one colour (the high-contrast set's `sidebar.background`) | back: both dividers and the status bar's rule |
| on, Reduce Transparency | one colour | none |
| on, Liquid Glass | the sidebar and the status bar are glass, no fill; the column and the panel the same colour | none (as with the setting off) |
| on, mid-slide: sidebar in, panel out, panel in, sidebar out | two colours across the whole row: the fill and the card, no seam pixel and no darker band | none |

The setting off against the build before it: dark, opacity 0.8 and 1, identical in all 4,329,600 pixels. Light: identical outside the
terminal's card; inside it the card differs by at most 2 of 255 between two runs (it differs by as much between two runs of the same build:
the Metal view's last easing to the palette, and the material behind it at 0.8).

## Left as it is

The small rings cut out round the title row's waiting-panel dot, a split's gaps and a terminal tile are still drawn in `surface.window`
(`TitleButton.swift`, `PaneFrame.swift`, `TerminalTile.swift`). Swift's main window does not draw the column dividers under Increase
Contrast when the new setting is off, where the GPUI app does (`derive.rs` `dividers`): a separate gap, not part of this change.
