// Where the Notices feature is off, the notice chip is not drawn and what it would have said goes where it always could: a
// program's copy and a refused Put at Prompt in the pane's bar for a few seconds, a warning about the app as the banner over the
// pane until it is dismissed (`terminal_view/pane_bar.rs`, `hosted.rs`). A script driving a terminal, and a chip's own offers
// (an update ready, a file a Probe names), say nothing then.

import TerminalUI

extension WindowHub {
    func sayInBar(_ notice: Notice, detail: String?) {
        guard let overlay = selectedOverlay else { return }
        if overlay.sayInBar(notice) { return }
        switch notice {
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

extension TerminalOverlay {
    /// The notices said in the terminal they are about, in its own bar (`say_in_bar`); false for any other kind.
    func sayInBar(_ notice: Notice) -> Bool {
        switch notice {
        case .copied(.programLines), .copied(.programCharacters), .copied(.namedLines), .copied(.namedCharacters):
            sayInBar(.info, notice.words)
        case .failed(.putAtPrompt):
            sayInBar(.warning, notice.words)
            Self.announce(notice.words)
        default: return false
        }
        return true
    }
}
