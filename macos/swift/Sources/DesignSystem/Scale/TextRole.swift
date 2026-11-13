// The four type roles of the GPUI app plus its label role (`style/scale.rs` `TextRole`): sizes are rems of
// the interface font (the theme's `ui_font_size`, 13 pt by default, in its `ui_font_family` or the system
// font), so at the default they are 12 / 13 / 14 / 16 / 20. Line height is GPUI's own ratio unless a row fixes it (`NAME_LINE`, `META_LINE`).
//
// A view reads a role through the tokens of its window (`tokens.size(.caption)`, `tokens.font(.body)`, `.textRole(.body)`):
// a window of another profile than the one Wormhole is in has that profile's interface font. The role's own `size` and
// `font` read the app's store, for the windows that are the app's (Settings, the Studio, the first run).

import CoreText
import SwiftUI

public enum TextRole: CaseIterable, Sendable {
    /// Metadata, badges, section labels, second lines.
    case caption
    /// A folder band's name, a section band's title.
    case label
    /// Content: list rows, form labels, buttons, paragraphs.
    case body
    /// A subsection heading, a panel's title.
    case heading
    /// The top of a page.
    case title

    /// The role's size in rems (a rem is the interface font's size scaled 16 / 13, `rem_size`).
    var rems: CGFloat {
        switch self {
        case .caption: 0.75
        case .label: 0.8125
        case .body: 0.875
        case .heading: 1
        case .title: 1.25
        }
    }

    /// Points at the theme's interface font size (13 by default: 12 / 13 / 14 / 16 / 20).
    @MainActor public var size: CGFloat { size(in: ThemeStore.shared.tokens.metrics) }

    func size(in metrics: ThemeMetrics) -> CGFloat { rems * metrics.uiFontSize * 16 / TextRole.defaultInterfaceSize }

    public var weight: Font.Weight {
        switch self {
        case .caption, .body: .regular
        case .label, .heading, .title: .semibold
        }
    }

    /// `DEFAULT_UI_FONT_SIZE`: the size a rem is 16 points at.
    public static let defaultInterfaceSize: CGFloat = 13
    /// GPUI's default line box as a multiple of the size (the golden ratio).
    public static let lineHeightRatio: CGFloat = 1.618_034
    @MainActor public var lineHeight: CGFloat { size * Self.lineHeightRatio }

    /// The theme's interface font (`metrics.ui_font_family`), or the system font when it sets none. `scale` is the
    /// window's own text step (`textScale`); `weight` and `design` replace the role's.
    @MainActor public func font(scale: CGFloat = 1, weight: Font.Weight? = nil, design: Font.Design? = nil) -> Font {
        font(in: ThemeStore.shared.tokens.metrics, scale: scale, weight: weight, design: design)
    }

    @MainActor func font(in metrics: ThemeMetrics, scale: CGFloat, weight: Font.Weight?, design: Font.Design?) -> Font {
        let size = size(in: metrics) * scale
        // Font files are registered for the whole process, so the app's count stands for every window's (a profile's
        // own fonts folder moves it too).
        _ = ThemeStore.shared.fontFiles
        if let family = metrics.uiFontFamily, family != Self.systemFont, let face = CTFont.face(of: family, size: size) {
            return Font(face).weight(weight ?? self.weight)
        }
        return .system(size: size, weight: weight ?? self.weight, design: design ?? .default)
    }

    @MainActor public var font: Font { font() }

    /// `SYSTEM_FONT`: what core's font settings call the system UI font.
    private static let systemFont = ".SystemUIFont"
}

extension EnvironmentValues {
    /// How much a window grows its text: Settings' own step (`settings_text_size`), 1 everywhere else.
    @Entry public var textScale: CGFloat = 1
}

extension ThemeTokens {
    /// The role's points in this theme's interface font size.
    public func size(_ role: TextRole) -> CGFloat { role.size(in: metrics) }
    /// The role's line box (`TextRole.lineHeightRatio` of its size).
    public func lineHeight(_ role: TextRole) -> CGFloat { size(role) * TextRole.lineHeightRatio }
    /// The role's font in this theme's interface font, or the system font when it sets none.
    @MainActor public func font(_ role: TextRole, scale: CGFloat = 1, weight: Font.Weight? = nil, design: Font.Design? = nil) -> Font {
        role.font(in: metrics, scale: scale, weight: weight, design: design)
    }
}

private struct RoleFont: ViewModifier {
    @Environment(\.textScale) private var scale
    @Environment(\.tokens) private var tokens
    let role: TextRole
    let weight: Font.Weight?
    let design: Font.Design?

    func body(content: Content) -> some View {
        content.font(tokens.font(role, scale: scale, weight: weight, design: design))
    }
}

extension View {
    /// The role's font, at the window's text step.
    public func textRole(_ role: TextRole, weight: Font.Weight? = nil, design: Font.Design? = nil) -> some View {
        modifier(RoleFont(role: role, weight: weight, design: design))
    }
}

extension CTFont {
    /// The family's face at `size`, or nil when CoreText has no such family and would give another. Asked of CoreText each
    /// time, not by SwiftUI's or AppKit's font names, which keep what they found: a family whose file came or went (the
    /// fonts folder) would stay as it first drew.
    static func face(of family: String, size: CGFloat) -> CTFont? {
        let font = CTFontCreateWithName(family as CFString, size, nil)
        return CTFontCopyFamilyName(font) as String == family ? font : nil
    }
}
