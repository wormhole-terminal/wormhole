// The tab bar's sets as the core lists them (`view::sidebar::tabs`, `Workspace.tab_sets`): every group, a folded one with
// all its terminals, then the terminals in no group; the set the selected terminal is in; each set's alert and the closed
// switcher's dot. The window draws them and words "Ungrouped" in its language.

public struct TabSetFacts: Sendable, Equatable {
    public struct Entry: Sendable, Equatable {
        /// Nil for the terminals in no group.
        public var group: GroupID?
        public var label: String
        public var members: [TerminalID]
        /// The loudest of its terminals' alerts (needs you before failed), if one has any.
        public var alert: Urgency?

        public init(group: GroupID?, label: String, members: [TerminalID], alert: Urgency?) {
            (self.group, self.label, self.members, self.alert) = (group, label, members, alert)
        }
    }

    public var sets: [Entry]
    /// The set the selected terminal is in.
    public var current: Int?
    /// The loudest alert of the sets other than the current one: the amber dot on the closed switcher.
    public var othersAlert: Urgency?

    public init(sets: [Entry], current: Int?, othersAlert: Urgency?) {
        (self.sets, self.current, self.othersAlert) = (sets, current, othersAlert)
    }
}
