// The one place colours, spacing, type and radii are named. Views never write a literal number or colour.
// Spacing is a 4-pt grid like the GPUI app's `style/`; colours follow its tokens (`sidebar.row.selected`,
// `agent.*`, `status.*`) but come from the system's adaptive colours, so light and dark need no second table.

import SwiftUI

public enum Space {
    public static let _1: CGFloat = 4
    public static let _2: CGFloat = 8
    public static let _3: CGFloat = 12
    public static let _4: CGFloat = 16
    public static let _6: CGFloat = 24
}

public enum Radius {
    public static let chip: CGFloat = 6
    public static let row: CGFloat = 8
    public static let card: CGFloat = 12
}

/// Sizes that are not spacing: the GPUI scale's heights, glyphs and hit areas (`style/scale.rs`).
public enum Metric {
    /// A form control and an icon button's face.
    public static let control: CGFloat = 28
    /// A two-line list row: four above, a 20 name line, a 16 meta line, four below.
    public static let rowTwoLine: CGFloat = 44
    /// The inset from a column's edge to its content, shared by header, rows and search.
    public static let edgeInset: CGFloat = 12
    /// The lead column of a row: a status glyph, a chevron.
    public static let lead: CGFloat = 16
    public static let glyph: CGFloat = 14
    public static let badgeHeight: CGFloat = 16
    public static let statusDot: CGFloat = 8
    public static let statusHalo: CGFloat = 3
    public static let ring: CGFloat = 1.5
    public static let chevron: CGFloat = 9
}

public enum Palette {
    public static let sidebarBackground = Color(nsColor: .underPageBackgroundColor)
    public static let hover = Color.primary.opacity(0.06)
    public static let selected = Color.accentColor.opacity(0.18)
    public static let divider = Color.primary.opacity(0.10)
    public static let textPrimary = Color.primary
    public static let textSecondary = Color.secondary
    /// Idle rings and hints: quieter than secondary text.
    public static let textFaint = Color.primary.opacity(0.30)
    public static let working = Color.accentColor
    public static let attention = Color.orange
    public static let done = Color.green
    public static let danger = Color.red
    /// A field's face, a badge's face: a faint wash that reads on both appearances.
    public static let fill = Color.primary.opacity(0.08)
}

/// The four type roles of the GPUI app (12 / 14 / 16 / 20), as fonts that scale with Dynamic Type.
public enum TextRole {
    case caption, body, heading, title

    public var font: Font {
        switch self {
        case .caption: .system(size: 12)
        case .body: .system(size: 14)
        case .heading: .system(size: 16, weight: .semibold)
        case .title: .system(size: 20, weight: .semibold)
        }
    }
}

extension View {
    public func textRole(_ role: TextRole) -> some View { font(role.font) }
}
