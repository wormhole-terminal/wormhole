// Where the Notices feature is off, the notice chip is not drawn and what it would have said goes where it always could: a
// program's copy and a refused Put at Prompt in the pane's bar for a few seconds, a warning about the app as the banner over the
// pane until it is dismissed (`terminal_view/pane_bar.rs`, `hosted.rs`). A script driving a terminal, and a chip's own offers
// (an update ready, a file a Probe names), say nothing then.

import TerminalUI

extension WindowHub {
    func sayInBar(_ notice: Notice, detail: String?) {
        guard let overlay = selectedOverlay else { return }
        switch notice {
        case .copied(.programLines), .copied(.programCharacters): overlay.sayInBar(.info, notice.words)
        case .failed(.putAtPrompt):
            overlay.sayInBar(.warning, notice.words)
            TerminalOverlay.announce(notice.words)
        case .warned(.hostFallback), .warned(.dataFolder), .warned(.moveToApplications):
            overlay.bars.hostNotice = detail ?? notice.words
        default: break
        }
    }
}

extension WindowHub {
    /// What the selected terminal's pane says in its bar and its banner, for the control folder's `chip`.
    public var selectedBarText: String? {
        if case let .notice(_, text)? = selectedOverlay?.bars.bar { text } else { nil }
    }

    public var selectedHostNotice: String? { selectedOverlay?.bars.hostNotice }
}
