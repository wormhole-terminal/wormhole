// Probes in the window: Edit ▸ Show Hidden Lines, and the files a Probe offers or a rule's link names. Rust decides what
// may be folded and where a path is (`probes/terminal.rs`); this relays the gesture and puts the answer on screen: the
// editor Open in… names opens the file, or a chip says it is not in the terminal's folder.

import Foundation
import TerminalCore
import TerminalUI
import WorkspaceModel

extension WindowHub {
    /// Edit ▸ Show Hidden Lines: every fold of the selected terminal opens.
    public func showHiddenLines() { selectedOverlay?.showHiddenLines() }

    /// A Probe's hook offers `text` (`path[:line]`) in terminal `terminal`'s folder: a chip says so, and a click opens it. Nothing
    /// opens by itself: an event can fire with nobody there.
    public func offerFile(terminal: TerminalID, text: String, words: String) {
        notices.post(.warned(.probeOffer(terminal: terminal, text: text, words: words)))
    }

    /// A hook was paused for failing too often.
    public func probePaused(words: String) { notices.post(.warned(.probePaused(words: words))) }

    /// A click on a chip that offers something: the file a Probe offered.
    func noticeActed(_ notice: Notice) {
        guard case let .warned(.probeOffer(terminal, text, _)) = notice else { return }
        openFile(text, in: terminal)
        notices.dismiss()
    }

    /// Look for `text` in `terminal`'s folder off the main thread (Rust reads the disk, link-safe), then open it in the editor or say
    /// it was not found.
    func openFile(_ text: String, in terminal: TerminalID) {
        guard let probes = terminals.existing(terminal)?.source.probes else { return }
        Task { [weak self] in
            let found = await Task.detached { probes.resolvePath(text) }.value
            guard let self else { return }
            if let path = found.path {
                openIn.open(path, in: openIn.choice().app)
            } else if let words = found.words {
                notices.post(.failed(.fileNotFound(words: words)))
            }
        }
    }
}
