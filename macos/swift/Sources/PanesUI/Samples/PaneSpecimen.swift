// Two pane headers over a divider at rest and one under the pointer, drawn by the views a split uses, for the
// Theme Studio's gallery (`studio/gallery/panes.rs`): the focused header wears the primary line, the other is muted.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

/// Two pane headers over their dividers, for the Studio's gallery.
public struct PaneSpecimen: View {
    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            header(PaneInfo(title: "claude", folder: "~/Projects/wormhole", activity: .working), focused: true)
            PaneDividerView(axis: .column, gaps: false, look: .rest)
            header(PaneInfo(title: "cargo test", folder: "~/Projects/wormhole/crates", activity: .idle), focused: false)
            PaneDividerView(axis: .column, gaps: false, look: .hot)
        }
        .allowsHitTesting(false)
    }

    private func header(_ info: PaneInfo, focused: Bool) -> some View {
        PaneHeader(
            info: info, focused: focused, gaps: false, zoomed: false, dragged: DraggedTerminal(id: 1, origin: .paneHeader), position: 1, of: 2,
            focus: {}, zoom: {}, close: {}, menu: { EmptyView() })
    }
}
