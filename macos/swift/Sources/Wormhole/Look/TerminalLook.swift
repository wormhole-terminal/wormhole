// What every terminal is drawn with, kept once: the fonts Settings chose and the colours of the theme in force. A terminal
// made later takes them from here as it starts, and a change reaches the ones that exist (`TerminalViews.forEach`, in every window), so
// no terminal ever shows an earlier theme or font. The bell is the same: which parts of it happen (Settings) and the
// colour of its glow (`agent.attention`, as the GPUI app's `bell_edge`).

import AppKit
import DesignSystem
import Observation
import PlatformModel
import TerminalCore
import TerminalMetal
import WindowUI

@MainActor
final class TerminalLook {
    private(set) var fonts: TerminalFontSettings?
    private(set) var palette: TerminalCore.TerminalPalette?
    /// The Dock bounce is not the bell's: Settings' "Bounce once when a terminal needs input" is the notifications'.
    private(set) var bell = BellStyle(dock: false)
    private(set) var bellColor = NSColor.systemOrange
    /// The glow's corners are the pane's (`radius_lg`, as `bell_edge`).
    private(set) var bellRadius: CGFloat = 8
    /// The tint over a pane while files are dragged over it (`state.highlight`).
    private(set) var dropTint = NSColor.controlAccentColor.withAlphaComponent(0.2)
    /// What a program may do with the clipboard (Settings → Clipboard): the core's defaults until Settings says.
    private(set) var clipboard: (read: ClipboardReadRule, write: Bool) = (.ask, true)
    /// Which Option key types Meta (Settings → Terminal): none, as the core's default is.
    private(set) var optionAsMeta = OptionAsMeta.off
    /// The terminals of every window, asked for when a change is to reach them.
    private let terminals: () -> [TerminalViews]
    private let themes: ThemeStore
    /// Each terminal's view tells the engine the colours it is drawn in (`MetalTerminalView.setPalette`); the engine also
    /// keeps one palette per profile for a terminal no view has dressed yet, which this look tells for the profile it is
    /// (asked each time: the app's own look is the shown profile's, whichever that is now).
    private let engineProfile: () -> String?

    init(terminals: @escaping () -> [TerminalViews], themes: ThemeStore = .shared, engineProfile: @escaping () -> String?) {
        self.terminals = terminals
        self.themes = themes
        self.engineProfile = engineProfile
    }

    /// Fonts from Settings. Nothing is redrawn when they are the ones in use.
    func use(_ fonts: TerminalFontSettings) {
        guard fonts != self.fonts else { return }
        self.fonts = fonts
        terminals().forEach { $0.forEach { $0.setFonts(fonts) } }
    }

    /// Which parts of the bell happen (Settings → Terminal → Bell).
    func use(bell: BellStyle) {
        guard bell != self.bell else { return }
        self.bell = bell
        terminals().forEach { $0.forEach { $0.bell = bell } }
    }

    /// `option_as_meta` from Settings, which also reads the boolean older config files have (`true` is both).
    func use(optionAsMeta value: OptionAsMeta) {
        guard value != optionAsMeta else { return }
        optionAsMeta = value
        terminals().forEach { $0.forEach { $0.optionAsMeta = value } }
    }

    /// `clipboard_read` and `clipboard_write` from Settings.
    func use(clipboardRead read: ClipboardReadRule, write: Bool) {
        guard (read, write) != clipboard else { return }
        clipboard = (read, write)
        terminals().forEach { $0.forEach { $0.setClipboardRules(read: read, writeAllowed: write) } }
    }

    /// Takes the theme's colours now and again whenever the theme changes (a pick, the appearance, the Studio's preview).
    func followTheme() {
        let tokens = withObservationTracking { themes.tokens } onChange: { [weak self] in
            Task { @MainActor in self?.followTheme() }
        }
        let glow = NSColor(tokens.agentAttention), radius = CGFloat(tokens.metrics.radiusLg)
        if glow != bellColor || radius != bellRadius {
            (bellColor, bellRadius) = (glow, radius)
            terminals().forEach { $0.forEach { ($0.bellColor, $0.bellCornerRadius) = (glow, radius) } }
        }
        if let highlight = tokens.rgba(named: "state.highlight").map({ NSColor(red: $0.red, green: $0.green, blue: $0.blue, alpha: $0.alpha) }), highlight != dropTint {
            dropTint = highlight
            terminals().forEach { $0.forEach { $0.dropTint = highlight } }
        }
        let next = PaletteMaker.palette(for: tokens)
        guard next != palette else { return }
        palette = next
        terminals().forEach { $0.forEach { $0.setPalette(next) } }
        shareEngineColours()
    }

    /// What a terminal of this look's profile starts with, and has until a view dresses it (the views above told the engine
    /// their own). Told again when the shown profile changed: the colours may stay while the profile they are for does not.
    /// From the theme as it is now, not the palette last followed: right after a switch that is still the profile left's.
    func shareEngineColours() {
        #if WITH_UNIFFI
            if palette != nil, let profile = engineProfile() { EngineColors.share(PaletteMaker.palette(for: themes.tokens), profile: profile) }
        #endif
    }

    /// Whether the menus own this key: an entry of the app's scope has it (`keymap::action_for(key, Scope::App)`).
    private static func isAppKey(_ name: String, _ flags: NSEvent.ModifierFlags) -> Bool {
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        let spec = KeySpec(mods, name.lowercased())
        return Keymap.shared.entries.contains { $0.scope == .app && $0.keys.contains { $0.sameKey(spec) } }
    }

    /// The action the table gives a key in the Terminal or Edit scope (`keymap::terminal_action`).
    private static func terminalAction(_ name: String, _ flags: NSEvent.ModifierFlags) -> String? {
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        let spec = KeySpec(mods, name.lowercased())
        return Keymap.shared.entries.first { ($0.scope == .terminal || $0.scope == .edit) && $0.isLive && $0.keys.contains { $0.sameKey(spec) } }?.id.rawValue
    }

    /// A terminal that was just made.
    func dress(_ view: MetalTerminalView) {
        if let fonts { view.setFonts(fonts) }
        if let palette { view.setPalette(palette) }
        view.bell = bell
        view.optionAsMeta = optionAsMeta
        view.isAppKey = Self.isAppKey
        view.terminalAction = Self.terminalAction
        view.dropTint = dropTint
        view.bellColor = bellColor
        view.bellCornerRadius = bellRadius
        view.setClipboardRules(read: clipboard.read, writeAllowed: clipboard.write)
    }
}
