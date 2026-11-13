// The programs the person let drive Wormhole through its command line, MCP or socket (Agent access): each is a row with
// the one action that fits, Revoke. Held for a session or until quit, or stored for good; the core writes the lines.

public struct ApprovalRecord: Sendable, Equatable, Identifiable {
    /// A stored approval's database row, or a held one's number: with `isStored` it names the approval to take back.
    public let number: UInt64
    public let isStored: Bool
    public let label: String
    /// How long it lasts, who signed it and when it was last used, in one line.
    public let detail: String
    /// Where it ran from, last time; empty when it was not noted.
    public let location: String
    /// What it may do, in words, one per capability.
    public let allowed: [String]
    /// The registry agent it is (`claude`), for its tile; nil for any other program.
    public let agent: String?
    /// What it may do, as the ticks of its row (closing is never one: it always asks).
    public let capabilities: Set<ApprovalCapability>

    public var id: String { "\(isStored ? "stored" : "held")-\(number)" } // text: an id

    public init(number: UInt64, isStored: Bool, label: String, detail: String, location: String = "", allowed: [String] = [], agent: String? = nil, capabilities: Set<ApprovalCapability> = []) {
        (self.number, self.isStored, self.label, self.detail, self.location, self.allowed, self.agent, self.capabilities) =
            (number, isStored, label, detail, location, allowed, agent, capabilities)
    }
}

/// One tick of an approved program's row.
public enum ApprovalCapability: String, Sendable, CaseIterable {
    case read, type, run, open
}
