// What the status bar shows of the core's counts (`view::layout::status_bar`: `StatusRules`): the terminal that needs you
// first (its own button), then the rest as counts in the bar's order, and how much of it fits the window's width. The
// rules are the core's; the words, the localized tooltip and the glyphs are this window's.

import CoreGraphics
import WordsModel
import WorkspaceModel

struct StatusSummary: Equatable {
    struct Count: Equatable, Identifiable {
        var activity: Activity
        var number: Int
        var first: TerminalID
        var id: Activity { activity }

        /// "2 working", "1 done", "2 need input": all of it in words.
        var words: String {
            switch activity {
            case .working: Words.text("window-status-working", ["count": .whole(number)], fallback: "\(number) working")
            case .needsInput:
                Words.text(
                    "window-status-need-input", ["count": .whole(number)],
                    fallback: number == 1 ? "1 needs input" : "\(number) need input")
            case .finished: Words.text("window-status-done", ["count": .whole(number)], fallback: "\(number) done")
            case .failed: Words.text("window-status-failed", ["count": .whole(number)], fallback: "\(number) failed")
            case .idle, .exited: "\(number)"
            }
        }
    }

    /// How much the bar says at a window's width (the core's rule).
    typealias Detail = StatusBarDetail

    /// The terminal the button is for, and the counts after it.
    var attention: TerminalSummary?
    var rest: [Count]
    /// Who the tooltip names, longest waiting first, and how many more need input (the core's split, `needs_you_named`).
    var waitingNames: [String] = []
    var waitingMore = 0

    /// "a, then b, c and 2 more · go to the first" (core `needs_you_tooltip`), with the jump key (the keymap's now) after it.
    /// Who is named and how many are counted is the core's; the words are the catalog's.
    func attentionTooltip(jumpKey: String?) -> String? {
        guard let first = waitingNames.first else { return nil }
        let others = waitingNames.dropFirst()
        var names = first
        if !others.isEmpty {
            let rest = others.joined(separator: ", ")
            names = Words.text("window-attention-then", ["first": .text(first), "rest": .text(rest)], fallback: "\(first), then \(rest)")
        }
        if waitingMore > 0 {
            let more = waitingMore
            names = Words.text("window-attention-more", ["names": .text(names), "count": .whole(more)], fallback: "\(names) and \(more) more")
        }
        let go =
            others.isEmpty
            ? Words.text("window-attention-go-there", ["names": .text(names)], fallback: "\(names) · go there")
            : Words.text("window-attention-go-first", ["names": .text(names)], fallback: "\(names) · go to the first")
        // The key the jump has now (`status_bar.rs`); none when it has none.
        return jumpKey.map { go + " (\($0))" } ?? go
    }

    init(_ terminals: [TerminalSummary]) {
        // The core says who waits longest and what is counted; the words are this window's.
        let counts = StatusRules.rules.counts(of: terminals)
        let waiting = counts.needsYou.compactMap { terminals.indices.contains($0) ? terminals[$0] : nil }
        attention = waiting.first
        waitingNames = waiting.prefix(counts.named).map(\.title)
        waitingMore = counts.more
        rest = counts.rest.compactMap { item in
            terminals.indices.contains(item.terminal)
                ? Count(activity: item.activity, number: item.count, first: terminals[item.terminal].id) : nil
        }
    }

    var isEmpty: Bool { attention == nil && rest.isEmpty }

    /// What a click on each item goes to, in the bar's order (the keys walk them).
    var items: [TerminalID] { (attention.map { [$0.id] } ?? []) + rest.map(\.first) }
}

extension StatusBarDetail {
    init(windowWidth: CGFloat) { self = StatusRules.rules.detail(windowWidth: Double(windowWidth)) }

    var words: Bool { self == .words }
    var size: Bool { self >= .numbers }
}
