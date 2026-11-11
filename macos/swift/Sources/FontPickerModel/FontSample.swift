// The text a picker shows a font in: the GPUI specimen's four lines (a prompt, a result, code with the pairs
// ligatures join, the characters that tell fonts apart) and three the terminal needs that GPUI does not try: box
// drawing, powerline and Nerd glyphs, and wide characters. Colours are the theme's terminal ones, by role.

public struct SampleRun: Sendable, Equatable {
    public enum Tone: Sendable, Equatable { case foreground, muted, ansi(Int) }
    public let text: String
    public let tone: Tone
    init(_ text: String, _ tone: Tone = .foreground) { (self.text, self.tone) = (text, tone) }
}

public enum TerminalSample {
    // text: sample (terminal text, never translated)
    public static let lines: [[SampleRun]] = [
        [SampleRun("→ ", .ansi(2)), SampleRun("wormhole ", .ansi(4)), SampleRun("(main) ", .ansi(5)), SampleRun("cargo test")],
        [SampleRun("test result: "), SampleRun("ok", .ansi(2)), SampleRun(". 57 passed; 0 failed")],
        [SampleRun("fn ", .ansi(5)), SampleRun("resolve", .ansi(4)), SampleRun("(&self) -> Result<Tokens>")],
        [SampleRun("{ a != b && c >= d }")],
        [SampleRun("0O 1lI {}[] => != ~ |> <= www //", .muted)],
        [SampleRun("┌──┬──┐ ├──┼──┤ └──┴──┘ ▀▄█░▒▓", .ansi(6))],
        [SampleRun("\u{E0B0} \u{E0A0} main \u{E0B1} \u{F07B} ~/wormhole \u{E0B2}\u{E0B3}", .ansi(4))],
        [SampleRun("日本語 한글 ✓ ➜", .ansi(3))],
    ]

    /// What the interface font sets: the chrome's own kinds of text, then a pangram and the characters that
    /// tell fonts apart.
    // text: sample (interface text, never translated)
    public static let interface: [[SampleRun]] = [
        [SampleRun("Sidebar   Notes   Changes   History")],
        [SampleRun("Wormhole 1.6.3 · 3 terminals · main")],
        [SampleRun("The quick brown fox jumps over the lazy dog")],
        [SampleRun("0123456789   Il1 O0   Aa Bb Cc", .muted)],
    ]
}

/// A kind of glyph a terminal needs, and scalars that stand for it: a font that has them draws them itself, any
/// other leaves them to the system's fallback (which may not line up with the cell grid).
public struct GlyphGroup: Identifiable, Sendable, Equatable {
    public let id: String
    public let label: String
    public let scalars: [Unicode.Scalar]

    public static var all: [GlyphGroup] {
        [
            GlyphGroup(id: "box", label: FontWords.boxDrawing, scalars: ["─", "│", "┌", "┐", "└", "┘", "├", "█", "▀"]),
            GlyphGroup(id: "powerline", label: FontWords.powerline, scalars: ["\u{E0A0}", "\u{E0B0}", "\u{E0B1}", "\u{E0B2}"]),
            GlyphGroup(id: "nerd", label: FontWords.nerdIcons, scalars: ["\u{F07B}", "\u{F015}"]),
            GlyphGroup(id: "wide", label: FontWords.wide, scalars: ["日", "本", "한", "글"]),
        ]
    }
}

/// What a family can do for the sample, for the preview's chips.
public struct FontCoverage: Sendable, Equatable {
    public var drawn: [String: Bool]
    public var ligatures: Bool
    public var styles: [String]

    public var hasBold: Bool { styles.contains { $0.localizedCaseInsensitiveContains("bold") } }
    public var hasItalic: Bool {
        styles.contains { $0.localizedCaseInsensitiveContains("italic") || $0.localizedCaseInsensitiveContains("oblique") }
    }
}
