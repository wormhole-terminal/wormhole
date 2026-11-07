// What a pane stacks over its grid, in the order `render/mod.rs` does: the host notice at the top, the bar above
// the exited bar at the bottom, 8 pt off the edge. Each takes the clicks that fall on it; the pane under keeps the rest.

import DesignSystem
import SwiftUI

public struct PaneOverlayState: Equatable, Sendable {
    public var bar: PaneBar?
    public var hostNotice: String?
    public var exited: Bool

    public init(bar: PaneBar? = nil, hostNotice: String? = nil, exited: Bool = false) {
        (self.bar, self.hostNotice, self.exited) = (bar, hostNotice, exited)
    }
}

public struct PaneOverlays: View {
    let state: PaneOverlayState
    let shell: String
    let exitCode: Int?
    let hostLost: Bool
    let gridWidth: CGFloat
    /// What the pane keeps clear at the right of the grid (the minimap), beside the padding.
    let trailingInset: CGFloat
    let actions: PaneBarActions

    public init(state: PaneOverlayState, shell: String, exitCode: Int? = nil, hostLost: Bool = false,
                gridWidth: CGFloat, trailingInset: CGFloat = 0, actions: PaneBarActions) {
        (self.state, self.shell, self.exitCode, self.hostLost, self.gridWidth, self.trailingInset, self.actions) =
            (state, shell, exitCode, hostLost, gridWidth, trailingInset, actions)
    }

    public var body: some View {
        VStack(spacing: 0) {
            if let notice = state.hostNotice { HostNoticeBanner(text: notice, actions: actions).transition(.opacity) }
            Spacer(minLength: 0).allowsHitTesting(false)
            if let bar = state.bar {
                PaneBarView(bar: bar, actions: actions, gridWidth: gridWidth)
                    .padding(.leading, Space._1).padding(.trailing, Space._1 + trailingInset).padding(.bottom, Space._2)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
            if state.exited {
                ExitedBar(shell: shell, exitCode: exitCode, hostLost: hostLost, actions: actions)
            }
        }
        .motion(Motion.layout, value: state)
    }
}
