# Accessibility

Wormhole should work for everyone who uses a Mac to write software, with or without a mouse, with
VoiceOver, and with the display settings macOS offers. A barrier is a bug, and it is fixed like one.

## What works

On macOS 14 or later:

- **VoiceOver.** The sidebar, the panel, the status bar and every control are labelled in words: a
  terminal row reads as one sentence ("sidebar polish, working, Editing index.ts…, selected, 1 of 3,
  Command 1"), counts are spoken instead of shown as glyphs, and a narrow window drops words on
  screen but never in speech. When an agent needs you or finishes, Wormhole says so even when you are
  looking elsewhere; working, idle and the bell are never announced.
- **The keyboard alone.** Everything in a window is reachable without the mouse: F6 and ⇧F6 move
  between areas (from the terminal too), ⇥ walks the stops, lists use Finder's keys and type-ahead,
  and Esc steps back one layer at a time to the terminal. The focus ring shows only for keyboard
  focus. With macOS Keyboard navigation on, the title row's buttons are stops too.
- **Reduce Motion.** No spinners, halos, sliding panels, animated menu bar icon or blinking cursor,
  even when a program asks for one.
- **Reduce Transparency and Increase Contrast.** Every window turns opaque; Increase Contrast also
  switches to the high-contrast variant of the theme (stronger borders, tints and a thicker focus
  ring) without changing your settings.
- **Text size.** Terminal, code and interface sizes are separate settings; ⌘+ and ⌘- change the
  terminal's text in a window. The interface size scales the whole interface.
- **Color.** Every color comes from the theme's named tokens, and the Theme Studio shows the contrast
  of the pairs that matter.

The full description, key by key: [getwormhole.app/docs/accessibility](https://getwormhole.app/docs/accessibility).

## Known limitations

Most of these come from GPUI, the toolkit Wormhole is built on, and need a change there first:

- **The terminal's text is not exposed to VoiceOver.** VoiceOver reads the sidebar, the panel, the
  controls and the announcements, but not the screen of the terminal itself.
- **Announcements are not live regions.** VoiceOver can't navigate to where an announcement came from.
- **Disabled controls read like enabled ones.** A setting a project has fixed is greyed out on screen
  but not marked as disabled for VoiceOver.
- **Reduce Motion cuts instead of fading.** Menus and the command palette appear at once rather than
  with a short fade.
- **Contrast over a translucent window is estimated.** It is judged against a stand-in for the blur,
  not the real desktop behind it; set the opacity to 100 percent for the most readable text.

## Reporting a barrier

[Open an accessibility issue](https://github.com/wormhole-terminal/wormhole/issues/new?template=accessibility.yml):
say what you tried to do, what got in the way, and what you use (VoiceOver, Full Keyboard Access,
Increase Contrast, a text size, another assistive technology). Barriers are labelled `accessibility`
and treated as bugs, not feature requests.
