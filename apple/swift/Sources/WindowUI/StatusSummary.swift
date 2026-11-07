// What the status bar counts, ported from `view::layout::status_bar` (`Summary`, `Detail`): the terminal that
// needs you first (its own button), then the rest as counts in the bar's order, and how much of it fits the
// window's width. The model does not time how long a terminal has waited, so the button has no "2m" yet.

import CoreGraphics
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
            case .working: "\(number) working"
            case .needsInput: number == 1 ? "1 needs input" : "\(number) need input"
            case .finished: "\(number) done"
            case .failed: "\(number) failed"
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

    init(_ terminals: [TerminalSummary]) {
        let waiting = terminals.filter { $0.activity == .needsInput }
        attention = waiting.first
        rest = [(Activity.needsInput, Array(waiting.dropFirst())),
            (.working, terminals.filter { $0.activity == .working }),
            (.finished, terminals.filter { $0.activity == .finished }),
            (.failed, terminals.filter { $0.activity == .failed })]
            .compactMap { activity, group in group.first.map { Count(activity: activity, number: group.count, first: $0.id) } }
    }

    var isEmpty: Bool { attention == nil && rest.isEmpty }
}
