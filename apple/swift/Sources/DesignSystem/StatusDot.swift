// A terminal's state as a glyph in the lead column. Shape as well as colour says what it is, so the
// meaning survives colour blindness (the GPUI app's rule): ring, dot, haloed dot, check, cross.
// A change of state swaps the glyph with a small spring; working breathes and needs-input's halo swells,
// both steady under Reduce Motion.

import SwiftUI

public enum StatusKind: Sendable, Equatable {
    case idle, working, attention, done, failed
}

public struct StatusDot: View {
    public let kind: StatusKind
    @Environment(\.reduceMotion) private var reduceMotion

    public init(_ kind: StatusKind) { self.kind = kind }

    public var body: some View {
        ZStack {
            glyph
                .id(kind)
                .motionTransition(.scale(scale: 0.4).combined(with: .opacity))
        }
        .frame(width: Metric.lead, height: Metric.lead)
        .marked("status.glyph")
        .motion(Motion.selection, value: kind)
        // The row speaks the state in its own label; the glyph alone would be read twice.
        .accessibilityHidden(true)
    }

    private var dot: some View { Circle().frame(width: Metric.statusDot, height: Metric.statusDot) }

    @ViewBuilder private var glyph: some View {
        switch kind {
        case .idle:
            Circle().strokeBorder(Palette.textFaint, lineWidth: Metric.ring)
                .frame(width: Metric.statusDot, height: Metric.statusDot)
        case .working:
            if reduceMotion {
                dot.foregroundStyle(Palette.working)
            } else {
                dot.foregroundStyle(Palette.working)
                    .phaseAnimator([1.0, 0.35]) { view, opacity in view.opacity(opacity) } animation: { _ in Motion.breath }
            }
        case .attention:
            dot.foregroundStyle(Palette.attention).background { halo }
        case .done:
            Image(systemName: "checkmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(Palette.done)
        case .failed:
            Image(systemName: "xmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(Palette.danger)
        }
    }

    @ViewBuilder private var halo: some View {
        let ring = Circle().fill(Palette.attention.opacity(0.25)).padding(-Metric.statusHalo)
        if reduceMotion {
            ring
        } else {
            ring.phaseAnimator([1.0, 1.3]) { view, scale in view.scaleEffect(scale) } animation: { _ in Motion.breath }
        }
    }
}
