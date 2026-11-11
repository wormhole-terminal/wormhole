// What the window lays over the terminal while a diff or a file is open and the presentation is `.terminalArea`: the
// diff of the file open in Changes or History, or the viewer of the file open in Files, filling the terminal's own
// surface as the GPUI app's do. A diff's header is the title row's (`PanelCoverTitle`), so only its rows are here.
// Esc closes it and gives the keys back to the terminal.

import PanelModel
import SwiftUI

public struct PanelCoverView: View {
    let controller: PanelController

    public init(controller: PanelController) { self.controller = controller }

    public var body: some View {
        Group {
            switch controller.cover {
            case .changesDiff:
                ChangesDiffPane(store: controller.changes, headerInRow: true)
            case .historyDiff:
                HistoryDiffPane(store: controller.history, headerInRow: true)
            case .file:
                FileViewerView(store: controller.files, showsHeader: false)
            case nil:
                Color.clear
            }
        }
        .environment(\.panelHost, controller.host)
        .environment(\.panelRoot, controller.folder)
        .environment(\.showFileHistory, controller.showFileHistory)
        .environment(\.panelCopy, { [host = controller.host] in host.copy($0, as: $1) })
    }
}
