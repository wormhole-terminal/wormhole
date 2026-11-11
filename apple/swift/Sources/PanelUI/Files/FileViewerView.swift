// A file shown in place of the terminal, or under the tree (`features/files/viewer`): the header with the file's path,
// its size, Open in…, Reveal, Copy Path and close, and the body for what the file is: lines with their numbers and the
// syntax colours, a picture scaled to fit, or a panel that says why there is nothing to read. Esc closes it.

import DesignSystem
import PanelModel
import SwiftUI

struct FileViewerView: View {
    let store: FilesStore
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let path = store.viewing {
            VStack(spacing: 0) {
                FileViewerHeader(store: store, path: path)
                Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                FileViewerBody(store: store, path: path)
            }
            .background(tokens.surfaceWindow)
            .onExitCommand(perform: store.closeViewer)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(panelText("files-spoken-viewer", "\(path.split(separator: "/").last.map(String.init) ?? path), file viewer"))
        }
    }
}

struct FileViewerHeader: View {
    let store: FilesStore
    let path: String
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        let name = path.split(separator: "/").last.map(String.init) ?? path
        let folder = path.contains("/") ? String(path[..<path.lastIndex(of: "/")!]) : ""
        HStack(spacing: PanelMetrics.gap) {
            FileIcon(kind: .file(name), tint: tokens.textMuted)
            Text(name).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).lineLimit(1).truncationMode(.middle)
            if !folder.isEmpty { Text(folder).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.head) }
            Spacer(minLength: 0)
            if let size = store.content.size { Text(ByteSize.text(size)).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize() }
            if let editor = host?.editorName {
                BandButton(symbol: "arrow.up.forward.square", label: panelText("files-viewer-open-in-editor", "Open in \(editor)"), inRow: true) { host?.open(absolute, line: nil) }
            }
            BandButton(symbol: "folder", label: panelText("files-row-reveal", "Reveal in Finder"), inRow: true) { host?.reveal(absolute) }
            BandButton(symbol: "doc.on.doc", label: panelText("files-row-copy-path", "Copy Path"), inRow: true) { host?.copyPath(absolute) }
            BandButton(symbol: "xmark", label: panelText("files-viewer-close", "Close file (esc)"), inRow: true, action: store.closeViewer)
        }
        .padding(.horizontal, PanelMetrics.inset).frame(height: 40)
    }

    private var absolute: String { store.root.map { store.join($0, path) } ?? path }
}
