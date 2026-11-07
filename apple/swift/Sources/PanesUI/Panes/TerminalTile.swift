// A terminal's tile in a pane's header and the title row: the terminal symbol in a rounded square drawn
// like an app's icon, black unless the theme says otherwise, with the live state at its corner. The sidebar
// draws the same tile larger with more state (`SidebarUI` `StatusTile`); this is the small, shared-by-two
// use, until that one moves into the design system.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct TerminalTile: View {
    let activity: Activity
    private let given: CGFloat?
    @Environment(\.tokens) private var tokens

    public init(activity: Activity, size: CGFloat? = nil) { (self.activity, given) = (activity, size) }

    private var size: CGFloat { given ?? PaneMetrics.tile }

    public var body: some View {
        let shape = RoundedRectangle(cornerRadius: size * 0.22, style: .continuous)
        shape.fill(tokens.tileBackground)
            .overlay { shape.strokeBorder(tokens.tileEdge, lineWidth: tokens.metrics.borderWidth) }
            .overlay {
                Image(systemName: "terminal")
                    .font(.system(size: size * 0.55, weight: .medium))
                    .foregroundStyle(tokens.tileMark)
            }
            .overlay(alignment: .bottomTrailing) { state.padding(-size * 0.12) }
            .frame(width: size, height: size)
            .motion(Motion.selection, value: activity)
            .accessibilityHidden(true)
    }

    /// The state's dot at the tile's corner: shape as well as colour, so it survives colour blindness.
    @ViewBuilder private var state: some View {
        switch activity {
        case .idle: EmptyView()
        case .working: dot(tokens.agentWorking)
        case .needsInput: dot(tokens.agentAttention)
        case .finished: dot(tokens.agentDone)
        case .failed: dot(tokens.statusDanger)
        }
    }

    private func dot(_ colour: Color) -> some View {
        Circle().fill(colour).frame(width: size * 0.36, height: size * 0.36)
            .overlay { Circle().strokeBorder(tokens.surfaceWindow, lineWidth: 1.5) }
            .transition(.scale.combined(with: .opacity))
    }
}
