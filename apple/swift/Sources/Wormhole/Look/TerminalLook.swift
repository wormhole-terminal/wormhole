// What every terminal is drawn with, kept once: the fonts Settings chose and the colours of the theme in force. A terminal
// made later takes them from here as it starts, and a change reaches the ones that exist (`TerminalViews.forEach`, in every window), so
// no terminal ever shows an earlier theme or font. The bell is the same: which parts of it happen (Settings) and the
// colour of its glow (`agent.attention`, as the GPUI app's `bell_edge`).

import AppKit
import DesignSystem
import Observation
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
    /// The terminals of every window, asked for when a change is to reach them.
    private let terminals: () -> [TerminalViews]
    private let themes: ThemeStore

    init(terminals: @escaping () -> [TerminalViews], themes: ThemeStore = .shared) {
        self.terminals = terminals
        self.themes = themes
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

    /// Takes the theme's colours now and again whenever the theme changes (a pick, the appearance, the Studio's preview).
    func followTheme() {
        let tokens = withObservationTracking { themes.tokens } onChange: { [weak self] in
            Task { @MainActor in self?.followTheme() }
        }
        let glow = NSColor(tokens.agentAttention)
        if glow != bellColor {
            bellColor = glow
            terminals().forEach { $0.forEach { $0.bellColor = glow } }
        }
        let next = PaletteMaker.palette(for: tokens)
        guard next != palette else { return }
        palette = next
        terminals().forEach { $0.forEach { $0.setPalette(next) } }
    }

    /// A terminal that was just made.
    func dress(_ view: MetalTerminalView) {
        if let fonts { view.setFonts(fonts) }
        if let palette { view.setPalette(palette) }
        view.bell = bell
        view.bellColor = bellColor
    }
}
