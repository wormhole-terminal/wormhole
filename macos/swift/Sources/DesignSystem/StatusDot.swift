// A terminal's state as a glyph in the lead column (`style::StatusGlyph`). Shape as well as colour says what it is, so
// the meaning survives colour blindness (the GPUI app's rule): ring, dot, haloed dot, check, cross, bell, terminal.
// A change of state swaps the glyph with a small spring; working breathes and needs-input's halo swells,
// both steady under Reduce Motion.

import SwiftUI

public enum StatusKind: Sendable, Equatable {
    case idle, working, attention, done, failed
    /// A command runs in a terminal nothing tells the state of: the working drawing, neutral (muted).
    case running
    /// A terminal nothing tells the state of rang its bell: an amber bell, until it is seen.
    case bell
    /// A bare shell at its prompt, nothing running: the terminal symbol, faint.
    case shell
}

public struct StatusDot: View {
    @Environment(\.tokens) private var tokens
    public let kind: StatusKind
    @Environment(\.reduceMotion) private var reduceMotion

    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(_ kind: StatusKind, file: StaticString = #fileID, line: UInt = #line) { (self.kind, self.file, self.line) = (kind, file, line) }

    public var body: some View {
        ZStack {
            glyph
                .id(kind)
                .motionTransition(.scale(scale: 0.4).combined(with: .opacity))
        }
        .frame(width: Metric.lead, height: Metric.lead)
        .marked("status.glyph", style: "size lead · dot statusDot · ring ring", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .motion(Motion.selection, value: kind)
        // The row speaks the state in its own label; the glyph alone would be read twice.
        .accessibilityHidden(true)
    }

    private var dot: some View { Circle().frame(width: Metric.statusDot, height: Metric.statusDot) }

    @ViewBuilder private var glyph: some View {
        switch kind {
        case .idle:
            Circle().strokeBorder(tokens.textFaint, lineWidth: Metric.ring)
                .frame(width: Metric.statusDot, height: Metric.statusDot)
        case .working: working(tokens.agentWorking)
        case .running: working(tokens.textMuted)
        case .attention:
            dot.foregroundStyle(tokens.agentAttention).background { halo }
        case .done:
            Image(systemName: "checkmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(tokens.agentDone)
        case .failed:
            Image(systemName: "xmark.circle.fill").font(.system(size: Metric.glyph)).foregroundStyle(tokens.statusDanger)
        case .bell:
            Image(systemName: "bell.fill").font(.system(size: Metric.glyph - 3)).foregroundStyle(tokens.agentAttention)
        case .shell:
            Image(systemName: "terminal").font(.system(size: Metric.glyph - 3)).foregroundStyle(tokens.textFaint)
        }
    }

    /// The working drawing: a dot that breathes, still under Reduce Motion.
    @ViewBuilder private func working(_ colour: Color) -> some View {
        if reduceMotion {
            dot.foregroundStyle(colour)
        } else {
            dot.foregroundStyle(colour)
                .phaseAnimator([1.0, 0.35]) { view, opacity in view.opacity(opacity) } animation: { _ in Motion.breath }
        }
    }

    @ViewBuilder private var halo: some View {
        let ring = Circle().fill(tokens.agentAttention.opacity(0.25)).padding(-Metric.statusHalo)
        if reduceMotion {
            ring
        } else {
            ring.phaseAnimator([1.0, 1.3]) { view, scale in view.scaleEffect(scale) } animation: { _ in Motion.breath }
        }
    }
}
