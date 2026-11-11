// The cue that a code column runs on past its edge (`overflow_cue`): a soft fade over the column's end where lines are cut,
// on the right while there is more and on the left once the code has scrolled, and a thin bar under the code saying where
// the view is. Over the rows, never in the way of the pointer.

import DesignSystem
import SwiftUI

struct OverflowCue: View {
    /// Where each column's code is, in the body's x.
    let spans: [ClosedRange<CGFloat>]
    /// How much of the code shows, how far it is scrolled and how far it can be.
    let visible: CGFloat
    let scroll: CGFloat
    let max: CGFloat
    @Environment(\.tokens) private var tokens

    private static let fade: CGFloat = 24
    private static let barHeight: CGFloat = 6
    private static let barEnd: CGFloat = 4
    private static let barBottom: CGFloat = 3
    private static let thumbMin: CGFloat = 24

    var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(spans.enumerated()), id: \.offset) { _, span in
                if scroll < max { fade(at: span.upperBound - Self.fade, from: .clear, to: tokens.surfaceWindow) }
                if scroll > 0 { fade(at: span.lowerBound, from: tokens.surfaceWindow, to: .clear) }
                bar(span)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .allowsHitTesting(false)
    }

    private func fade(at x: CGFloat, from: Color, to: Color) -> some View {
        LinearGradient(colors: [from, to], startPoint: .leading, endPoint: .trailing)
            .frame(width: Self.fade).frame(maxHeight: .infinity).offset(x: x)
    }

    private func bar(_ span: ClosedRange<CGFloat>) -> some View {
        let track = Swift.max(0, span.upperBound - span.lowerBound - Self.barEnd)
        let thumb = Swift.min(Swift.max(track * visible / (visible + max), Self.thumbMin), track)
        let thumbAt = (track - thumb) * Swift.min(Swift.max(scroll / max, 0), 1)
        return VStack {
            Spacer(minLength: 0)
            Capsule().fill(tokens.stateHover).frame(width: track, height: Self.barHeight)
                .overlay(alignment: .leading) { Capsule().fill(tokens.scrollbarThumb).frame(width: thumb).offset(x: thumbAt) }
                .padding(.bottom, Self.barBottom)
        }
        .frame(maxWidth: .infinity, alignment: .topLeading).padding(.leading, span.lowerBound)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
