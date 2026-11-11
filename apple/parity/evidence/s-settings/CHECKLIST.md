# Settings parity checklist, round S (lane settings)

Rows come from core's settings index (`SettingsModel/Resources/settings.json`, regenerated from `settings_index`), so every page has
GPUI's rows in GPUI's order, with its labels, descriptions, keywords, search hits and per-row reset by construction (live check in
round P: "shell" = 8 hits in 5 pages). What differs by page is what GPUI draws beyond the index. OK = same and checked, FIXED = this round,
OPEN = still different.

## The 33 keys the punch list named (consumer, and the proof in this folder)

| Key(s) | Consumer now | Proof |
|---|---|---|
| theme_primary, primary_color, branch_color, branch_color_custom | `ThemeSync.applyAccents` -> `ThemeStore.settings` -> core's resolver | keys-a.txt (accent primary/branch change), appearance-hidden-themes.png (orange accent) |
| borders | `ThemeStore.showsBorders` clears `border`, `card.edge`, `settings.divider` (Increase Contrast keeps them) | keys-a.txt (border #e3ebf21f -> #00000000) |
| hidden_themes | `ThemeCards`: Hide theme (menu), "Hidden themes (n)" with Show; a pick stays | appearance-hidden-themes.png |
| icon_animations | `WindowLook.iconAnimations`: panel toggles' strip snaps instead of following the slide | keys-a.txt (state); motion itself not seen |
| ui_font_family, ui_font_size | `ThemeStore.interfaceFont` -> tokens' metrics (every `TextRole`) | keys-a.txt (interface_font Menlo 16) |
| code_font_family, code_font_size, tab_width, size_units | `WindowLook.codeFont`, `\.tabWidth`, `PanelPreferences`, core's `byte_size` through the FFI | keys-a.txt |
| changes_tree, group_untracked, diff_split, diff_wrap | `PanelPreferences` shared by the Changes list and every diff; panel controls write back | keys-a.txt; rows-scope-*.png (flat list, no Untracked section) |
| sidebar_status_dot | `SidebarAppearance.showsStatus` (`row/tile.rs tile_parts`) | dot-on.png / dot-off.png (bell badge gone) |
| sidebar_cmd_hints | `SidebarAppearance.showsCommandHints` | state only (⌘ cannot be held by a script) |
| sidebar_numbers | `setNumbersFollowList` (core), core now raises Chrome | numbers-a.txt (false -> true live, true at launch) |
| sidebar_changes_scope | `RowChanges` (once, on the topmost row of a repository) | rows-scope-repository.png (±2 on one row) vs rows-scope-terminal.png |
| sidebar_changes_plain, sidebar_changes_shared | new core action `SetChangeRules` (+ core test) | plain-a2.txt (none: 0, finishing: 1); shared: core unit test only |
| sidebar_changes_remember | NOT DONE: needs claims saved with the windows (`Store::save_claims`) and `Attribution::restore` at restore | - |
| menu_bar_animate (+ menu_bar_icon) | `AmbientController.animatesIcon/iconMode` -> `StatusSnapshot` | state only: no menu bar item in `--background`; `control ambient` now reads the settings |
| never_hide_lines | already core's (`probes/run.rs` reads it with the Probes' flag); the applier re-reads on change | not seen: needs an output fold |
| auto_update, anonymous_count | `UpdateEngine.setAutoUpdate/setAnonymousCount`, automatic checks (launch + `next_check_in`) | updates-b.txt (local feed: `Wormhole-Count: day, month` at launch; plain user agent after the switch; install tried by the check only while on) |
| settings_translucent, studio_translucent, hosts_translucent | `secondaryBackdrop(.settings/.studio/.hosts)`: blur and opacity of the main window, or opaque | state only (a script's window capture has no desktop behind it) |
| settings_text_size | NOT DONE: `TextRole` reads one global store; the Settings window would need its own scale | - |
| show_config_keys | already consumed (`EntryRowView`) | - |

## Pages
- Window shell, search, nav groups, Esc ladder, size rules: OK (round P).
- Profile: OK (round P). Profile desks: Trust window and the launch-time Access desk follow the active profile FIXED; Session Hosts are the data folder's (same in core); live switch does not remake the hook socket's Access desk OPEN (profiles-a.txt).
- Appearance: accent, branch, borders, icon animations, translucent, hidden themes FIXED. User themes are not among the cards (GPUI lists them) OPEN; per-version cards, Import theme, Move to Trash OPEN.
- Fonts: code and interface rows now take effect FIXED; Settings text size OPEN.
- Terminal, Terminals & sidebar, Changes & diffs: every key has a consumer except Remember after restart OPEN; preview scenes follow the keys that GPUI's do (round P).
- Agents, Notifications, Menu bar, Hotkey, Import, Updates, Advanced, Changed: as round P; Updates now acts on its two switches FIXED.
- Project folders (STX2): picker search ("Search projects") and the "Approval isn't available on this Mac" banner FIXED (banner not seen: the test instance's owner can always answer).
- Extensions (STX1), Feature flags (STX3): unchanged from round R-V.
- Not checkable from a script: pointer, VoiceOver, native menus, the three windows' blur, Reduce Motion playback.
