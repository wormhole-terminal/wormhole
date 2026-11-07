// A row's second line: what the terminal has to report, else where it is, and the tone it is set in. A port of
// `second_line` and `agent_detail` (`sidebar/row/second_line.rs`, `wording.rs`); the model carries one `subtitle`
// for the agent's own words, the command in front or the folder, so it plays all three.

import Foundation
import WorkspaceModel

enum RowTone: Sendable, Equatable { case plain, attention, done, failed }

struct RowLine: Sendable, Equatable {
    var text: String?
    var tone: RowTone
    /// Faint, at the line's right: "new output".
    var trail: String?

    /// A compact or dense row has no second line; a regular one always has, so no row is a single line.
    static func of(_ terminal: TerminalSummary, twoLines: Bool) -> RowLine {
        guard twoLines else { return RowLine(text: nil, tone: .plain, trail: nil) }
        let message = terminal.subtitle?.trimmingCharacters(in: .whitespaces).nilIfEmpty
        switch terminal.activity {
        case .needsInput: return RowLine(text: "Needs input" + (message.map { " · \($0)" } ?? ""), tone: .attention, trail: nil)
        case .working: return RowLine(text: message ?? "Working", tone: .plain, trail: nil)
        case .finished: return RowLine(text: "Finished" + (message.map { " · \($0)" } ?? ""), tone: .done, trail: nil)
        case .failed: return RowLine(text: "Failed" + (message.map { " · \($0)" } ?? ""), tone: .failed, trail: nil)
        case .idle: return RowLine(text: message ?? "Shell", tone: .plain, trail: terminal.unread > 0 ? "new output" : nil)
        }
    }
}

private extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}
