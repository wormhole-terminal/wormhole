import SwiftUI
import AppKit

/// A terminal theme as the previews draw it: background, foreground, ANSI blue, green, yellow, red, magenta.
struct ThemePalette {
    let bg, fg, blue, green, yellow, red, magenta: Color
    init(_ hex: [UInt32]) {
        bg = Color(hex: hex[0]); fg = Color(hex: hex[1]); blue = Color(hex: hex[2]); green = Color(hex: hex[3])
        yellow = Color(hex: hex[4]); red = Color(hex: hex[5]); magenta = Color(hex: hex[6])
    }
}

extension Color {
    var luminance: Double {
        let c = NSColor(self).usingColorSpace(.sRGB) ?? .black
        return 0.299 * c.redComponent + 0.587 * c.greenComponent + 0.114 * c.blueComponent
    }
}

/// The themes the pickers and previews draw, by theme id: palettes, the pairs with a dark and a light half, and the
/// lists for choosing the two separately. A host hands over its own; `standard` is for a window with no host.
public struct ThemeData {
    public struct Pair { public var name, dark, light: String
        public init(name: String, dark: String, light: String) { self.name = name; self.dark = dark; self.light = light } }
    public var palettes: [String: [UInt32]]
    public var names: [String: String] = [:]
    public var pairs: [Pair]
    public var darks: [String]
    public var lights: [String]
    public init(palettes: [String: [UInt32]], pairs: [Pair], darks: [String], lights: [String]) {
        self.palettes = palettes; self.pairs = pairs; self.darks = darks; self.lights = lights
    }

    static let table: [String: [UInt32]] = [
    "Wormhole / Dark": [0x08131e, 0xe3ebf2, 0x2fa3ff, 0x4fd6a0, 0xffc857, 0xff5f6d, 0xb78cff],
    "Wormhole / Light": [0xf7f9fb, 0x13202d, 0x0062c4, 0x1b7046, 0x8a5a00, 0xc4213a, 0x7a3cc4],
    "Crimson / Dark": [0x140709, 0xf2e4e6, 0x8aa2ff, 0x7fd99a, 0xf5b35c, 0xff4d5e, 0xff6fae],
    "Crimson / Light": [0xfbf4f4, 0x2a1418, 0x2a55b8, 0x2a7030, 0x835c00, 0xb3122e, 0x9a2f7a],
    "Soltari / Dark": [0x15131c, 0xece6d8, 0x7fa8ff, 0x7fd88f, 0xf2c14e, 0xff6b7f, 0xd39bff],
    "Soltari / Light": [0xf6f2ea, 0x2b2a35, 0x2459c2, 0x2a7030, 0x835c00, 0xb8243c, 0x8e3bb3],
    "Wormhole Hot Pink / Dark": [0x160b10, 0xede3e7, 0x2fa3ff, 0x4fd6a0, 0xffc857, 0xff5f6d, 0xb78cff],
    "Wormhole Hot Pink / Light": [0xfbf7f8, 0x2b121d, 0x0062c4, 0x1b7046, 0x8a5a00, 0xc4213a, 0x7a3cc4],
    "Wormhole Volt / Dark": [0x14160b, 0xebede3, 0x2fa3ff, 0x4fd6a0, 0xffc857, 0xff5f6d, 0xb78cff],
    "Wormhole Volt / Light": [0xfafbf7, 0x262b12, 0x0062c4, 0x1b7046, 0x8a5a00, 0xc4213a, 0x7a3cc4],
    "Wormhole White / Dark": [0x111111, 0xe8e8e8, 0x2fa3ff, 0x4fd6a0, 0xffc857, 0xff5f6d, 0xb78cff],
    "Wormhole White / Light": [0xf9f9f9, 0x1f1f1f, 0x0062c4, 0x1b7046, 0x8a5a00, 0xc4213a, 0x7a3cc4],
    "Catppuccin Mocha": [0x1e1e2e, 0xcdd6f4, 0x89b4fa, 0xa6e3a1, 0xf9e2af, 0xf38ba8, 0xcba6f7],
    "Catppuccin Latte": [0xeff1f5, 0x4c4f69, 0x1e66f5, 0x40a02b, 0xdf8e1d, 0xd20f39, 0x8839ef],
    "GitHub Dark": [0x0d1117, 0xe6edf3, 0x58a6ff, 0x3fb950, 0xd29922, 0xff7b72, 0xbc8cff],
    "GitHub Light": [0xffffff, 0x1f2328, 0x0969da, 0x1a7f37, 0x9a6700, 0xcf222e, 0x8250df],
    "Tokyo Night": [0x1a1b26, 0xc0caf5, 0x7aa2f7, 0x9ece6a, 0xe0af68, 0xf7768e, 0xbb9af7],
    "Dracula": [0x282a36, 0xf8f8f2, 0xbd93f9, 0x50fa7b, 0xf1fa8c, 0xff5555, 0xff79c6],
]
    static let pairTable: [(String, String, String)] = [
    ("Wormhole", "Wormhole / Dark", "Wormhole / Light"), ("Catppuccin", "Catppuccin Mocha", "Catppuccin Latte"),
    ("GitHub", "GitHub Dark", "GitHub Light"), ("Crimson", "Crimson / Dark", "Crimson / Light"),
    ("Soltari", "Soltari / Dark", "Soltari / Light"), ("Hot Pink", "Wormhole Hot Pink / Dark", "Wormhole Hot Pink / Light"),
    ("Wormhole Volt", "Wormhole Volt / Dark", "Wormhole Volt / Light"), ("Wormhole White", "Wormhole White / Dark", "Wormhole White / Light"),
]
    static let darkList = ["Wormhole / Dark", "Tokyo Night", "Catppuccin Mocha", "Dracula"]
    static let lightList = ["Wormhole / Light", "Catppuccin Latte", "GitHub Light", "Soltari / Light"]
    public static let standard = ThemeData(palettes: table, pairs: pairTable.map { Pair(name: $0.0, dark: $0.1, light: $0.2) },
                                           darks: darkList, lights: lightList)
}

