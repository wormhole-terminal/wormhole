# Settings pages, wave 2 (lane settings-b): page by page against the GPUI page

How to read this. The Settings window keeps native SwiftUI controls, the native sidebar and SF Symbols (the owner's decision), so
`looks = 1` there means every row, state and effect is present and tidy on the 4-pt grid, in dark and light, not GPUI's pixels. Rows,
their order, labels, descriptions, keywords, hit counts and per-row reset arrows come from core's `settings_index` for both apps, so a
page cannot lack a row or order it differently by construction; what can differ is what GPUI draws beyond the index (the "beyond"
column). Captures of every page in dark are in `pages/swift-<id>-dark.png` (window 920 x 1300). Light captures of each page were
not made in this wave: looks stays 0.75 on ST01 to ST03.

What was driven (transcripts in this folder): `controls.txt` (search with hit counts: "shell" 8 hits in 5 pages, "font" 11 in 3; a
set and its effect; Import review and apply from a FAKE Ghostty home; profile edit and new), `themes.txt` (Import theme by a real
pointer click through a scratch-file seam, the card's menu, Move to Trash with its question held for the driver), `trust-access.txt`
(project trust window and agent access questions). Not driven by pointer or keys in this wave: each toggle and select of every page
once, the shortcut recorder's record/conflict/clear, the font picker popover, the profile sheet's delete question. Those stay as the
earlier lanes proved them (`../s-settings/`, `../s2-settings/`); no page was "driven row by row".

| Page | Rows vs GPUI | Beyond the index | Result |
|---|---|---|---|
| Profile | same | profile form fields; Danger zone | unchanged; a profile created live (`profile-new`) |
| Changed | same | list of changed rows | unchanged |
| Appearance | same | theme cards, preview scenes, accent | **new:** Import theme… (scratch seam `WORMHOLE_THEME_IMPORT_FILE`, no open panel), user themes carry their file so a pick that names the file (path) marks the card, Move to Trash… on a user theme with core's question and its pick reset (real-FFI test `theme_files`; live `themes.txt`). Still open: per-version cards (a family's versions), the Theme Studio button beside Import |
| Fonts | same | font picker | unchanged |
| Motion | same | four looping preview scenes | still open: the looping scenes |
| Terminal | same (all rows seen live) | Shells list, worktree caption | **fixed:** Default shell's face is the shell's name ("zsh"), as GPUI's select; the Shells list and worktree caption were already built (the old checklist line was stale) |
| Terminals & sidebar | same | one preview scene (GPUI: three densities) | still open: the two other density scenes; Remember after restart (sidebar lane) |
| Changes & diffs | same | previews | unchanged |
| Coding agents | same | skills, MCP, access list | unchanged |
| Import | same | review cards | **driven:** `import-open` and `import-apply` against a fake home wrote font_size 17 and opacity 0.92 into config.json (`controls.txt`, and the welcome's "Use my Ghostty settings" in `welcome-walk.txt`). Never this Mac's Terminal.app or Ghostty |
| Notifications | same | preview scene | unchanged; the permission word reaches the welcome (outside bundle in a test instance) |
| Menu bar, Hotkey, Updates | same | previews, recorder, update details | unchanged |
| Advanced | same | Reset all, Tips | still: Reset tips never disables |
| Project folders | same | picker, banner | unchanged; the trust window now makes the Project tab follow (below) |
| Extensions, Feature flags | same | probes sheet, flags | unchanged |

Bugs found and fixed while driving: the project commands window approved through a desk of its own, so the Project tab kept saying
"not trusted" (the hub cached its trust store): `Hub::reload_trust` on each `project()` read, with a real-FFI test, and the window
tells the tab to read again.
Not fixed: the window is one per key, so opening it again for another project keeps the first project's (A06).
