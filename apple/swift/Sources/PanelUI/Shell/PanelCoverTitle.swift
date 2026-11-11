// The diff's header as the terminal's title row shows it while the diff covers the terminal (the GPUI app's title
// row becomes the diff's header). The rows are `PanelCoverView`'s; both read one `DiffSession`, which keeps the layout
// and wrapping they share. A file's header is the viewer's own, drawn in the same place.

import PanelModel
import SwiftUI

public struct PanelCoverTitle: View {
    let controller: PanelController

    public init(controller: PanelController) { self.controller = controller }

    /// Whether the cover has a header for the title row to carry.
    public static func carries(_ cover: PanelCover?) -> Bool { cover != nil }

    public var body: some View {
        switch controller.cover {
        case .changesDiff:
            header(controller.changes.current, session: controller.changes.diff, position: controller.changes.position,
                   step: controller.changes.step, close: controller.changes.close)
        case .historyDiff:
            header(controller.history.current, session: controller.history.diff, position: controller.history.position,
                   step: controller.history.step, close: controller.history.close)
        case .file:
            if let path = controller.files.viewing {
                FileViewerHeader(store: controller.files, path: path, inTitleRow: true)
                    .environment(\.panelHost, controller.host)
            }
        case nil:
            Color.clear
        }
    }

    @ViewBuilder private func header(
        _ file: ChangedFile?, session: DiffSession, position: (index: Int, count: Int)?,
        step: @escaping (Int) -> Void, close: @escaping () -> Void
    ) -> some View {
        if let file {
            @Bindable var session = session
            GeometryReader { proxy in
                DiffHeader(
                    file: file, diff: session.parsed, position: position, splitFits: proxy.size.width >= DiffView.splitMinWidth && DiffView.hasLines(session),
                    split: $session.split, wrap: $session.wrap, step: step, close: controller.closeCover, gone: session.gone, outOfDate: session.refreshError,
                    inTitleRow: true)
                    .frame(maxHeight: .infinity)
            }
            .environment(\.panelHost, controller.host)
            .environment(\.panelRoot, controller.folder)
        }
    }
}
