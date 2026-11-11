# Themes lane: icon motion (X01, gap 37) and Increase Contrast outlines (X05, gap 42)

Base `exp/swift` 271894722, branch `exp/swift-s3-themes-motion`. A line is ticked `[x]` only after it ran (a real-render test in
`apple/swift/Tests/UITests/IconMotionTests.swift`, or a `--background` instance driven through the control folder, transcripts in this
folder). `[m]` is a hover-only or system-only line: it is a numbered step of `apple/parity/MANUAL.md` ("Themes lane: icon motion").

## What GPUI does: icon motion (`style/icon_motion/`, `style/buttons.rs`, `style/controls.rs`)

- [x] Gate (test `testAStartPlaysTheMotion…`: Animate icons off and Reduce Motion each leave the copy icon untouched; live: `gate-off-reduce-on.png`, `icon_animations` is one flag read by every window through `\.iconAnimations`; Settings/Studio/About use `.themed()` too): `UiColors::icon_motion` follows Animate icons (`icon_animations`) in every window (main, Settings, Studio, About); Reduce Motion
      shows an animation's last frame, so nothing plays (`icon_motion/mod.rs:5-9`, `settings/tests.rs:207`).
- [x] Nothing plays at count 0 (the picture at rest is the plain icon, test `testAMotionsPictureAtRestIsThePlainIcon`; the panel's tab glyph only counts from `onChange(of: isSelected)`):: a window opening or a panel coming back shows icons at rest; a start made while the gate is off is not counted,
      so turning it on later does not play what was missed (`icon_motion/trigger.rs:1-12`).
- [x] + turns (live `plus-quarter-turn.png`, sidebar + and tab-bar +; group-head + and IconButton share the modifier): a quarter on a click (220 ms, ease-out) and looks the same after it (`turn.rs:quarter`, `IconMotion::of(Plus)`).
- [x] Refresh (live `refresh-spin.png`, Changes band; reset shares `spin(clockwise: false)`, not driven separately) and reset arrows: arrows turn once round, 520 ms, clockwise and counter-clockwise (`turn.rs:spin`).
- [x] Copy, test only (`testAStartPlays…` plays 0.6 s in and ends on the copy icon); the copy buttons in a diff note are hover-only: MANUAL step 4: the front sheet fills (to 12 %), lets go as the check grows in (22 %), the check holds to 86 %, copy returns; 1.4 s (`copied.rs`).
- [x] Split right / Split down fill the half the new pane takes, growing from the divider (decelerating to 45 %) then letting go; 480 ms
      (`pane_fill.rs`, `window/title/mod.rs:333`); the title row's splits are Lucide columns-2 and rows-2 so the fill lies on the frame.
- [m] An icon swap, built (IconSwap transition) and checked at rest only by test; MANUAL step 6: (the pane zoom button's corners) shrinks the old icon to 0.6 and grows the new one, 240 ms, by the change of icon, so the
      keys show it too (`swap.rs`, `window/panes/render.rs:296`).
- [m] A menu's chevron: rotation checked by test (`testAChevronTurnedOverIsTheOtherChevron`); the open state comes from AppKit menu tracking, which an injected click never opens: MANUAL step 7: points up while its menu is open and turns over with it, ease-out opening and ease-in closing (`turn.rs:chevron`,
      `buttons.rs:317`); Animate icons off flips it without the turn.
- [x] The right panel's tab glyph (every one of the five plays and rests on the plain icon in `testEachTabGlyphPlaysThenRestsOnItsIcon`; live: history, notes and changes frames in `tab-glyphs-*.png`; the gauge's tab (Usage) is not in this instance's bar, MANUAL step 8) plays once as its tab becomes the active one, never when the window opens or the panel comes back
      (`tab_glyph/mod.rs`, `controls.rs:94`): git-compare's two circles fill in turn (600 ms), the commit's dot fills as its lines draw out
      (480 ms), the gauge needle sweeps up with a 6 % overshoot (520 ms), the page's lines are written one after the other (480 ms), the folder
      flashes full (420 ms); each ends on the plain icon.
- [x] Every picture at rest is the plain icon (the animation's first and last frame).
- [x] The panel toggles' strip fill (`strip.rs`) and Settings' copy check: already in Swift, left as they are (`WindowBody.swift:57`, `CopyButton.swift`).

## What GPUI does: Increase Contrast outlines (`outlined_always`, line `window/title/mod.rs:151`)

- [x] Choice cards (live `theme-cards-ic.png`) (theme cards of Settings): `card.edge` outline on every card, the chosen one's in `primary` (`choice_card.rs:39-40`); the
      onboarding answers and theme cards already draw a border always (primary when chosen).
- [x] Segmented control: the shared `Segmented` has the outlines (code); test `testSegmentedControlIsOutlinedOnlyUnderIncreaseContrast` (pixels differ with and without); keycap: the track outlined `border.strong`, the active segment too (`controls.rs:271, 384`); keycap `border.strong`
      (`controls.rs:204`, already in Swift).
- [x] Status badge in its own colour (`labels.rs:76`, already in Swift); flat card / setting group `card.edge` (`labels.rs:218`, `setting_group.rs:55`).
- [x] The diff preview (live `git-preview-ic.png`) card (Settings, Git) `card.edge` (`features/git/diff/preview.rs:72`).
- [x] The title row's group colour (live `title-line-ic.png`; test `testTheTitleLineIsSeenAgainstTheWindowInEveryTheme`: 3:1 against the window in every bundled theme, normal and high-contrast tokens) is a 2 pt line under the row's bar instead of a tint (`window/title/mod.rs:151`, `ProjectColor::line`: kept in
      OKLab L 0.30 to 0.88 then moved to 3:1 against `surface.window`).
- [x] Not part of GPUI's rule, so not done: section labels have no outline in GPUI (`labels.rs` has none on `section_label`).
