// A program asking to control Wormhole through its control API, with what is known of it (core `ipc::access`
// `Pending` and `Resolved`, `platform::codesign`). The approval sheet is built from these facts; the program's
// name, path, pid and signer are data, the sentences are the catalog's.

import Foundation

public struct AccessRequest: Equatable, Sendable {
    public enum Capability: Sendable { case read, type, run, open, close }

    /// What kind of program it is, which decides what can be remembered about it.
    public enum Kind: Sendable { case scripts, script, unsigned, adHoc, developerID, apple }

    public enum SignerKind: Sendable { case developerID, appleSystem, adHoc, other, unsigned, unreadable }

    public struct Signer: Equatable, Sendable {
        public var kind: SignerKind
        public var subject: String?
        public var team: String?
        public init(kind: SignerKind, subject: String? = nil, team: String? = nil) { (self.kind, self.subject, self.team) = (kind, subject, team) }
    }

    public struct Program: Equatable, Sendable {
        public var path: String
        public var pid: Int
        public var signer: Signer
        /// For a script: the interpreter it runs on.
        public var interpreter: String?
        public var startedBy: String?
        /// The terminal it runs in, if one of the app's.
        public var runsIn: String?

        public init(path: String, pid: Int, signer: Signer, interpreter: String? = nil, startedBy: String? = nil, runsIn: String? = nil) {
            (self.path, self.pid, self.signer, self.interpreter, self.startedBy, self.runsIn) = (path, pid, signer, interpreter, startedBy, runsIn)
        }
    }

    /// The terminal the question is about, when it is about one terminal and not the program.
    public struct Terminal: Equatable, Sendable {
        public var name: String
        public var folder: String?
        public init(name: String, folder: String? = nil) { (self.name, self.folder) = (name, folder) }
    }

    public var label: String
    public var capability: Capability
    public var kind: Kind
    public var program: Program?
    public var terminal: Terminal?
    public var unusualSigner = false
    public var changedSinceAllowed = false
    public var elevated = false
    /// Whether "Always" is offered: not for a terminal, and not for a program that can't be told again.
    public var canBeRemembered = false

    public init(
        label: String, capability: Capability, kind: Kind, program: Program? = nil, terminal: Terminal? = nil,
        unusualSigner: Bool = false, changedSinceAllowed: Bool = false, elevated: Bool = false, canBeRemembered: Bool = false
    ) {
        (self.label, self.capability, self.kind, self.program, self.terminal) = (label, capability, kind, program, terminal)
        (self.unusualSigner, self.changedSinceAllowed, self.elevated, self.canBeRemembered) = (unusualSigner, changedSinceAllowed, elevated, canBeRemembered)
    }
}
