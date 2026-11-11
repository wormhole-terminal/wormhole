// Flare and Nudge, the status motions that add something on top of a tile that has already changed: a ring leaves the tile's
// edge and fades (Flare), the row moves aside and back (Nudge). Both are for a terminal that starts asking for input, and the
// numbers are the core's: one linear run per change, and each frame asks the core where the pose stands
// (`view::motion::pose::status`, through `RowMotion.tables`). Off under Reduce Motion, and for any other change.

import DesignSystem
import SwiftUI
import WorkspaceModel

/// A terminal's latest status change in the list: how many it has had (each starts one run) and between which states.
struct StatusShift: Equatable {
    var count: Int
    var from: TileState
    var to: TileState
}

extension StatusMotion {
    /// The settings' spelling, which the core's tables are asked by.
    var coreName: String {
        switch self {
        case .classic: "classic"
        case .crossfade: "crossfade"
        case .badgePop: "badge-pop"
        case .wash: "wash"
        case .flare: "flare"
        case .nudge: "nudge"
        case .lineRoll: "line-roll"
        }
    }
}

private struct StatusPoseEffect: ViewModifier, @preconcurrency Animatable {
    var t: Double
    let shift: StatusShift?
    let variant: StatusMotion
    let size: CGFloat?
    let ring: Color

    var animatableData: Double {
        get { t }
        set { t = newValue }
    }

    private var pose: StatusPose {
        guard let shift, t < 1, let tables = RowMotion.tables else { return .rest }
        return tables.statusPose(variant.coreName, shift.from.rawValue, shift.to.rawValue, t)
    }

    func body(content: Content) -> some View {
        let pose = pose
        if let size {
            content.overlay {
                // The ring leaves the amber edge: as thick as it reaches far, so it fills the room between the edge and where it
                // has got to, and fades as it goes (`style/tile/transition.rs`).
                if pose.flareReach > 0, pose.flareStrength > 0 {
                    let out = TileGeometry.ringOut + pose.flareReach
                    RoundedRectangle(cornerRadius: size * TileGeometry.corner + out)
                        .strokeBorder(ring.opacity(pose.flareStrength), lineWidth: pose.flareReach)
                        .frame(width: size + 2 * out, height: size + 2 * out)
                        .allowsHitTesting(false)
                }
            }
        } else {
            content.offset(x: pose.dx)
        }
    }
}

private struct StatusPlayer: ViewModifier {
    let shift: StatusShift?
    let variant: StatusMotion
    /// The tile's side for a flare; nil is a nudge of the whole row.
    let tile: CGFloat?
    @Environment(\.sidebarLook) private var look
    @Environment(\.reduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens
    @State private var t = 1.0

    func body(content: Content) -> some View {
        content
            .modifier(StatusPoseEffect(t: t, shift: shift, variant: variant, size: tile, ring: tokens.agentAttention))
            .onChange(of: shift?.count) { _, _ in play() }
    }

    private func play() {
        guard look.statusMotion == variant, !reduceMotion, let shift, let tables = RowMotion.tables else { return }
        let seconds = tables.statusSeconds(variant.coreName, shift.from.rawValue, shift.to.rawValue)
        guard seconds > 0 else { return }
        t = 0
        // The start must be seen before the run is: the next turn of the main loop.
        Task { @MainActor in withAnimation(.linear(duration: seconds)) { t = 1 } }
    }
}

extension View {
    /// The row moves aside and back when its terminal starts asking for input and the person chose Nudge.
    func statusNudge(_ shift: StatusShift?) -> some View { modifier(StatusPlayer(shift: shift, variant: .nudge, tile: nil)) }

    /// A ring leaves the tile's edge when its terminal starts asking for input and the person chose Amber flare.
    func statusFlare(_ shift: StatusShift?, tile: CGFloat) -> some View { modifier(StatusPlayer(shift: shift, variant: .flare, tile: tile)) }
}
