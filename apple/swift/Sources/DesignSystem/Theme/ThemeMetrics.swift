// The shape a theme sets (`metrics.*`): the radii by role and the rule widths. A theme file may round
// its chips into pills or draw no divider, so views read these and never write a radius.

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
