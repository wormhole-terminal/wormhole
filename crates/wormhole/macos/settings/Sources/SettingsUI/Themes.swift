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
/// lists for choosing the two separately. The app hands them over; empty until it has.
struct ThemeData: Equatable {
    struct Pair: Equatable { var name, dark, light: String }
    var palettes: [String: [UInt32]] = [:]
    var names: [String: String] = [:]
    var pairs: [Pair] = []
    var darks: [String] = []
    var lights: [String] = []
}

func shortThemeName(_ k: String) -> String { k.replacingOccurrences(of: " / Dark", with: "").replacingOccurrences(of: " / Light", with: "") }

let accentColours: [(String, UInt32)] = [("Blue", 0x0a84ff), ("Purple", 0xbf5af2), ("Pink", 0xff375f), ("Red", 0xff453a),
                                         ("Orange", 0xff9f0a), ("Yellow", 0xffd60a), ("Green", 0x30d158), ("Graphite", 0x8e8e93)]

/// The font to draw a family in: the installed one, else the system's monospaced face.
func specimenFont(_ name: String, size: CGFloat) -> Font {
    switch name {
    case "SF Pro", ".SystemUIFont": return .system(size: size)
    case "SF Mono": return .system(size: size, design: .monospaced)
    case "New York": return .system(size: size, design: .serif)
    default:
        return NSFont(name: name, size: size) != nil ? .custom(name, size: size) : .system(size: size, design: .monospaced)
    }
}

extension SettingsModel {
    var themeRowKey: String { "Appearance:0:2" }
    var pairName: String { let v = value(themeRowKey); return v.isEmpty ? (themeData.pairs.first?.name ?? "") : v }
    func themeChoice(light: Bool) -> String {
        let v = value(themeRowKey + (light ? "#light" : "#dark"))
        return v.isEmpty ? (light ? themeData.lights.first : themeData.darks.first) ?? "" : v
    }
    var separateThemes: Bool { str("Appearance", 0, 1) != "pairing_both" }

    var colourScheme: ColorScheme? { switch str("Appearance", 0, 0) { case "settings_appearance_dark": .dark; case "settings_appearance_light": .light; default: nil } }

    /// The theme the previews use for this mode.
    func theme(light: Bool) -> ThemePalette {
        if let real = preview?.side(light: light).palette { return ThemePalette(real) }
        let pair = themeData.pairs.first { $0.name == pairName } ?? themeData.pairs.first
        let key = separateThemes ? themeChoice(light: light) : pair.map { light ? $0.light : $0.dark } ?? ""
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
        if n == "custom" { return customColour(key) }
        return accentColours.first { snakeID($0.0) == n }.map { Color(hex: $0.1) }
    }
    var accent: Color? { swatch("Appearance:1:0") }
}
