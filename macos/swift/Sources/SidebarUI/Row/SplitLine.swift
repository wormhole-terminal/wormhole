// A split in the list (`style/splits.rs`): the rows of a split sit together in one card, joined by a thin line through
// the middle of their tiles, behind them. It runs from the middle of the first row to the middle of the last, so it
// never reaches the card's edge, and only its two ends are rounded, so the stretches between rows meet without a notch.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SplitLine: View {
    let part: Bracket
    /// How far in from the row's left the line runs: the middle of the tile it joins.
    let x: CGFloat
    @Environment(\.tokens) private var tokens

    private static let width: CGFloat = 2

    var body: some View {
        GeometryReader { area in
            let (top, bottom): (CGFloat, CGFloat) = switch part {
            case .start: (area.size.height / 2, area.size.height)
            case .mid: (0, area.size.height)
            case .end: (0, area.size.height / 2)
            }
            UnevenRoundedRectangle(
                topLeadingRadius: part == .start ? Self.width / 2 : 0, bottomLeadingRadius: part == .end ? Self.width / 2 : 0,
                bottomTrailingRadius: part == .end ? Self.width / 2 : 0, topTrailingRadius: part == .start ? Self.width / 2 : 0
            )
            .fill(tokens.textMuted)
            .frame(width: Self.width, height: bottom - top)
            .offset(x: x - Self.width / 2, y: top)
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}
