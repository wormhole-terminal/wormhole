// What the status bar counts, ported from `view::layout::status_bar` (`Summary`, `Detail`): the terminal that
// needs you first (its own button), then the rest as counts in the bar's order, and how much of it fits the
// window's width. The model does not time how long a terminal has waited, so the button has no "2m" yet.

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

    /// How much the bar says at a window's width: the words go below `wordsMin`, the size below `sizeMin`.
    enum Detail: Comparable {
        case lean, numbers, words

        static let wordsMin: CGFloat = 1100
        static let sizeMin: CGFloat = 720

        init(windowWidth: CGFloat) {
            self = windowWidth < Self.sizeMin ? .lean : windowWidth < Self.wordsMin ? .numbers : .words
        }

        var words: Bool { self == .words }
        var size: Bool { self >= .numbers }
    }

    /// The terminal the button is for, and the counts after it.
    var attention: TerminalSummary?
    var rest: [Count]
    /// Who needs input, longest waiting first.
    var waitingNames: [String] = []

    /// "a, then b, c and 2 more · go to the first" (core `needs_you_tooltip`), with the jump key (the keymap's now) after it.
    func attentionTooltip(jumpKey: String?) -> String? {
        guard let first = waitingNames.first else { return nil }
        let others = waitingNames.dropFirst().prefix(2)
        var names = first
        if !others.isEmpty {
            let rest = others.joined(separator: ", ")
            names = Words.text("window-attention-then", ["first": .text(first), "rest": .text(rest)], fallback: "\(first), then \(rest)")
        }
        if waitingNames.count > 3 {
            let more = waitingNames.count - 3
            names = Words.text("window-attention-more", ["names": .text(names), "count": .whole(more)], fallback: "\(names) and \(more) more")
        }
        let go =
            waitingNames.count == 1
            ? Words.text("window-attention-go-there", ["names": .text(names)], fallback: "\(names) · go there")
            : Words.text("window-attention-go-first", ["names": .text(names)], fallback: "\(names) · go to the first")
        // The key the jump has now (`status_bar.rs`); none when it has none.
        return jumpKey.map { go + " (\($0))" } ?? go
    }

    init(_ terminals: [TerminalSummary]) {
        let waiting = terminals.filter { $0.activity == .needsInput }
        // The one that has waited longest (core `Summary::of`: stable, so equal starts keep the list's order).
        let longest = waiting.enumerated().sorted { ($0.element.since ?? .max, $0.offset) < ($1.element.since ?? .max, $1.offset) }.map(\.element)
        attention = longest.first
        waitingNames = longest.map(\.title)
        rest = [(Activity.needsInput, Array(longest.dropFirst())),
            (.working, terminals.filter { $0.activity == .working }),
            (.finished, terminals.filter { $0.activity == .finished }),
            (.failed, terminals.filter { $0.activity == .failed })]
            .compactMap { activity, group in group.first.map { Count(activity: activity, number: group.count, first: $0.id) } }
    }

    var isEmpty: Bool { attention == nil && rest.isEmpty }

    /// What a click on each item goes to, in the bar's order (the keys walk them).
    var items: [TerminalID] { (attention.map { [$0.id] } ?? []) + rest.map(\.first) }
}
