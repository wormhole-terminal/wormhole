| scenario | shift px | pass % | fill % | mad | mad blur | same |
|---|---|---|---|---|---|---|
| dark-blocks | 0,0 | 99.96 | 100.0 | 0.58 | 0.356 | yes |
| dark-contrast | 0,0 | 98.88 | 99.96 | 4.389 | 2.796 | yes |
| dark-cursor-bar | 0,0 | 99.85 | 100.0 | 0.288 | 0.171 | yes |
| dark-cursor-block | 0,0 | 99.85 | 100.0 | 0.284 | 0.167 | yes |
| dark-cursor-underline | 0,0 | 99.87 | 100.0 | 0.285 | 0.17 | yes |
| dark-find | 0,0 | 99.83 | 100.0 | 10.432 | 5.057 | yes |
| dark-find-bar | 0,0 | 85.26 | 62.18 | 14.637 | 12.716 | no |
| dark-images | 0,0 | 99.83 | 99.03 | 0.747 | 0.472 | yes |
| dark-minimap | 0,0 | 99.93 | 99.98 | 11.692 | 5.657 | yes |
| dark-minimap-strip | 0,0 | 98.5 | 100.0 | 1.913 | 1.645 | yes |
| dark-pad | 0,0 | 100.0 | 100.0 | 3.482 | 2.112 | yes |
| dark-selection | 0,0 | 98.88 | 99.96 | 4.43 | 2.822 | yes |
| dark-text | 0,0 | 98.88 | 99.96 | 4.389 | 2.796 | yes |
| light-blocks | 0,0 | 99.91 | 99.98 | 0.592 | 0.364 | yes |
| light-contrast | 0,0 | 98.11 | 99.91 | 4.399 | 2.741 | yes |
| light-cursor-bar | 0,0 | 99.87 | 100.0 | 0.315 | 0.187 | yes |
| light-cursor-block | 0,0 | 99.87 | 99.98 | 0.322 | 0.194 | yes |
| light-cursor-underline | 0,0 | 99.89 | 100.0 | 0.313 | 0.186 | yes |
| light-find | 0,0 | 95.5 | 98.46 | 10.925 | 5.42 | no |
| light-find-bar | 0,0 | 39.74 | 46.15 | 20.646 | 18.205 | no |
| light-images | 0,0 | 99.49 | 99.03 | 0.772 | 0.481 | yes |
| light-minimap | 0,0 | 95.54 | 99.98 | 12.002 | 5.767 | no |
| light-minimap-strip | 0,0 | 100.0 | 100.0 | 0.989 | 0.741 | yes |
| light-pad | 0,0 | 98.67 | 100.0 | 3.49 | 2.06 | yes |
| light-selection | 0,0 | 98.13 | 99.91 | 4.431 | 2.76 | yes |
| light-text | 0,0 | 98.13 | 99.91 | 4.431 | 2.76 | yes |

Run: visual-run.sh with SKIP_SETTINGS=1 on this tree (release build, `fontSmoothing` on), GPUI debug build with devctl at main's target, scratch dirs, both apps in background mode,
window 1320x820 at scale 2, `--swift-origin 8,8`, 1 px interior rule (see MEASURING.md). `same` = pass >= 98 %, fill >= 99 %, no shift.
Not same, and why:
- dark/light-find-bar: the find bar is a SwiftUI field over the Metal view and GPUI's is its own: the field's inset, height and text position differ (looks of T12 stays 0.75).
- light-find, light-minimap (grid, the scrollback scene): dense `#` lines carry about 3 % more ink in Swift on a light terminal (CoreText stem darkening; mean of a `#` run 230/178/188 against GPUI's 232/185/194), over the 12-level block tolerance on 15 blocks a row. Tried
  `fontSmoothing = false`: the light scenes pass (99.9 %) but the dark ones fall to 87-96 %, so GPUI's raster is bolder on dark and lighter on light than either of CoreText's two modes; it needs a coverage curve chosen by the text's contrast with its background (not done).
