// A program that controls a terminal says so on the chip (`agent_access::announce`): who, and whether it types or reads.

import TerminalUI
import WorkspaceModel

extension WindowHub {
    /// Only the terminal on screen has a chip to say it on: a message about another is dropped, never queued, since the
    /// person could not tell which terminal it is about.
    public func driven(terminal: TerminalID, typing: Bool, agent: String?) {
        guard state.selected == terminal else { return }
        let driver: Notice.Driver = agent.map(Notice.Driver.agent) ?? .scripts
        notices.post(.driven(typing ? .typing(driver) : .reading(driver)))
    }
}
