// Links: with Command held the link under the pointer (a program's OSC 8 hyperlink or a URL in the text) is
// underlined and the pointer becomes a hand; Command-click opens it, also in a program that takes the mouse.
// What may be opened is the seam's `openTarget`; the system opens it.

import AppKit
import OSLog
import TerminalCore

/// How a link is opened; the system's by default, replaced in tests.
public enum LinkOpener {
    @MainActor public static func system(_ target: OpenTarget) {
        switch target {
        case .url(let string):
            if let url = URL(string: string) { NSWorkspace.shared.open(url) }
        case .reveal(let path):
            NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
        case .refused:
            break
        }
    }
}

private let log = Logger(subsystem: "app.getwormhole.terminal.swift", category: "links")

extension MetalTerminalView {
    /// The pointer moved, or Command went down or up: underline the link under it while Command is held.
    func updateLink(under hit: CellHit?, command: Bool) {
        let link = (command ? hit : nil).flatMap { interaction?.link(row: $0.row, column: $0.column) }
        guard link != pointer.link else { return }
        pointer.link = link
        renderer.setLinkHighlight(link?.spans ?? [])
        window?.invalidateCursorRects(for: self)
        requestRedraw()
    }

    /// Command-click: open the link at `hit`. `false` when there is none, so the click does what it would.
    func openLink(at hit: CellHit) -> Bool {
        guard let interaction, let link = interaction.link(row: hit.row, column: hit.column) else { return false }
        let target = interaction.openTarget(for: link.uri)
        if target == .refused {
            // The scheme only: the rest of a link can be private.
            log.info("a terminal link wasn't opened: \(link.uri.split(separator: ":").first.map(String.init) ?? "", privacy: .public) is not a kind that is allowed")
        }
        linkOpener(target)
        return true
    }

    /// The pointer over the grid: a hand over a link, an arrow where a program has the mouse, else the text cursor.
    public override func resetCursorRects() {
        let cursor: NSCursor = pointer.link != nil ? .pointingHand : (interaction?.takesPointer == true ? .arrow : .iBeam)
        addCursorRect(bounds, cursor: cursor)
    }
}
