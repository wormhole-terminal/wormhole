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
            .accessibilityLabel(panelText("files-spoken-viewer", ["name": .text(Self.name(path))], "\(Self.name(path)), file viewer"))
        }
    }
}

extension FileViewerView {
    /// The file's name as drawn (a name that is not text is a key, `DiskName`).
    static func name(_ path: String) -> String { DiskName.shown(path.split(separator: "/").last.map(String.init) ?? path) }
}

struct FileViewerHeader: View {
    let store: FilesStore
    let path: String
    /// In the terminal's title row the row gives the height and the insets.
    var inTitleRow = false
    @State private var card = PathCardHover()
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        let name = FileViewerView.name(path)
        let folders = path.contains("/") ? DiskName.shown(String(path[..<path.lastIndex(of: "/")!])) + "/" : ""
        HStack(spacing: PanelMetrics.gap) {
            FileIcon(kind: .file(name), tint: tokens.textMuted)
            // The path as the diff's crumb draws it: folders quiet, the name loud, the head elided first. A button as the
            // title row's crumb is (`title/breadcrumb.rs`): a click shows the file in Finder, resting on it opens the
            // path card, a right-click its menu.
            Button {
                card.close()
                store.revealViewed()
            } label: {
                Text("\(Text(folders).foregroundStyle(tokens.textMuted))\(Text(name).foregroundStyle(tokens.textPrimary))")
                    .font(tokens.font(.body)).lineLimit(1).truncationMode(.head)
            }
            .buttonStyle(.plain).layoutPriority(-1)
            .onHover { card.hover(.path, $0) }
            .popover(isPresented: $card.shown, arrowEdge: .bottom) {
                PathCard(path: DiskName.shown(absolute), buttons: cardButtons, hover: card)
            }
            .accessibilityLabel(panelText("files-spoken-reveal-file", ["path": .text(DiskName.shown(path))], "Reveal \(DiskName.shown(path)) in Finder"))
            .contextMenu {
                Button(panelText("files-row-reveal", "Reveal in Finder"), action: store.revealViewed)
                Button(panelText("files-row-copy-path", "Copy Path")) { host?.copyPath(absolute) }
            }
            Spacer(minLength: 0)
            if let size = store.content.size { Text(ByteSize.text(size)).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted).fixedSize() }
            // Always there: with no editor known it is Finder showing the file (core's words say which).
            if let open = host?.openTooltip(line: nil) {
                BandButton(symbol: "arrow.up.forward.square", label: open, inRow: true) { store.openViewed() }
            }
            BandButton(symbol: "xmark", label: panelText("files-viewer-close", "Close file (esc)"), inRow: true) {
                store.closeViewer()
                host?.focusTerminal()
            }
        }
        .padding(.horizontal, inTitleRow ? 0 : PanelMetrics.inset)
        .frame(height: inTitleRow ? nil : 40)
    }

    private var absolute: String { store.root.map { store.join($0, path) } ?? path }

    /// The card's three buttons, as the folder's card has them (`title/path_card.rs`), for the file. A path that is
    /// not text is not typed at a prompt, and copying it is refused by the host with the chip's words.
    private var cardButtons: [PathCardButton] {
        let absolute = absolute
        return [
            PathCardButton(id: "open", symbol: "arrow.up.forward.app", title: panelText("title-row-path-open-in-finder", "Open in Finder")) { [store] in store.revealViewed() },
            PathCardButton(id: "copy", symbol: "doc.on.doc", title: panelText("title-row-path-copy", "Copy path"), copied: panelText("title-row-path-copied", "Copied")) { [host] in host?.copyPath(absolute) },
            PathCardButton(id: "insert", symbol: "text.insert", title: panelText("title-row-path-insert", "Insert into terminal"), enabled: host != nil && !DiskName.isKey(absolute)) { [host] in host?.insertPath(absolute) },
        ]
    }
}
