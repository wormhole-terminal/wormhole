// A program's question about the clipboard, on the pane's bar (`terminal_view/pane_bar.rs`): the core decides whether to
// ask (one at a time, only where it can be seen) and answers the program whatever happens; this shows the question,
// announces it, and says that a program wrote to the clipboard.

import AppKit
import TerminalMetal
import TerminalUI

extension TerminalOverlay {
    func wireClipboard(view: MetalTerminalView, notices: NoticeCenter) {
        view.onProgramCopy = { [weak self, weak view] text in
            // A terminal that is not on screen gets no chip (`pane_bar.rs:86`).
            guard let view, view.window != nil else { return }
            let program = view.source.foregroundProgram()
            guard notices.chipOn() else {
                // The chip is off: a program silently writing to the clipboard is worth seeing, in this terminal's own bar.
                self?.sayInBar(.info, Notice.copied(text: text, byProgram: true, program: program).words)
                return
            }
            // In the terminal the person is typing in, a copy is "Copied 12 characters", as any copy; anywhere else it
            // is a program's doing, and the chip names the program.
            let typing = view.window?.firstResponder === view && view.window?.isKeyWindow == true
            notices.post(.copied(text: text, byProgram: !typing, program: program))
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
