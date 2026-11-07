// One session host as a card (`hosts_window` `card.rs`, core `ipc::hosts::report::HostRow`): how it stands, its id,
// the facts in two columns, and what can be done to it. The words of the facts (versions, protocol ranges,
// durations, sizes, link counters) are core's; a card carries them already said, with the tone to draw them in.

import Foundation

public struct HostFact: Equatable, Sendable, Identifiable {
    public enum Field: CaseIterable, Sendable {
        case hostPid, shellPid, program, memory, version, protocolRange, started, answer, link, folder, socket, meta

        public var label: String {
            switch self {
            case .hostPid: AuxText.hostFieldHostPid
            case .shellPid: AuxText.hostFieldShellPid
            case .program: AuxText.hostFieldProgram
            case .memory: AuxText.hostFieldMemory
            case .version: AuxText.hostFieldVersion
            case .protocolRange: AuxText.hostFieldProtocol
            case .started: AuxText.hostFieldStarted
            case .answer: AuxText.hostFieldAnswer
            case .link: AuxText.hostFieldLink
            case .folder: AuxText.hostFieldFolder
            case .socket: AuxText.hostFieldSocket
            case .meta: AuxText.hostFieldMeta
            }
        }

        /// Paths and the link run the card's full width; the rest sit two to a row.
        public var wide: Bool { [.link, .folder, .socket, .meta].contains(self) }
    }

    /// How a value is drawn: plainly, quietly (a path), or as a problem.
    public enum Tone: Sendable { case normal, muted, warning, danger }

    public var field: Field
    public var value: String
    public var tone: Tone
    public var id: Field { field }

    public init(_ field: Field, _ value: String, tone: Tone = .normal) { (self.field, self.value, self.tone) = (field, value, tone) }
}

public enum HostStanding: Sendable {
    /// A terminal of this app holds it.
    case attached
    /// Running, and no terminal of this app holds it.
    case detached
    /// Running, and no saved terminal names it.
    case orphan
    /// Its process, socket or meta file is gone: the leftovers can be forgotten.
    case stale

    public var label: String {
        switch self {
        case .attached: AuxText.hostsAttached
        case .detached: AuxText.hostsDetached
        case .orphan: AuxText.hostsOrphan
        case .stale: AuxText.hostsStale
        }
    }

    public var isLive: Bool { self != .stale }
}

public struct HostCard: Equatable, Sendable, Identifiable {
    public var id: String
    public var standing: HostStanding
    /// "Window 1, “api”", "Running, no terminal of this app holds it", why it is stale.
    public var note: String
    public var facts: [HostFact]
    /// The terminal that holds it, for the question before ending it.
    public var attachedTitle: String?
    /// A version mismatch, a lost link, no answer: the card's edge turns to the warning colour.
    public var needsAttention: Bool

    public init(
        id: String, standing: HostStanding, note: String, facts: [HostFact], attachedTitle: String? = nil,
        needsAttention: Bool = false
    ) {
        (self.id, self.standing, self.note, self.facts) = (id, standing, note, facts)
        (self.attachedTitle, self.needsAttention) = (attachedTitle, needsAttention)
    }

    public var canEnd: Bool { standing.isLive }
    public var canForget: Bool { standing == .stale }

    /// The details as plain text, one fact a line, for Copy.
    public var text: String {
        (["\(id) (\(standing.label)): \(note)"] + facts.map { "\($0.field.label): \($0.value)" }).joined(separator: "\n")
    }
}
