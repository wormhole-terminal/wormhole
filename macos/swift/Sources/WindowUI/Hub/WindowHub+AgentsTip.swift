// The list's note after a first run without coding agents (`FirstRun::agents_note`): what stage it shows and what its buttons
// do, in one place for `NoAgentsTip` (`SideColumns`) and the control folder's `sidebar-tip`.

import SidebarUI

extension WindowHub {
    /// What the note shows: none once a terminal of the window runs an agent.
    public var agentsTipStage: AgentsTip { state.all.contains { $0.agent != nil } ? .off : agentsTip }

    /// The note's ×: it gives way to the offer of Don't show again.
    public func closeAgentsTip() { agentsTip = .offer }

    /// Don't show again: never again in this profile.
    public func neverAgentsTip() {
        actions.dismissAgentsTip()
        agentsTip = .off
    }

    /// The offer's ×: the note goes, to come back on the next first run.
    public func keepAgentsTip() { agentsTip = .off }
}
