// The quick panel's state (`status_item/quick_panel/view.rs`): what it lists, what is selected, and Reply in
// place. Pure state and the commands it sends; the panel and its view only show it. Rows are found again by what
// they are (rows come and go as terminals change), never by position.

import Foundation
import Observation
import PlatformModel

@MainActor @Observable
public final class QuickPanelModel {
    /// The tallest the panel gets before its list scrolls (`MAX_H`).
    static let maxHeight: Double = 560
    /// The answer region under a waiting row: the question, the reply field and its hints (`ANSWER_H`).
    static let answerHeight: Double = 112

    public struct Answering: Equatable {
        public var tab: Int64
        public var question: String
    }

    public var terminals: [AmbientTerminal] = []
    public var items: [QuickPanelDo] = []
    /// What the times are measured to: set when the terminals are read.
    public var now = Date()
    public var query = "" { didSet { if query != oldValue { selected = nil } } }
    public var reply = ""
    public private(set) var answering: Answering?
    /// The hotkey, drawn as a key cap at the search row's right.
    public var hotkey: String?
    private var selected: String?
    /// What choosing a row or sending a reply asks of the app, after the panel has closed.
    public var send: (AmbientCommand) -> Void = { _ in }
    public var close: () -> Void = {}

    public init() {}

    public var rows: [QuickPanelRow] { QuickPanelRows.rows(terminals, items, query: query) }

    /// The selected row's index: the one chosen, else the first that can be.
    public var selectedIndex: Int? {
        let rows = rows
        return rows.firstIndex { $0.id == selected && $0.selectable } ?? rows.firstIndex(where: \.selectable)
    }

    public var selectedRow: QuickPanelRow? { selectedIndex.map { rows[$0] } }

    public func select(_ row: QuickPanelRow) { if row.selectable { selected = row.id } }

    public func step(_ delta: Int) {
        guard answering == nil else { return }
        let rows = rows
        let choices = rows.indices.filter { rows[$0].selectable }
        guard let current = selectedIndex.flatMap(choices.firstIndex(of:)), !choices.isEmpty else { return }
        selected = rows[choices[min(max(current + delta, 0), choices.count - 1)]].id
    }

    /// ⏎ or a click: carry out the selected row after closing.
    public func run(_ row: QuickPanelRow? = nil) {
        guard let command = (row ?? selectedRow)?.command else { return }
        close()
        send(command)
    }

    /// ⌘⏎ on a waiting row: Reply in place. While answering it opens the terminal instead.
    public func commandReturn() {
        if let answering {
            close()
            send(.openTab(answering.tab))
        } else if let terminal = selectedRow?.waiting {
            reply = ""
            answering = Answering(tab: terminal.tab, question: terminal.message ?? "Waiting for your answer")
        }
    }

    public func stopAnswering() { answering = nil }

    public enum Key: Sendable { case up, down, enter, escape, commandReturn }

    /// The panel's keys (the view and the control folder's `quick-key` both come here): ↑↓ move, ⏎ runs the row or sends the
    /// reply, ⌘⏎ answers in place (or opens the terminal while answering), Esc steps back out of Reply, then closes.
    public func press(_ key: Key) {
        switch key {
        case .escape: if answering == nil { close() } else { stopAnswering() }
        case .up: step(-1)
        case .down: step(1)
        case .enter: if answering == nil { run() } else { sendReply() }
        case .commandReturn: commandReturn()
        }
    }

    /// ⏎ in Reply: type the answer into the terminal and press Return, then go on to the next waiting terminal.
    public func sendReply() {
        guard let answering, !reply.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        send(.reply(tab: answering.tab, text: reply))
        let waiting = rows.compactMap(\.waiting).map(\.tab)
        let next = waiting.drop { $0 != answering.tab }.dropFirst().first ?? waiting.first
        selected = next.flatMap { $0 == answering.tab ? nil : "terminal.\($0)" }
        self.answering = nil
        reply = ""
    }

    /// The window's height for these rows: search, rows, footer; the window is the panel.
    public var height: Double {
        let list = rows.reduce(0.0) { sum, row in
            switch row {
            case .label: sum + Double(QuickPanelMetrics.labelHeight)
            case .note, .terminal, .action: sum + Double(QuickPanelMetrics.rowHeight)
            }
        }
        let extra = answering == nil ? 0 : Self.answerHeight
        let chrome = Double(QuickPanelMetrics.searchHeight + QuickPanelMetrics.listTop + QuickPanelMetrics.rowHeight)
        return min(chrome + list + extra, Self.maxHeight)
    }

    public var footerHints: [(keys: String, label: String)] {
        answering == nil
            ? [("↑↓", "select"), ("⏎", "open"), ("⌘⏎", "answer here"), ("esc", "close")]
            : [("⏎", "send reply"), ("⌘⏎", "open terminal"), ("esc", "back")]
    }
}
