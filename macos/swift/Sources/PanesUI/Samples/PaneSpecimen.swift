// Two pane headers over a divider at rest, one under the pointer and one dragged, drawn by the views a split uses, for the
// Theme Studio's gallery (`studio/gallery/panes.rs`): the focused header wears the primary line, the other is muted. Under
// them a pane as a terminal is dragged over it: the side it would land on lit with what a drop does, the others as hints.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

/// Two pane headers over their dividers, for the Studio's gallery.
public struct PaneSpecimen: View {
    @Environment(\.tokens) private var tokens
    private static let pane = (width: 320.0, height: 160.0)

    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header(PaneInfo(title: "claude", folder: "~/Projects/wormhole", activity: .working), focused: true)
            PaneDividerView(axis: .column, gaps: false, look: .rest)
            header(PaneInfo(title: "cargo test", folder: "~/Projects/wormhole/crates", activity: .idle), focused: false)
            PaneDividerView(axis: .column, gaps: false, look: .hot)
            // The divider while it is dragged.
            PaneDividerView(axis: .column, gaps: false, look: .dragging).padding(.top, Space._2)
            dropZones.padding(.top, Space._4)
        }
        .allowsHitTesting(false)
    }

    /// The drop overlay over one pane, a terminal from the sidebar held on its right side.
    private var dropZones: some View {
        let panes = PanesState(shown: [PanePlace(1, PaneRect(0, 0, Self.pane.width, Self.pane.height))], focused: 1)
        let drop = PaneDropState(dragged: DraggedTerminal(id: 2, origin: .sidebarRow), target: 1, zone: .right)
        return PaneDropZones(panes: panes, state: drop, size: CGSize(width: Self.pane.width, height: Self.pane.height), names: { _ in "claude" }, refusal: { _, _, _ in nil })
            .background(tokens.terminal.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
    }

    private func header(_ info: PaneInfo, focused: Bool) -> some View {
        PaneHeader(
            info: info, focused: focused, gaps: false, zoomed: false, dragged: DraggedTerminal(id: 1, origin: .paneHeader), position: 1, of: 2,
            focus: {}, zoom: {}, close: {}, menu: { EmptyView() })
    }
}
