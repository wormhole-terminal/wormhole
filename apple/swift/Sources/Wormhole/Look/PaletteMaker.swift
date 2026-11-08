// The terminal's colours from the live theme: the grid's 16 colours, foreground, background, cursor and selection are the
// theme's terminal palette, and find's matches come from the `find.match*` tokens and a Probe's tones from `text.link`, `status.*.text` and `text.muted` (`Palette::find_match` in the GPUI
// app). One function, so the theme in force is the one every terminal draws with.

import DesignSystem
import TerminalCore

enum PaletteMaker {
    static func palette(for tokens: ThemeTokens) -> TerminalCore.TerminalPalette {
        let theme = tokens.terminal
        func rgb(_ colour: RGBA) -> SIMD3<UInt8> {
            func byte(_ value: Double) -> UInt8 { UInt8((min(max(value, 0), 1) * 255).rounded()) }
            return SIMD3(byte(colour.red), byte(colour.green), byte(colour.blue))
        }
        var find: TerminalCore.TerminalPalette.FindColors?
        if let match = tokens.rgba(named: "find.match"), let current = tokens.rgba(named: "find.match.current"),
            let text = tokens.rgba(named: "find.match.current.text")
        {
            find = .init(match: rgb(match), matchAlpha: Float(match.alpha), current: rgb(current), currentText: rgb(text))
        }
        let tones = ProbeTone.allCases.compactMap { tokens.rgba(named: $0.token).map(rgb) }
        return TerminalCore.TerminalPalette(
            ansi: theme.ansi.map(rgb), foreground: rgb(theme.foreground), background: rgb(theme.background),
            cursor: rgb(theme.cursor), selection: rgb(theme.selectionBackground),
            selectionText: theme.selectionForeground.map(rgb), find: find,
            tones: tones.count == ProbeTone.allCases.count ? tones : nil)
    }
}
