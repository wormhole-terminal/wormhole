// text-file: the control folder's `files-viewer` (dev tooling), not interface text
// What the Files viewer shows, from the store `FileViewerBody` draws (`FilesStore.content`, `lines`, `spans`, `picture`): the
// path, the kind of content, the line count and the gutter's digits, the card's words and buttons, a picture's size, and the
// first lines with their colour runs. Nothing here decides. The shape is `DesignSystem.Readout`'s:
//
//     files-viewer "src/main.rs" kind="text" lines="120" digits="3" size="3.1 KB" truncated
//     note "Showing the first 1 MB of 4 MB"
//     button "Open in…"
//     picture width="4096" height="2731"
//     line 1 text="fn main() {" spans="0-2:keyword"

import CoreGraphics
import DesignSystem
import PanelModel
import WordsModel

@MainActor
public enum FilesViewerReadout {
    public static func text(_ files: FilesStore, from: Int, count: Int) -> String {
        guard let path = files.viewing else { return Readout.line("files-viewer", quoted: ["none"]) }
        var pairs: [(String, String)] = [("kind", kind(files.content))]
        var flags: [String] = []
        if case let .text(text, _, truncated) = files.content, !text.isEmpty {
            pairs += [("lines", String(files.lines.count)), ("digits", String(String(files.lines.count).count))]
            if truncated { flags.append("truncated") }
        }
        if let size = files.content.size { pairs.append(("size", ByteSize.text(size))) }
        var lines = [Readout.line("files-viewer", quoted: [path], pairs: pairs, flags: flags)]
        // The card's words and whether Open in… and Reveal are under them (`FileViewerBody.panel`).
        let (note, buttons) = FileViewerBody.card(files)
        if let note { lines.append(Readout.line("note", quoted: [note])) }
        if buttons {
            lines.append(Readout.line("button", quoted: [panelText("files-viewer-open-in", "Open in\u{2026}")]))
            lines.append(Readout.line("button", quoted: [panelText("files-row-reveal", "Reveal in Finder")]))
        }
        if case let .shown(image)? = files.picture {
            lines.append(Readout.line("picture", pairs: [("width", String(image.width)), ("height", String(image.height))]))
        }
        for index in files.lines.indices.dropFirst(max(0, from)).prefix(max(0, count)) {
            let runs = files.spans.indices.contains(index) ? files.spans[index] : []
            lines.append(
                Readout.line(
                    "line", [String(index + 1)],
                    pairs: [("text", files.lines[index]), ("spans", runs.map { "\($0.range.lowerBound)-\($0.range.upperBound):\($0.key)" }.joined(separator: ","))]))
        }
        return lines.joined(separator: "\n")
    }

    private static func kind(_ content: FileContent) -> String {
        switch content {
        case .reading: "reading"
        case .text: "text"
        case .binary: "binary"
        case .image: "image"
        case .unavailable(let problem): "unavailable-\(problem)"
        }
    }
}
