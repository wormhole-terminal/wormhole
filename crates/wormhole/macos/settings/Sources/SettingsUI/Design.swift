import SwiftUI

/// The one place for spacing, type, shape and colour. Views use these names, never raw numbers.

/// 4-pt spacing grid.
enum Space {
    static let s1: CGFloat = 4, s2: CGFloat = 8, s3: CGFloat = 12, s4: CGFloat = 16, s6: CGFloat = 24, s8: CGFloat = 32
    /// A row's inner gutter and its two heights.
    static let rowInset: CGFloat = 14, rowHeight: CGFloat = 40, tileRowHeight: CGFloat = 56
    static let sidebarWidth: CGFloat = 240, contentWidth: CGFloat = 600, pageInset: CGFloat = 30
}

enum Radius {
    static let chip: CGFloat = 5, control: CGFloat = 6, tile: CGFloat = 7, window: CGFloat = 8, card: CGFloat = 10, banner: CGFloat = 14
}

/// The icon tiles of the sidebar and of tile rows: one side, one corner, the symbol inside an even inset.
enum TileSize {
    static let side: CGFloat = 24, radius: CGFloat = 6, symbol: CGFloat = 13, letter: CGFloat = 12, inset: CGFloat = 5
}

enum TextRole {
    case micro, caption, body, heading, title
    var size: CGFloat { switch self { case .micro: 10; case .caption: 11; case .body: 12; case .heading: 13; case .title: 22 } }
}

extension View {
    func role(_ r: TextRole, weight: Font.Weight = .regular, mono: Bool = false) -> some View {
        font(.system(size: r.size, weight: weight, design: mono ? .monospaced : .default))
    }
}

/// Colours from the design's tokens, dark and light.
struct Tok {
    let dark: Bool
    var accentOverride: Color? = nil
    func c(_ d: UInt32, _ l: UInt32) -> Color { Color(hex: dark ? d : l) }
    var window: Color { c(0x08131e, 0xf7f9fb) }
    var sidebar: Color { c(0x071019, 0xedeff1) }
    var panel: Color { c(0x071019, 0xedeff1) }
    var titlebar: Color { c(0x071019, 0xedeff1) }
    var elevated: Color { c(0x17222d, 0xfcfdfd) }
    var text: Color { c(0xe3ebf2, 0x13202d) }
    var muted: Color { c(0x969fa8, 0x5e6770) }
    var faint: Color { c(0x6b747d, 0x7c838a) }
    var primary: Color { accentOverride ?? c(0x0096ff, 0x006fbf) }
    var onPrimary: Color { c(0x000000, 0xffffff) }
    var button: Color { c(0x1e2933, 0xe9ecef) }
    var buttonHover: Color { c(0x2b3640, 0xdee1e4) }
    var input: Color { c(0x060f17, 0xfdfdfe) }
    var border: Color { dark ? Color(hex: 0xe3ebf2).opacity(0.12) : Color(hex: 0x13202d).opacity(0.14) }
    var hover: Color { dark ? Color(hex: 0xe3ebf2).opacity(0.08) : Color(hex: 0x13202d).opacity(0.08) }
    var danger: Color { c(0xff5f6d, 0xbd1735) }
    var warning: Color { c(0xffc857, 0x805900) }
    var removed: Color { c(0xff5f6d, 0xbd1735) }
    var shadow: Color { dark ? .black.opacity(0.5) : .black.opacity(0.16) }
}

/// Status colours that are the same in both themes.
enum Palette {
    static let green = Color(hex: 0x30d158), yellow = Color(hex: 0xffd60a), claude = Color(hex: 0xd97757), indigo = Color(hex: 0x5e5ce6)
    static let trafficRed = Color(hex: 0xff5f57), trafficYellow = Color(hex: 0xfebc2e), trafficGreen = Color(hex: 0x28c840)
    static let chipGood = Color(hex: 0x1f7a35), chipWarn = Color(hex: 0xa35200), chipBad = Color(hex: 0xc42b20), chipQuiet = Color(hex: 0x545b68)
    static let tileDark = Color(hex: 0x2c2c2e), tileSlate = Color(hex: 0x3b4252), added = Color(hex: 0x4fd6a0)
}

extension Color {
    init(hex: UInt32) {
        self.init(.sRGB, red: Double((hex >> 16) & 255) / 255, green: Double((hex >> 8) & 255) / 255, blue: Double(hex & 255) / 255)
    }
    init(css: String) { self.init(hex: UInt32(css.dropFirst(), radix: 16) ?? 0x3b4252) }
}

private struct TokKey: EnvironmentKey { static let defaultValue = Tok(dark: true) }
extension EnvironmentValues { var tok: Tok { get { self[TokKey.self] } set { self[TokKey.self] = newValue } } }
