// The question before quitting (`window::quit_question`): a plain Quit and Cancel, or with the box that keeps the
// terminals running in the background (persistent terminals), and what it says from the facts of the terminals
// that run something. "Quit and End All Terminals" has no choice to offer, so it asks plain.

import Foundation

public enum QuitAsk: Equatable, Sendable {
    case plain
    case keepBox(localRunning: Bool)
}

public enum QuitQuestion {
    /// `nil`: nothing runs, so no question.
    public static func ask(endHosts: Bool, hostedRunning: Int, localRunning: Int, anyHosted: Bool) -> QuitAsk? {
        guard hostedRunning + localRunning > 0 else { return nil }
        return endHosts || !anyHosted ? .plain : .keepBox(localRunning: localRunning > 0)
    }

    /// The box ticked keeps the hosts; unticked ends them.
    public static func endsHosts(keepRunning: Bool) -> Bool { !keepRunning }

    /// What the question says. Said from the counts: what is kept goes on, and what runs inside the app ends,
    /// however the box is.
    public static func detail(
        running: [StillRunning.Fact], programs: [String], otherProfiles: [(name: String, count: Int)] = [],
        hostedRunning: Int = 0, localRunning: Int = 0
    ) -> String {
        let head: String
        if hostedRunning == 0 {
            head = AuxText.quitEnds
        } else if localRunning == 0 {
            head = AuxText.quitKeepsAll
        } else {
            head = AuxText.quitKeepsSome(kept: hostedRunning, ended: localRunning)
        }
        let elsewhere = otherProfiles.map { " " + AuxText.quitOtherProfile(count: $0.count, profile: $0.name) }.joined()
        return head + StillRunning.detail(running, programs: programs) + elsewhere
    }

    /// The question, with the box when `ask` says so.
    public static func question(ask: QuitAsk, detail: String, endAll: Bool = false) -> Question {
        let confirm = endAll ? AuxText.quitAndEndAll : AuxText.quit
        let title = endAll ? AuxText.quitEndAllTitle : AuxText.quitTitle
        switch ask {
        case .plain: return Question(title: title, detail: detail, confirm: confirm)
        case .keepBox:
            return Question(
                title: title, detail: detail, confirm: confirm, checkbox: .init(label: AuxText.keepRunning, checked: true))
        }
    }
}
