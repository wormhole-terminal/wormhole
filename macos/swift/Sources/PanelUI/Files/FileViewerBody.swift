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

    var body: some View {
        switch store.content {
        case .reading: note(panelText("files-reading", "Reading\u{2026}"))
        case let .text(text, size, truncated):
            if text.isEmpty { note(panelText("files-viewer-empty", "This file is empty.")) } else { lines(truncated: truncated, size: size) }
        case let .binary(size): panel(panelText("files-viewer-binary", ["size": .text(ByteSize.text(size))], "Binary file, \(ByteSize.text(size))"), actions: true)
        case .image: picture
        // A deleted file has nothing to open or show, nor has a link that leads out; the others might still be revealed.
        case let .unavailable(problem): panel(Self.words(problem), actions: problem != .gone && problem != .outside)
        }
    }

    private func lines(truncated: Bool, size: Int) -> some View {
        VStack(spacing: 0) {
            FileLines(lines: store.lines, spans: store.spans, path: path)
            if truncated {
                let shown = ByteSize.text(store.lines.reduce(0) { $0 + $1.utf8.count + 1 })
                Text(panelText("files-viewer-truncated", ["shown": .text(shown), "size": .text(ByteSize.text(size))], "Showing the first \(shown) of \(ByteSize.text(size))"))
                    .font(TextRole.caption.font).foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity).frame(height: Metric.row)
                    .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
            }
        }
    }

    /// A note in the middle of the card, with Open in… and Reveal under it when they have something to act on.
    private func panel(_ words: String, actions: Bool) -> some View {
        VStack(spacing: Space._3) {
            PanelIcon("doc", size: Metric.glyph).foregroundStyle(tokens.textMuted)
            Text(words).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).multilineTextAlignment(.center)
            if actions {
                HStack(spacing: Space._2) {
                    NoticeButton(title: panelText("files-viewer-open-in", "Open in\u{2026}"), primary: true) { store.openViewed() }
                    NoticeButton(title: panelText("files-row-reveal", "Reveal in Finder")) { store.revealViewed() }
                }
            }
        }
        .padding(Space._4).frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder private var picture: some View {
        switch store.picture {
        case nil: note(panelText("files-reading", "Reading\u{2026}"))
        case let .shown(image):
            Image(decorative: image, scale: 1).resizable().scaledToFit().padding(Space._4).frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityLabel(path)
        case .tooLarge: panel(panelText("files-viewer-image-too-large", "This picture is too large to show."), actions: true)
        case .unreadable: panel(panelText("files-viewer-image-failed", "This picture can't be shown."), actions: true)
        }
    }

    /// A line of words in the middle of the card (reading, an empty file).
    private func note(_ text: String) -> some View {
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

/// The text's lines with their numbers, the way the diff draws its rows: tabs at the tab width, colours from the spans.
struct FileLines: View {
    let lines: [String]
    let spans: [[SyntaxSpan]]
    let path: String
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont
    @Environment(\.tabWidth) private var tabWidth
    /// How far the code is scrolled sideways: the numbers are moved back by as much, so they stay put (`card.rs`).
    @State private var across: CGFloat = 0

    var body: some View {
        let style = DiffStyle(code: codeFont, digits: String(lines.count).count, wrap: false, advance: codeFont.advance)
        let code = DiffCode(tokens: tokens, tabWidth: tabWidth)
        ScrollView([.vertical, .horizontal]) {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                    HStack(spacing: 0) {
                        Text("\(index + 1)").foregroundStyle(tokens.diffLineNumber).padding(.trailing, Space._2)
                            .frame(width: style.gutter, height: style.rowHeight, alignment: .trailing)
                            .background(tokens.surfaceWindow).offset(x: across).zIndex(1)
                        Text(code.text(line, spans: index < spans.count ? spans[index] : [])).fixedSize()
                            .padding(.trailing, Space._6)
                    }
                    .frame(height: style.rowHeight)
                }
            }
            .font(codeFont.font())
        }
        .defaultScrollAnchor(.topLeading)
        .onScrollGeometryChange(for: CGFloat.self) { max($0.contentOffset.x, 0) } action: { _, now in across = now }
    }
}
