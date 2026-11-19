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
    @Environment(\.listShownAt) private var shownAt
    @Environment(\.reduceMotion) private var reduceMotion
    /// How far through its drawing down the rows the bracket is (0...1); 1 at rest.
    @State private var drawn = 1.0

    private static let width: CGFloat = 2
    /// A bracket that appears once the list has been up a while is a split just made; the ones a list is first drawn with are not.
    private static let settle: TimeInterval = 1.5

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
        .modifier(BracketDraw(progress: drawn, stretch: stretch))
        .onAppear(perform: draw)
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }

    /// The part of the drawing time this row's stretch is drawn in: down the rows, the first row's before the last one's.
    private var stretch: (from: Double, to: Double) {
        switch part {
        case .start: (0, 0.5)
        case .mid: (0.25, 0.75)
        case .end: (0.5, 1)
        }
    }

    /// A split's bracket draws down its rows as the split is made (`motion/join.rs` `draw_bracket`), the core's time and curve.
    private func draw() {
        guard !reduceMotion, Date().timeIntervalSince(shownAt) > Self.settle, let tables = RowMotion.tables, case let seconds = tables.bracketSeconds(),
            seconds > 0
        else { return }
        drawn = 0
        Task { @MainActor in withAnimation(.linear(duration: seconds)) { drawn = 1 } }
    }
}

/// The bracket's opacity at a moment of its drawing: the core's share of the time its stretch is drawn in.
private struct BracketDraw: ViewModifier, @preconcurrency Animatable {
    var progress: Double
    let stretch: (from: Double, to: Double)

    var animatableData: Double {
        get { progress }
        set { progress = newValue }
    }

    func body(content: Content) -> some View {
        content.opacity(RowMotion.tables?.bracketShare(progress, stretch.from, stretch.to) ?? 1)
    }
}
