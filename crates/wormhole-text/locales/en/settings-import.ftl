# Settings window, Import page: bringing the font, colours, shell and more over from another terminal, what
# doesn't carry over, and how each terminal compares with { -brand }. Also the words of the first-run step that
# does the same in one click. The comparisons' content is checked against primary sources (see
# import/compare.rs in core, which holds the ids, links and dates).

settings-section-import = Import
    .intro = Bring your font, colours, shell and more over from the terminal you used before. Nothing changes until you choose what to import, and the other terminal's own files are never touched.

## Found on this Mac

settings-import-group-found = Found on this Mac

settings-import-found = Terminals found
    .keywords = import migrate switch move settings config configuration ghostty iterm iterm2 terminal.app alacritty kitty wezterm warp zed vscode vs code font theme colours colors shell

settings-import-scan-again = Scan again
settings-import-scanning = Looking for terminals…
settings-import-none = No other terminal's settings found
settings-import-none-detail = { -brand } looked for Ghostty, iTerm2, Terminal.app, Alacritty, kitty, WezTerm, Warp, Zed and VS Code in the places they keep their settings, and found none.

settings-import-files-read = Files read

# { $count } is a whole number.
settings-import-count = { $count ->
    [one] { $count } setting
   *[other] { $count } settings
}

# { $terminal } is a terminal's name, { $count } how many settings it has ("13 settings"). What a screen reader
# says for the card's header, which opens and closes the card.
settings-import-card-label = { $terminal }, { $count }

settings-import-take = Brings over
settings-import-leave = Doesn't carry over
settings-import-nothing-to-take = Nothing in these settings can be brought over.
settings-import-runs-everywhere = { -brand } would run this in every new terminal, so it isn't ticked until you tick it.

# { $selected } of { $total } rows are ticked.
settings-import-selected = { $selected } of { $total } selected
settings-import-apply = Import selected

# { $brought } is how many settings were brought over and { $left } how many didn't carry over: both whole numbers.
settings-import-summary = { $brought ->
    [one] { $brought } setting brought over, { $left } didn't
   *[other] { $brought } settings brought over, { $left } didn't
}

# { $error } is what went wrong when saving the colours.
settings-import-failed = Couldn't import: { $error }

## The terminals

settings-import-terminal-ghostty = Ghostty
settings-import-terminal-iterm2 = iTerm2
settings-import-terminal-terminal-app = Terminal.app
settings-import-terminal-alacritty = Alacritty
settings-import-terminal-kitty = kitty
settings-import-terminal-wezterm = WezTerm
settings-import-terminal-warp = Warp
settings-import-terminal-zed = Zed
settings-import-terminal-vscode = VS Code
settings-import-terminal-wormhole = { -brand }

## What a setting is called

settings-import-setting-font-family = Font
settings-import-setting-font-size = Font size
settings-import-setting-line-height = Line height
settings-import-setting-ligatures = Ligatures
settings-import-setting-theme = Colours
settings-import-setting-theme-dark = Colours, dark mode
settings-import-setting-theme-light = Colours, light mode
settings-import-setting-opacity = Opacity
settings-import-setting-blur = Background blur
settings-import-setting-option-as-meta = Option as Meta
settings-import-setting-scrollback = Scrollback lines
settings-import-setting-bell-sound = Bell sound
settings-import-setting-bell-visual = Visual bell
settings-import-setting-default-shell = Shell
settings-import-setting-shell-profile = Shell command

## What it would be set to

settings-import-value-on = On
settings-import-value-off = Off
settings-import-value-meta-left = Left Option key
settings-import-value-meta-right = Right Option key
settings-import-value-meta-both = Both Option keys

## Why a setting doesn't carry over

