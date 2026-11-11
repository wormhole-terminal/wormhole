// The Extensions page's values (Probes: folders that add to Wormhole) as the core writes them, and the seam the page
// reads and acts through. A Probe's words are written by whoever made the folder; the core has already made them safe
// to show. The sentences (states, notes, permissions) arrive in the language in use.

public struct ProbeRow: Sendable, Equatable, Identifiable {
    public enum State: String, Sendable { case on, off, changed, unsupported, invalid }
    /// The Probe's id, or its folder's name when its manifest does not read: what every call takes.
    public let id: String
    public let title: String
    /// "id · version · author".
    public let identity: String
    public let description: String
    public let state: State
    public let stateWord: String
    /// A stored approval that is not this Wormhole's: off, in a warning's colour.
    public let unsealed: Bool
    /// What is wrong with a changed, unsupported or unsealed Probe, and why an invalid one is.
    public let note: String?
    public let canApprove: Bool
    public let canForget: Bool

    public init(
        id: String, title: String, identity: String = "", description: String = "", state: State, stateWord: String,
        unsealed: Bool = false, note: String? = nil, canApprove: Bool = false, canForget: Bool = false
    ) {
        (self.id, self.title, self.identity, self.description, self.state, self.stateWord) =
            (id, title, identity, description, state, stateWord)
        (self.unsealed, self.note, self.canApprove, self.canForget) = (unsealed, note, canApprove, canForget)
    }
}

public struct ProbeList: Sendable, Equatable {
    public var rows: [ProbeRow] = []
    /// Why every Probe reads as off (the approvals' key cannot be read).
    public var keyProblem: String?
    /// More folders than a list can be: none are listed.
    public var tooMany: String?
    /// The Probes folder, as a person says it, and its path.
    public var folder = ""
    public var folderPath = ""

    public init(rows: [ProbeRow] = [], keyProblem: String? = nil, tooMany: String? = nil, folder: String = "", folderPath: String = "") {
        (self.rows, self.keyProblem, self.tooMany, self.folder, self.folderPath) = (rows, keyProblem, tooMany, folder, folderPath)
    }
}

public struct ProbePermission: Sendable, Equatable, Identifiable {
    /// Its place in the request: what a tick sends back.
    public let id: Int
    public let words: String
    /// Approving with it on needs Touch ID.
    public let touchID: Bool
    public let granted: Bool
    public let programNote: String?
    public let programWarns: Bool
    public let commands: [String]

    public init(id: Int, words: String, touchID: Bool = false, granted: Bool = true, programNote: String? = nil, programWarns: Bool = false, commands: [String] = []) {
        (self.id, self.words, self.touchID, self.granted) = (id, words, touchID, granted)
        (self.programNote, self.programWarns, self.commands) = (programNote, programWarns, commands)
    }
}

public struct ProbeSheet: Sendable, Equatable, Identifiable {
    /// What changed since the last approval: a permission it did not ask for before is a warning.
    public struct Change: Sendable, Equatable { public let text: String; public let added: Bool
        public init(text: String, added: Bool) { (self.text, self.added) = (text, added) }
    }
    public let id: String
    public let title: String
    public let identity: String
    public let description: String
    public let since: [Change]
    public let asksHeading: String
    public let permissions: [ProbePermission]
    public let combinations: [String]
    public let filesFingerprint: String

    public init(
        id: String, title: String, identity: String = "", description: String = "", since: [Change] = [], asksHeading: String = "",
        permissions: [ProbePermission] = [], combinations: [String] = [], filesFingerprint: String = ""
    ) {
        (self.id, self.title, self.identity, self.description, self.since) = (id, title, identity, description, since)
        (self.asksHeading, self.permissions, self.combinations, self.filesFingerprint) = (asksHeading, permissions, combinations, filesFingerprint)
    }
}

public enum ProbeApproval: Sendable, Equatable {
    case approved
    /// Cancelled at the system's question: nothing changed, nothing is said.
    case cancelled
    case failed(String)
}

/// The core's Probes behind the page. Every call may read the approval keys or wait for the system: none runs on the
/// main thread.
public protocol ProbesBackend: Sendable {
    func list() async -> ProbeList
    /// Reads the Probe once and keeps what it read: approving records exactly what the sheet shows. Throws the sentence.
    func sheet(_ id: String) async throws -> ProbeSheet
    func needsTouchID(_ id: String, granted: Set<Int>) async -> Bool
    func approve(_ id: String, granted: Set<Int>) async -> ProbeApproval
    /// Turns it off, or forgets what is stored for it: the approval and its copy go.
    func turnOff(_ id: String) async throws
}
