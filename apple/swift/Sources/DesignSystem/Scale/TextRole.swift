// The four type roles of the GPUI app plus its label role (`style/scale.rs` `TextRole`): sizes are rems of
// the interface font (13 pt, `DEFAULT_UI_FONT_SIZE`, in the system font), so at the default they are
// 12 / 13 / 14 / 16 / 20. Line height is GPUI's own ratio unless a row fixes it (`NAME_LINE`, `META_LINE`).

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

    public var size: CGFloat {
        switch self {
        case .caption: 12
        case .label: 13
        case .body: 14
        case .heading: 16
        case .title: 20
        }
    }

    public var weight: Font.Weight {
        switch self {
        case .caption, .body: .regular
        case .label: .semibold
        case .heading, .title: .semibold
        }
    }

    /// GPUI's default line box as a multiple of the size (the golden ratio).
    public static let lineHeightRatio: CGFloat = 1.618_034
    public var lineHeight: CGFloat { size * Self.lineHeightRatio }

    public var font: Font { .system(size: size, weight: weight) }
}

extension View {
    public func textRole(_ role: TextRole) -> some View { font(role.font) }
}
