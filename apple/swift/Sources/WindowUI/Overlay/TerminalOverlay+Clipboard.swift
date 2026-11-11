// A program's question about the clipboard, on the pane's bar (`terminal_view/pane_bar.rs`): the core decides whether to
// ask (one at a time, only where it can be seen) and answers the program whatever happens; this shows the question,
// announces it, and says that a program wrote to the clipboard.

import AppKit
import TerminalMetal
import TerminalUI

extension TerminalOverlay {
    func wireClipboard(view: MetalTerminalView, notices: NoticeCenter) {
        view.onProgramCopy = { [weak self, weak view] text in
            // Nothing for a terminal that is not on screen (`pane_bar.rs`).
            guard let view, view.window != nil, !view.isHiddenOrHasHiddenAncestor else { return }
            // In the terminal the person is typing in, a copy is "Copied 12 characters", as any copy; anywhere else it
            // is a program's doing, and the chip names the program.
            let typing = view.window?.firstResponder === view && view.window?.isKeyWindow == true
            let notice = Notice.copied(text: text, byProgram: !typing, program: typing ? nil : view.source.foregroundProgram())
            // With the chip off the copy is said in this terminal's own bar for a few seconds, typing in it or not.
            if notices.chipOn() { notices.post(notice) } else { self?.sayInBar(.info, notice.words) }
        }
        // Every copy of the pane's own (⌘C, Edit ▸ Copy, the context menu's) says what it copied, never the text.
        view.onCopied = { notices.post(.copied(text: $0)) }
        view.onHistoryKept = { notices.post(.failed(.historyKeptByHost)) }
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
