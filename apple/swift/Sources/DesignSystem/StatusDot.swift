// A terminal's state as a glyph in the lead column. Shape as well as colour says what it is, so the
// meaning survives colour blindness (the GPUI app's rule): ring, dot, haloed dot, check, cross.

import SwiftUI

public enum StatusKind: Sendable, Equatable {
    case idle, working, attention, done, failed
}

public struct StatusDot: View {
    public let kind: StatusKind
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var dim = false

    public init(_ kind: StatusKind) { self.kind = kind }

    public var body: some View {
        glyph
            .frame(width: Metric.lead, height: Metric.lead)
            // The row speaks the state in its own label; the glyph alone would be read twice.
            .accessibilityHidden(true)
    }

    @ViewBuilder private var glyph: some View {
        switch kind {
        case .idle:
            Circle().strokeBorder(Palette.textFaint, lineWidth: Metric.ring).frame(width: Metric.statusDot, height: Metric.statusDot)
        case .working:
            Circle().fill(Palette.working).frame(width: Metric.statusDot, height: Metric.statusDot)
                .opacity(dim ? 0.35 : 1)
                .onAppear { pulse() }
        case .attention:
            Circle().fill(Palette.attention).frame(width: Metric.statusDot, height: Metric.statusDot)
                .background(Circle().fill(Palette.attention.opacity(0.25)).padding(-Metric.statusHalo))
        case .done:
            Image(systemName: "checkmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(Palette.done)
        case .failed:
            Image(systemName: "xmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(Palette.danger)
        }
    }

    /// A gentle breathing, steady under Reduce Motion.
    private func pulse() {
        guard !reduceMotion else { return }
        withAnimation(.easeInOut(duration: 1.0).repeatForever(autoreverses: true)) { dim = true }
    }
}