settings-import-reason-cursor-shape = { -brand } has no setting for the cursor's shape yet.
settings-import-reason-cursor-blink = { -brand } has no setting for a blinking cursor yet.
settings-import-reason-padding = { -brand } has no setting for the space around the text yet.
settings-import-reason-copy-on-select = { -brand } has no copy-on-select setting yet.
settings-import-reason-confirm-close = { -brand } has no setting for confirming before a terminal closes.
settings-import-reason-working-directory = { -brand } has no setting for the folder a new terminal starts in.
settings-import-reason-keybinding = { -brand } has no file for your own key bindings yet.
settings-import-reason-environment = Environment variables are set per profile in Settings, not imported.
settings-import-reason-font-variant = { -brand } has no separate bold or italic fonts, or a second font for other text.
settings-import-reason-cell-width = { -brand } has no setting for letter spacing or cell width.
settings-import-reason-other = { -brand } has no such setting.
settings-import-reason-unsupported-value = { -brand } can't show this value.
settings-import-reason-dynamic-lua = It's computed in Lua, which { -brand } never runs.
settings-import-reason-executes = It runs a program, which an import never does.
settings-import-reason-unreadable = The file couldn't be read: it's missing, too big or not valid.
settings-import-reason-include-cycle = A file that includes itself, so it was read once.
settings-import-reason-include-limit = Files include each other too deep, or there are too many to read.
settings-import-reason-old-format = A settings file in a format that isn't read any more.
settings-import-reason-outside-home = It's outside your home folder, or a link leads out of it, so it wasn't read.
settings-import-reason-partial-palette = It sets only some colours; a theme needs at least a background and a foreground.

# { $font } is the font's name as the other terminal wrote it.
settings-import-reason-font-not-installed = The font { $font } isn't installed on this Mac.

# { $theme } is the name of the theme the other terminal ships inside its app.
settings-import-reason-built-in-theme = { $theme } is built into the other terminal, so its colours aren't in any file.

## Compared with { -brand }

settings-import-group-compare = Compared with { -brand }

settings-import-compare = How they compare
    .keywords = drawbacks limits strengths differences versus comparison alternatives

settings-import-compare-none = When another terminal's settings are found, you see here how it compares with { -brand }.

# { $terminal } is a terminal's name.
settings-import-compare-gaps = Where { $terminal } falls short today
settings-import-wormhole-gaps = What { -brand } doesn't do yet

# { $date } is the day a fact was checked (2026-10-26).
settings-import-as-of = As of { $date }
settings-import-source = Source
# { $number } counts a drawback's sources from 1 when it has more than one.
settings-import-source-numbered = Source { $number }

## First run

# { $terminal } is the name of the terminal whose settings were found.
settings-import-use-mine = Use my { $terminal } settings

# { $brought } and { $left } are whole numbers: the settings brought over and those that didn't carry over.
settings-import-onboarding-summary = { $brought ->
    [one] { $brought } setting brought over, { $left } didn't: Settings → Import lists them
   *[other] { $brought } settings brought over, { $left } didn't: Settings → Import lists them
}

# { $brought } is a whole number.
settings-import-onboarding-all = { $brought ->
    [one] { $brought } setting brought over
   *[other] { $brought } settings brought over
}

## ghostty

settings-import-strength-ghostty-native-tabs-graphics = Native macOS app with native tabs and splits, kitty graphics and keyboard protocols, ligatures.

settings-import-gap-ghostty-no-session-persistence = Running programs end when the app quits; window layout can be restored.
settings-import-gap-ghostty-agent-status-not-shown = Its library reads program status (OSC 7501); the app doesn't show it yet.
settings-import-gap-ghostty-command-notify-off = Notifications when a command finishes are off by default.
settings-import-gap-ghostty-no-sixel = No Sixel images; the kitty graphics protocol is offered instead.
settings-import-gap-ghostty-config-file-only = Settings live in a text file; a settings window is planned.

## iterm2

settings-import-strength-iterm2-broadest-features = The broadest macOS feature set: tmux integration, all image protocols, agent Session Status.

settings-import-gap-iterm2-quit-ends-programs = Programs survive crashes and updates; quitting with ⌘Q or rebooting ends them.
settings-import-gap-iterm2-ligatures-disable-gpu = Turning on ligatures disables the GPU renderer.
settings-import-gap-iterm2-ai-setup-required = AI features need a plugin, explicit consent and your own API key.

## terminal-app

settings-import-strength-terminal-app-built-into-macos = Built into macOS, fully native, no install; 24-bit colour and Powerline glyphs since macOS 26.

settings-import-gap-terminal-app-no-kitty-keyboard = No kitty keyboard protocol or other modern key reporting, in third-party tests.
settings-import-gap-terminal-app-no-inline-images = No documented image protocol (Sixel, kitty or iTerm2).
settings-import-gap-terminal-app-no-agent-status = No agent or program-status features.

## alacritty

settings-import-strength-alacritty-minimal-fast = Minimal and fast, cross-platform, vi mode, regex hints and the kitty keyboard protocol.

