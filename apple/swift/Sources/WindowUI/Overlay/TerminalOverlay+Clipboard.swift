// A program's question about the clipboard, on the pane's bar (`terminal_view/pane_bar.rs`): the core decides whether to
// ask (one at a time, only where it can be seen) and answers the program whatever happens; this shows the question,
// announces it, and says that a program wrote to the clipboard.

import AppKit
import TerminalMetal
import TerminalUI

extension TerminalOverlay {
    func wireClipboard(view: MetalTerminalView, notices: NoticeCenter) {
        view.onProgramCopy = { [weak view] text in
            // In the terminal the person is typing in, a copy is "Copied 12 characters", as any copy; anywhere else it
            // is a program's doing, and the chip says so.
            let typing = view?.window?.firstResponder === view && view?.window?.isKeyWindow == true
            notices.post(.copied(text: text, byProgram: !typing))
        }
        view.onClipboardAsk = { [weak self] program in
            let bar = PaneBar.asking(program)
            self?.bars.bar = bar
            if let words = bar.askWords { Self.announce(words) }
        }
        view.onClipboardAskClosed = { [weak self] in self?.bars.bar = nil; self?.askEnded() }
        actions.onAnswer = { [weak self, weak view] choice in
            view?.answerClipboard(choice)
            self?.bars.bar = nil
            self?.askEnded()
        }
    }
}
