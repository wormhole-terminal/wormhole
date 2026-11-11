// A file shown in place of the terminal, or under the tree (`features/files/viewer`): the header with the file's path,
// its size, Open in… and close (over the terminal the window's title row carries it), and the body for what the file is: lines with their numbers and the
// syntax colours, a picture scaled to fit, or a panel that says why there is nothing to read. Esc closes it.

import DesignSystem
import PanelModel
import SwiftUI

struct FileViewerView: View {
    let store: FilesStore
    /// The window draws the header in the terminal's title row while the file covers the terminal.
    var showsHeader = true
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let path = store.viewing {
            VStack(spacing: 0) {
                if showsHeader {
                    FileViewerHeader(store: store, path: path)
                    Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                }
                FileViewerBody(store: store, path: path)
            }
            .background(tokens.surfaceWindow)
            .onExitCommand(perform: store.closeViewer)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(panelText("files-spoken-viewer", ["name": .text(path.split(separator: "/").last.map(String.init) ?? path)], "\(path.split(separator: "/").last.map(String.init) ?? path), file viewer"))
        }
    }
}

struct FileViewerHeader: View {
    let store: FilesStore
    let path: String
    /// In the terminal's title row the row gives the height and the insets.
    var inTitleRow = false
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        let name = path.split(separator: "/").last.map(String.init) ?? path
        let folders = path.contains("/") ? String(path[..<path.lastIndex(of: "/")!]) + "/" : ""
        HStack(spacing: PanelMetrics.gap) {
            FileIcon(kind: .file(name), tint: tokens.textMuted)
            // The path as the diff's crumb draws it: folders quiet, the name loud, the head elided first.
            Text("\(Text(folders).foregroundStyle(tokens.textMuted))\(Text(name).foregroundStyle(tokens.textPrimary))")
                .font(TextRole.body.font).lineLimit(1).truncationMode(.head).layoutPriority(-1)
            Spacer(minLength: 0)
            if let size = store.content.size { Text(ByteSize.text(size)).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize() }
            if let editor = host?.editorName {
                BandButton(symbol: "arrow.up.forward.square", label: panelText("files-open-in-editor", ["editor": .text(editor)], "Open in \(editor)"), inRow: true) { host?.open(absolute, line: nil) }
            }
            BandButton(symbol: "xmark", label: panelText("files-viewer-close", "Close file (esc)"), inRow: true, action: store.closeViewer)
        }
        .padding(.horizontal, inTitleRow ? 0 : PanelMetrics.inset)
        .frame(height: inTitleRow ? nil : 40)
    }

    private var absolute: String { store.root.map { store.join($0, path) } ?? path }
}