settings-import-gap-alacritty-no-tabs-or-splits = No tabs or splits by design; a multiplexer or window manager is expected.
settings-import-gap-alacritty-no-images = No image protocols; Sixel exists only in a community fork.
settings-import-gap-alacritty-no-ligatures = No font ligatures (requested since 2017).
settings-import-gap-alacritty-config-file-only = No settings window; configuration is a TOML file.
settings-import-gap-alacritty-no-agent-status = No program-status or agent features among its escape sequences.

## kitty

settings-import-strength-kitty-fast-scriptable = Fast and scriptable; the origin of the kitty graphics and keyboard protocols; layouts and splits.

settings-import-gap-kitty-no-session-persistence = Sessions re-create windows; running programs don't survive quitting.
settings-import-gap-kitty-gui-launch-env = Started from the Dock, it often lacks your shell's environment variables.
settings-import-gap-kitty-no-sixel = No Sixel images; kitty's own graphics protocol is used instead.
settings-import-gap-kitty-config-file-only = Configuration is a single text file; no settings window.

## wezterm

settings-import-strength-wezterm-multiplexer-lua = Built-in multiplexer, ligatures, iTerm2 and kitty images, Lua config, every major platform.

settings-import-gap-wezterm-no-release-since-2024 = No tagged release since February 2024; nightly builds continue.
settings-import-gap-wezterm-persistence-needs-setup = Keeping programs running needs a configured multiplexer domain.
settings-import-gap-wezterm-kitty-keyboard-off = The kitty keyboard protocol is off by default.
settings-import-gap-wezterm-config-is-lua = Configuration is Lua code; no settings window.
settings-import-gap-wezterm-macos-26-reports = Open reports of a crash and high GPU use on macOS 26.

## warp

settings-import-strength-warp-agent-focused = Agent-focused: a built-in agent, a Claude Code plugin with tab status, block-based output.

settings-import-gap-warp-programs-end-on-relaunch = Running programs end when Warp relaunches or updates (an open request).
settings-import-gap-warp-telemetry-for-free-ai = On the Free plan, AI features require telemetry to be on.
settings-import-gap-warp-login-for-some-features = The terminal works without an account; some features need signing in.
settings-import-gap-warp-offline-requested = Full offline support is an open request.
settings-import-gap-warp-no-sixel = No Sixel images; kitty graphics and iTerm2 images are supported.

## zed

settings-import-strength-zed-terminal-in-editor = A terminal inside the editor; agents in the Agent Panel can run terminal commands.

settings-import-gap-zed-editor-panel = A panel inside the editor, not a standalone terminal app.
settings-import-gap-zed-no-images = No image protocols in the terminal (closed as not planned).
settings-import-gap-zed-no-kitty-keyboard = No kitty keyboard protocol encoding in its terminal.
settings-import-gap-zed-terminal-ligatures = Ligatures can't be turned on for the terminal (open since 2024).
settings-import-gap-zed-telemetry-on = Crash reports and usage data are sent by default; both can be turned off.

## vscode

settings-import-strength-vscode-terminal-in-editor = A terminal in the editor with tabs, splits and shell integration; kitty keyboard by default.

settings-import-gap-vscode-editor-panel = A panel inside the editor, not a standalone terminal app.
settings-import-gap-vscode-programs-relaunched = After a restart, programs are started again, not kept running.
settings-import-gap-vscode-images-off = Images (Sixel, iTerm2, kitty) are off by default.
settings-import-gap-vscode-scrollback-1000 = Scrollback keeps 1,000 lines by default.
settings-import-gap-vscode-telemetry-on = Telemetry is on unless turned off in settings.

## wormhole

settings-import-strength-wormhole-agent-sidebar = A native macOS terminal with a sidebar of terminals showing each agent's status, and a git diff panel.

settings-import-gap-wormhole-macos-only = macOS only (macOS 14 or later).
settings-import-gap-wormhole-young-project = A young project: first released in October 2026.
settings-import-gap-wormhole-programs-end-on-quit = Running programs end when the app quits; windows and recent output come back.
settings-import-gap-wormhole-voiceover-no-terminal-text = VoiceOver can't read the terminal's text yet; the controls around it are labelled.
settings-import-gap-wormhole-kitty-keyboard-partial = The kitty keyboard protocol's key-release and repeat events aren't sent.
