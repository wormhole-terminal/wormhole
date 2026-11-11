// The questions before closing a terminal, a window or a group, and before ending a session host
// (`window::session` titles and details, `window::host_close`). Each says how many terminals end, what is still
// running, and whether notes and tasks go with them (`stakes`: their sentences, from the features that hold them).

import Foundation

public enum CloseQuestions {
    public static func sessionTitle(_ name: String) -> String { AuxText.closeSessionTitle(name) }

    public static func groupTitle(_ name: String) -> String { AuxText.closeGroupTitle(name) }

    public static func session(
        agent: String?, stakes: [String], status: RunningTerminal.Status, commandRunning: Bool, programs: [String]
    ) -> String {
        let who = agent ?? AuxText.itsAgent
        var sentences: [String] = []
        switch status {
        case .busy: sentences.append(AuxText.isStillWorking(who))
        case .attention: sentences.append(AuxText.isWaiting(who))
        case .idle: break
        }
        let named = Set(programs).count
        let ending = sentences.count + named
        if let list = StillRunning.list(programs) {
            sentences.append(AuxText.stillRunning(list))
        } else if commandRunning, sentences.isEmpty {
            sentences.append(AuxText.commandRunning)
        }
        switch (sentences.isEmpty, ending) {
        case (true, _): sentences.append(AuxText.shellEnds)
        case (false, 0...1): sentences.append(AuxText.closingEndsProcess)
        case (false, _): sentences.append(AuxText.closingEndsThem)
        }
        sentences += stakes.map(AuxText.itsStake)
        return sentences.joined(separator: " ")
    }

    public static func window(terminals: Int, running: [StillRunning.Fact], programs: [String], stakes: [String]) -> String {
        let head = terminals == 1 ? AuxText.terminalInWindowEnds : AuxText.terminalsInWindowEnd(terminals)
        return head + StillRunning.detail(running, programs: programs) + stakes.map { " " + AuxText.theirStake($0) }.joined()
    }

    public static func group(terminals: Int, running: [StillRunning.Fact], programs: [String], stakes: [String]) -> String {
        let head = terminals == 1 ? AuxText.terminalInGroupEnds : AuxText.terminalsInGroupEnd(terminals)
        return head + StillRunning.detail(running, programs: programs) + stakes.map { " " + AuxText.theirStake($0) }.joined()
    }

    /// Closing a terminal whose session host runs but cannot be shown: end the host with the terminal, or leave it
    /// running (it stays in the Session Hosts window).
    public static func unavailableHost(name: String) -> Question {
        Question(
            title: sessionTitle(name), detail: AuxText.hostCloseDetail, confirm: AuxText.hostCloseEnd,
            alsoButtons: [AuxText.hostCloseLeave])
    }

    /// Ending a session host from its window.
    public static func endHost(id: String, attachedTitle: String?) -> Question {
        Question(
            title: AuxText.hostEndTitle(id),
            detail: attachedTitle.map(AuxText.hostEndDetailAttached) ?? AuxText.hostEndDetail,
            confirm: AuxText.hostEndConfirm)
    }
}
