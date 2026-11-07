// The tick box of a next step. The circle fills and the mark draws itself, in one spring, so the state change
// reads as an action rather than a swap. Reduce Motion cross-fades the same two states.

import DesignSystem
import SwiftUI

struct TaskCheck: View {
    let done: Bool
    @Environment(\.panelMotion) private var motion

    var body: some View {
        ZStack {
            Circle().strokeBorder(.tertiary, lineWidth: 1.2).opacity(done ? 0 : 1)
            Circle().fill(Color.accentColor).scaleEffect(done || motion.reduced ? 1 : 0.4).opacity(done ? 1 : 0)
            Mark().trim(from: 0, to: done ? 1 : 0).stroke(.white, style: StrokeStyle(lineWidth: 1.6, lineCap: .round, lineJoin: .round))
                .padding(4.5)
        }
        .frame(width: 16, height: 16)
        .animation(motion.tick, value: done)
        .accessibilityHidden(true)
    }

    private struct Mark: Shape {
        func path(in rect: CGRect) -> Path {
            var path = Path()
            path.move(to: CGPoint(x: rect.minX, y: rect.midY + 0.1 * rect.height))
            path.addLine(to: CGPoint(x: rect.minX + 0.38 * rect.width, y: rect.maxY - 0.1 * rect.height))
            path.addLine(to: CGPoint(x: rect.maxX, y: rect.minY + 0.1 * rect.height))
            return path
        }
    }
}
