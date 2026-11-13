// The Theme Studio's mark on one element: a dashed outline in the primary while the selected token is drawn on it, so an
// edit shows where it lands (`Env::mark`, `style::uses_outline`). A view that is a component of the design names itself
// with `.marked("sidebar.row")`, the id core's `TOKEN_USES` gives that component; the Studio's gallery puts the ids of what
// the selected token is drawn on into the environment, and everywhere else the set is empty and the mark draws nothing.
// The same call is what inspect mode knows an element by: it records where the element was built (`InspectSite`), and
// while the inspector is on the element reports its frame (`Inspect/Inspection.swift`).

import SwiftUI

/// The ids of the components the selected token is drawn on (the gallery's, empty outside it).
public struct TokenMarks: Equatable, Sendable {
    /// The component ids the selected token is drawn on.
    public var drawn: Set<String>
    /// What VoiceOver says of a marked element ("uses the selected token"); empty: the outline is only seen.
    public var spoken: String

    /// Marks the components `drawn` names (none by default).
    public init(drawn: Set<String> = [], spoken: String = "") {
        self.drawn = drawn
        self.spoken = spoken
    }
}

extension EnvironmentValues {
    /// What the Studio's gallery outlines; empty everywhere else.
    @Entry public var tokenMarks = TokenMarks()
}

extension View {
    /// Outlines this view while the Studio's selected token is drawn on the component `id` names, and names it for
    /// inspect mode: built at `file`:`line` (the caller's; a shared component passes its own caller's on), with `style`
    /// saying its spacing, radius and text in the scales' names.
    public func marked(_ id: String, style: String = "", file: StaticString = #fileID, line: UInt = #line) -> some View {
        modifier(TokenMark(site: InspectSite(id: id, style: style, file: file, line: line)))
    }
}

private struct TokenMark: ViewModifier {
    let site: InspectSite
    private var id: String { site.id }
    @Environment(\.tokenMarks) private var marks
    @Environment(\.tokens) private var tokens

    /// GPUI's outline stands this far off what it marks, in a line this wide: it draws a dash twice the line's width, so a
    /// hairline would read as dots (`style/controls.rs` `USES_OUTLINE_OFFSET`, `USES_OUTLINE_W`).
    private static let offset: CGFloat = 2
    private static let line: CGFloat = 1.5

    func body(content: Content) -> some View {
        content.overlay {
            if marks.drawn.contains(id) {
                RoundedRectangle(cornerRadius: tokens.metrics.radius + Self.offset)
                    .strokeBorder(tokens.primary, style: StrokeStyle(lineWidth: Self.line, dash: [Self.line * 2, Self.line * 2]))
                    .padding(-Self.offset)
                    .allowsHitTesting(false)
                    .accessibilityHidden(marks.spoken.isEmpty)
                    .accessibilityLabel(marks.spoken)
            }
        }
        // Inspect mode: where this element is. Nothing while it is off.
        .background { if Inspection.shared.isOn { InspectProbe(site: site).accessibilityHidden(true) } }
    }
}
