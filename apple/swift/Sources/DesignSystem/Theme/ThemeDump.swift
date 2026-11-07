// The shape of `themes.json`, as `apple/ffi/examples/dump_themes.rs` writes it.

struct Dump: Decodable {
    let format: Int
    let defaultDark: String
    let defaultLight: String
    let catalog: [[String]]
    let themes: [DumpTheme]
}

struct DumpTheme: Decodable {
    let id: String
    let name: String
    let family: [String]?
    let appearance: ThemeAppearance
    let terminal: TerminalPalette
    let normal: DumpVariant
    let highContrast: DumpVariant

    var theme: Theme {
        Theme(
            id: id, name: name, family: family.map { (name: $0[0], version: $0[1]) }, appearance: appearance,
            normal: normal.tokens(terminal), highContrast: highContrast.tokens(terminal))
    }
}

struct DumpVariant: Decodable {
    let ui: [String: RGBA]
    let syntax: [String: SyntaxStyle]
    let metrics: ThemeMetrics

    func tokens(_ terminal: TerminalPalette) -> ThemeTokens {
        ThemeTokens(terminal: terminal, metrics: metrics, ui: ui, syntax: syntax)
    }
}