func shortThemeName(_ k: String) -> String { k.replacingOccurrences(of: " / Dark", with: "").replacingOccurrences(of: " / Light", with: "") }

let accentColours: [(String, UInt32)] = [("Blue", 0x0a84ff), ("Purple", 0xbf5af2), ("Pink", 0xff375f), ("Red", 0xff453a),
                                         ("Orange", 0xff9f0a), ("Yellow", 0xffd60a), ("Green", 0x30d158), ("Graphite", 0x8e8e93)]

/// Font menu entries: name, monospace.
let fontChoices: [(String, Bool)] = [("JetBrains Mono", true), ("SF Mono", true), ("Menlo", true), ("Monaco", true), ("Fira Code", true),
    ("Iosevka", true), ("SF Pro", false), ("Helvetica Neue", false), ("Avenir Next", false), ("New York", false)]

/// The font to draw a family in: the installed one, else what the board's CSS stack falls back to.
func specimenFont(_ name: String, size: CGFloat) -> Font {
    switch name {
    case "SF Pro": return .system(size: size)
    case "SF Mono": return .system(size: size, design: .monospaced)
    case "New York": return .system(size: size, design: .serif)
    default:
        if NSFont(name: name, size: size) != nil { return .custom(name, size: size) }
        return fontChoices.first { $0.0 == name }?.1 == false ? .system(size: size) : .system(size: size, design: .monospaced)
    }
}
/// The terminal's own face in the mocks: JetBrains Mono if installed, else SF Mono.
func terminalFont(_ size: CGFloat) -> Font { specimenFont("JetBrains Mono", size: size) }

extension SettingsModel {
    var themeRowKey: String { "Appearance:0:2" }
    var pairName: String { let v = value(themeRowKey); return v.isEmpty ? (themeData.pairs.first?.name ?? "") : v }
    func themeChoice(light: Bool) -> String {
        let v = value(themeRowKey + (light ? "#light" : "#dark"))
        return v.isEmpty ? (light ? themeData.lights.first : themeData.darks.first) ?? "" : v
    }
    var separateThemes: Bool { str("Appearance", 0, 1) != "One theme for both" }

    /// The window's own light or dark: Mode, else what macOS says.
    func isLight(system: ColorScheme) -> Bool {
        switch str("Appearance", 0, 0) { case "Dark": false; case "Light": true; default: system == .light }
    }
    var colourScheme: ColorScheme? { switch str("Appearance", 0, 0) { case "Dark": .dark; case "Light": .light; default: nil } }

    /// The theme the previews use for this mode.
    func theme(light: Bool) -> ThemePalette {
        let key = separateThemes ? themeChoice(light: light)
            : { let p = themeData.pairs.first { $0.name == pairName } ?? themeData.pairs[0]; return light ? p.light : p.dark }()
        return themePalette(key)
    }

    func themePalette(_ key: String) -> ThemePalette {
        let t = themeData.palettes
        return ThemePalette(t[key] ?? t["Wormhole / Dark"] ?? t.values.first ?? [0x111111, 0xeeeeee, 0x2fa3ff, 0x4fd6a0, 0xffc857, 0xff5f6d, 0xb78cff])
    }

    func customColour(_ key: String) -> Color {
        let v = value(key + "#custom"); return v.isEmpty ? Color(hex: 0xb05cf2) : Color(css: v)
    }
    /// A swatch row's colour: nil for Theme.
    func swatch(_ key: String) -> Color? {
        let n = value(key)
        if n == "Custom" { return customColour(key) }
        return accentColours.first { $0.0 == n }.map { Color(hex: $0.1) }
    }
    var accent: Color? { swatch("Appearance:1:0") }
}
