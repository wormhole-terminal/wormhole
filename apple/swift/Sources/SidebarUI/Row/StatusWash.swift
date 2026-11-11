// The card washed in the new status's colour, which fades away (`Wash` in `view/motion/pose/status.rs`: the tint comes
// up over 120 ms and goes over 900 ms, at `WASH_TINT` of the colour at its fullest). One keyframe run per change,
// started by the row's change count; nothing is drawn at rest.

import DesignSystem
import SwiftUI

struct StatusWash: View {
    let state: TileState
    /// How many status changes the row has had: each one runs the wash once.
    let count: Int
    let enabled: Bool
    let shape: RoundedRectangle
    @Environment(\.tokens) private var tokens

    /// How strong a wash is at its fullest (`WASH_TINT`).
    private static let tint = 0.16

    var body: some View {
        if enabled, let color {
            KeyframeAnimator(initialValue: 0.0, trigger: count) { level in
                shape.fill(color.opacity(Self.tint * level)).allowsHitTesting(false)
            } keyframes: { _ in
                KeyframeTrack {
                    LinearKeyframe(1.0, duration: 0.12)
                    CubicKeyframe(0.0, duration: 0.9)
                }
            }
        }
    }

    /// The statuses that say something get a wash; a terminal gone quiet or whose command just runs gets none.
    private var color: Color? {
        switch state {
        case .needsYou, .bell: tokens.agentAttention
        case .done: tokens.agentDone
        case .failed: tokens.statusDanger
        case .working: tokens.agentWorking
        case .unread, .running, .shell: nil
        }
    }
}
