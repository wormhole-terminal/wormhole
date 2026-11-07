// What every terminal is drawn with, kept once: the fonts Settings chose and the colours of the theme in force. A terminal
// made later takes them from here as it starts, and a change reaches the ones that exist (`TerminalViews.forEach`), so
// no terminal ever shows an earlier theme or font.

import DesignSystem
import Observation
import TerminalCore
import TerminalMetal
import WindowUI

@MainActor
final class TerminalLook {
    private(set) var fonts: TerminalFontSettings?
    private(set) var palette: TerminalCore.TerminalPalette?
    private let terminals: TerminalViews
    private let themes: ThemeStore

    init(terminals: TerminalViews, themes: ThemeStore = .shared) {
        self.terminals = terminals
        self.themes = themes
    }

    /// Fonts from Settings. Nothing is redrawn when they are the ones in use.
    func use(_ fonts: TerminalFontSettings) {
        guard fonts != self.fonts else { return }
        self.fonts = fonts
        terminals.forEach { $0.setFonts(fonts) }
    }

    /// Takes the theme's colours now and again whenever the theme changes (a pick, the appearance, the Studio's preview).
    func followTheme() {
        let tokens = withObservationTracking { themes.tokens } onChange: { [weak self] in
            Task { @MainActor in self?.followTheme() }
        }
        let next = PaletteMaker.palette(for: tokens)
        guard next != palette else { return }
        palette = next
        terminals.forEach { $0.setPalette(next) }
    }

    /// A terminal that was just made.
    func dress(_ view: MetalTerminalView) {
        if let fonts { view.setFonts(fonts) }
        if let palette { view.setPalette(palette) }
    }
}
