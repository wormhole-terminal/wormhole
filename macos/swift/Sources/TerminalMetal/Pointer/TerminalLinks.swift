// Links: with Command held the link under the pointer (a program's OSC 8 hyperlink or a URL in the text) is
// underlined and the pointer becomes a hand; Command-click opens it, also in a program that takes the mouse.
// What may be opened is the seam's `openTarget`; the system opens it.

import AppKit
import AppLogging
import DesignSystem
import TerminalCore
import WordsModel

/// How a link is opened; the system's by default, replaced in tests.
public enum LinkOpener {
    @MainActor public static func system(_ target: OpenTarget) {
        switch target {
        case .url(let string):
            if let url = URL(string: string) { ExternalOpen.open(url) }
        case .reveal(let path):
            ExternalOpen.reveal([URL(fileURLWithPath: path)])
        case .refused:
            break
        }
    }
}

extension MetalTerminalView {
    /// The pointer moved, or Command went down or up: underline the link under it while Command is held.
    func updateLink(under hit: CellHit?, command: Bool) {
        let link = (command ? hit : nil).flatMap { interaction?.link(row: $0.row, column: $0.column) }
        guard link != pointer.link else { return }
        pointer.link = link
        onLinkReadout?(link?.readout)
        renderer.setLinkHighlight(link?.spans ?? [])
        window?.invalidateCursorRects(for: self)
        requestRedraw()
    }

    /// Command-click: open the link at `hit`. `false` when there is none, so the click does what it would.
    func openLink(at hit: CellHit) -> Bool {
        guard let interaction, let link = interaction.link(row: hit.row, column: hit.column) else { return false }
        // A file a Probe's rule named is looked for in the terminal's folder by the host, never opened as an address.
        if link.isFile {
            onFileLink?(link.uri)
            return true
        }
        let target = interaction.openTarget(for: link.uri)
        if target == .refused {
            // The scheme only: the rest of a link can be private.
            AppLog.log("info", "a terminal link wasn't opened: \(link.uri.split(separator: ":").first.map(String.init) ?? "") is not a kind that is allowed")
        }
        linkOpener(target)
        return true
    }

    /// A script's pointer over cell (`row`, `column`) with Command held (and clicked): says what is there and what
    /// opening it asked the opener for, after the same underline a real hover draws. Nothing is opened.
    public func scriptedLink(row: Int, column: Int, click: Bool) -> String {
        let hit = CellHit(row: row, column: column, side: .left, x: 0, y: 0)
        updateLink(under: hit, command: true)
        guard let link = pointer.link else { return "no link at \(row),\(column)" }  // text: devctl output, not interface text
        var asked: OpenTarget?
        let opener = linkOpener
        linkOpener = { asked = $0 }
        defer { linkOpener = opener }
        if click { _ = openLink(at: hit) }
        return "link \(link.uri) explicit=\(link.isExplicit) file=\(link.isFile) readout=\(link.readout ?? "-") spans=\(link.spans.count) opens=\(asked.map { "\($0)" } ?? "not clicked")"  // text: devctl output, not interface text
    }

    /// The pointer over the grid: a hand over a link, an arrow where a program has the mouse, else the text cursor.
    public override func resetCursorRects() {
        let cursor: NSCursor = pointer.link != nil ? .pointingHand : (interaction?.takesPointer == true ? .arrow : .iBeam)
        addCursorRect(bounds, cursor: cursor)
    }
}
