// A row arriving in the list or leaving it, and the rows of a group folding or opening, drawn from the core's pose tables
// (`WorkspaceModel.RowMotion`, `view::motion::pose`): one animated number carries the motion, and each frame asks the core
// where the row stands. 0...1 is the row coming (1 is at rest), 1...2 the row going. Without the tables (previews, tests)
// the rows keep their plain fade.

import SwiftUI
import WorkspaceModel

/// The row's height scaled by the pose's `clip`, so the rows around follow frame by frame.
private struct ClipHeight: Layout {
    var clip: Double
    var animatableData: Double {
        get { clip }
        set { clip = newValue }
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let size = subviews.first?.sizeThatFits(ProposedViewSize(width: proposal.width, height: nil)) ?? .zero
        return CGSize(width: size.width, height: size.height * clip)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, anchor: .topLeading, proposal: ProposedViewSize(width: bounds.width, height: nil))
    }
}

private struct RowPoseEffect: ViewModifier, @preconcurrency Animatable {
    var progress: Double
    let pose: @Sendable (_ comingAt: Double?, _ goingAt: Double?) -> RowPose

    var animatableData: Double {
        get { progress }
        set { progress = newValue }
    }

    func body(content: Content) -> some View {
        let p = progress <= 1 ? pose(progress, nil) : pose(nil, progress - 1)
        ClipHeight(clip: p.clip) { content.opacity(p.opacity).offset(x: p.dx, y: p.dy) }
            .clipped(antialiased: false)
    }
}

private struct RowPoseTransition: Transition {
    let pose: @Sendable (Double?, Double?) -> RowPose

    func body(content: Content, phase: TransitionPhase) -> some View {
        content.modifier(RowPoseEffect(progress: phase == .willAppear ? 0 : phase == .didDisappear ? 2 : 1, pose: pose))
    }
}

extension AnyTransition {
    /// The core's `motion.insert` choice `variant` for a row of the list; the plain fade without the tables.
    static func listRow(_ variant: String, reduceMotion: Bool) -> AnyTransition {
        guard let tables = RowMotion.tables, !reduceMotion else { return .opacity }
        return motion(
            coming: tables.insertPlan(variant, true).seconds, going: tables.insertPlan(variant, false).seconds,
            pose: { coming, going in coming.map { tables.insertPose(variant, true, $0) } ?? tables.insertPose(variant, false, going ?? 1) })
    }

    /// The pose as the animation's progress, linear: the pose's own easing is the core's. A motion of no length is a cut.
    private static func motion(coming: Double, going: Double, pose: @escaping @Sendable (Double?, Double?) -> RowPose) -> AnyTransition {
        let transition = AnyTransition(RowPoseTransition(pose: pose))
        return .asymmetric(
            insertion: transition.animation(coming > 0 ? .linear(duration: coming) : nil),
            removal: transition)
    }
}
