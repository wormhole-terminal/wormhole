// The shape a theme sets (`metrics.*`): the radii by role and the rule widths. A theme file may round
// its chips into pills or draw no divider, so views read these and never write a radius.

import Foundation

public struct ThemeMetrics: Sendable, Equatable, Decodable {
    /// Tiny elements: swatches, a small icon button's hover box.
    public let radiusSm: Double
    /// Tabs, choice chips, badges, key caps.
    public let radiusChip: Double
    /// Controls and rows: buttons, inputs, selects, list rows.
    public let radius: Double
    /// Cards, menus, popovers and other floating surfaces.
    public let radiusLg: Double
    /// Outlines and hairline rules.
    public let borderWidth: Double
    /// The lines between the window's columns; 0 draws none.
    public let dividerWidth: Double
    public let focusRingWidth: Double
    /// The interface font's size (13 unless the theme sets another); the type roles are rems of it.
    public let uiFontSize: Double
    /// The theme's own interface font; `nil` is the system font.
    public let uiFontFamily: String?

    enum CodingKeys: String, CodingKey {
        case radiusSm = "radius_sm", radiusChip = "radius_chip", radius, radiusLg = "radius_lg"
        case borderWidth = "border_width", dividerWidth = "divider_width", focusRingWidth = "focus_ring_width"
        case uiFontSize = "ui_font_size", uiFontFamily = "ui_font_family"
    }
}

/// The interface font the person picked in Settings → Fonts (`ui_font_family`, `ui_font_size`).
public struct InterfaceFont: Sendable, Equatable {
    /// Empty keeps the theme's own, or the system font.
    public var family = ""
    public var size = defaultSize
    /// `DEFAULT_UI_FONT_SIZE`: a size left here lets the theme's stand.
    public static let defaultSize = 13.0
    /// `FontSlot::Interface.size_range()`: what the fixed-height rows can hold.
    static let sizes = 10.0...20.0

    public init(family: String = "", size: Double = defaultSize) { (self.family, self.size) = (family, size) }
}

extension ThemeMetrics {
    /// `Settings::resolved_fonts`: the person's choice over the theme's, a size only when it is not the default.
    func with(_ font: InterfaceFont) -> ThemeMetrics {
        let family = font.family.trimmingCharacters(in: .whitespaces)
        let size = font.size != InterfaceFont.defaultSize ? font.size : uiFontSize
        return ThemeMetrics(
            radiusSm: radiusSm, radiusChip: radiusChip, radius: radius, radiusLg: radiusLg, borderWidth: borderWidth,
            dividerWidth: dividerWidth, focusRingWidth: focusRingWidth,
            uiFontSize: min(max(size, InterfaceFont.sizes.lowerBound), InterfaceFont.sizes.upperBound),
            uiFontFamily: family.isEmpty ? uiFontFamily : family)
    }
}
