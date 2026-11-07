// What the viewer's body is for each kind of file (`viewer/card.rs`): text as numbered lines in a virtual list, a picture
// scaled to fit, a binary file's panel with Open in…, or the sentence that says why there is nothing to show.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

struct FileViewerBody: View {
    let store: FilesStore
    let path: String
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        switch store.content {
        case .reading: centred(panelText("files-reading", "Reading\u{2026}"))
        case let .text(text, size, truncated):
            if text.isEmpty { centred(panelText("files-viewer-empty", "This file is empty.")) } else { lines(truncated: truncated, size: size) }
        case let .binary(size): binary(size)
        case let .image(data, _): picture(data)
        case let .unavailable(problem): centred(Self.words(problem))
        }
    }

    private func lines(truncated: Bool, size: Int) -> some View {
        VStack(spacing: 0) {
            FileLines(lines: store.lines, spans: store.spans, path: path)
            if truncated {
                Text(panelText("files-viewer-truncated", "Showing the first \(ByteSize.text(store.lines.reduce(0) { $0 + $1.utf8.count + 1 })) of \(ByteSize.text(size))"))
                    .font(TextRole.caption.font).foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity).frame(height: Metric.row)
                    .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
            }
        }
    }

    private func binary(_ size: Int) -> some View {
        VStack(spacing: PanelMetrics.gap) {
            Text(panelText("files-viewer-binary", "Binary file, \(ByteSize.text(size))")).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            NoticeButton(title: panelText("files-viewer-open-in", "Open in\u{2026}")) { host?.open(store.root.map { store.join($0, path) } ?? path, line: nil) }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder private func picture(_ data: Data) -> some View {
        if let image = NSImage(data: data) {
            Image(nsImage: image).resizable().scaledToFit().padding(Space._4).frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityLabel(path)
        } else {
            centred(panelText("files-viewer-image-failed", "This picture can't be shown."))
        }
    }

    private func centred(_ text: String) -> some View {
        Text(text).font(TextRole.body.font).foregroundStyle(tokens.textMuted).multilineTextAlignment(.center).padding(Space._4)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    static func words(_ problem: FileContent.Problem) -> String {
        switch problem {
        case .gone: panelText("files-viewer-gone", "This file was deleted.")
        case .outside: panelText("files-viewer-outside", "This is a link that leads out of the folder, so it isn't shown.")
        case .notAFile: panelText("files-viewer-not-file", "This isn't a regular file.")
        case .denied: panelText("files-viewer-denied", "Wormhole isn't allowed to read this file.")
        case .other: panelText("files-viewer-unreadable", "This file can't be read.")
        }
    }
}

/// The text's lines with their numbers, the way the diff draws its rows: tabs at stops of four, colours from the spans.
struct FileLines: View {
    let lines: [String]
    let spans: [[SyntaxSpan]]
    let path: String
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    var body: some View {
        let style = DiffStyle(code: codeFont, digits: String(lines.count).count, wrap: false, advance: codeFont.advance)
        let code = DiffCode(tokens: tokens)
        ScrollView([.vertical, .horizontal]) {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                    HStack(spacing: 0) {
                        Text("\(index + 1)").foregroundStyle(tokens.diffLineNumber).padding(.trailing, Space._2)
                            .frame(width: style.gutter, height: style.rowHeight, alignment: .trailing)
                        Text(code.text(line, spans: index < spans.count ? spans[index] : [])).fixedSize()
                            .padding(.trailing, Space._6)
                    }
                    .frame(height: style.rowHeight)
                }
            }
            .font(codeFont.font())
        }
    }
}
