// `AccessBackend` over core's question queue (`AccessDesk`): the questions the control API's gate raises, as the
// facts `AccessDisplay` words, and the person's answer to each, which core records (for a session, until quit, for
// good) and tells every request that waited. The terminal's name and folder are the window's to know, so they
// come in through `Resolver`.

import AuxModel
import Foundation
import WormholeBindings

@MainActor
public final class FFIAccessBackend: AccessBackend {
    /// What only the window knows: a terminal's name and folder, and the terminal a pid runs in.
    public struct Resolver {
        public var terminal: @MainActor (_ id: Int64) -> AccessRequest.Terminal?
        public var terminalOf: @MainActor (_ pid: UInt32) -> String?

        public init(
            terminal: @escaping @MainActor (Int64) -> AccessRequest.Terminal? = { _ in nil },
            terminalOf: @escaping @MainActor (UInt32) -> String? = { _ in nil }
        ) {
            (self.terminal, self.terminalOf) = (terminal, terminalOf)
        }
    }

    /// The desk the control socket's gate puts its questions on; Rust-side callers share it.
    public let desk: AccessDesk
    private let hub = WakeHub()
    private let resolver: Resolver

    public init(profile: String? = nil, dataDir: URL? = nil, resolver: Resolver = Resolver()) throws {
        desk = try AccessDesk(profile: profile, dataDir: dataDir?.path, listener: hub)
        self.resolver = resolver
    }

    public func next() -> AccessQuestion? {
        desk.nextQuestion().map { AccessQuestion(id: $0.id, request: request(of: $0)) }
    }

    public func changes() -> AsyncStream<Void> { hub.stream() }

    public func answer(_ id: UInt64, _ button: AccessButton) {
        let answer: AccessAnswer? =
            switch button {
            case .deny: .deny
            case .session: .session
            case .quit: .untilQuit
            case .always: .always
            // The pair for a mode edited outside the app belongs to the settings' own question.
            case .keep, .dontKeep: nil
            }
        if let answer { desk.answer(id: id, answer: answer) }
    }

    public func withdraw(_ id: UInt64) { desk.withdraw(id: id) }

    func request(of record: AccessQuestionRecord) -> AccessRequest {
        let program = record.program.map { program in
            AccessRequest.Program(
                path: program.path, pid: Int(program.pid),
                signer: .init(kind: program.signer.value, subject: program.subject, team: program.team),
                interpreter: program.interpreter, startedBy: program.startedBy, runsIn: resolver.terminalOf(program.pid))
        }
        return AccessRequest(
            label: record.label, capability: record.capability.value, kind: record.kind.value, program: program,
            terminal: record.aboutTerminal.map { resolver.terminal($0) ?? .init(name: String($0)) },
            unusualSigner: record.unusualSigner, changedSinceAllowed: record.changedSinceAllowed,
            elevated: record.elevated, canBeRemembered: record.canBeRemembered)
    }
}

extension AccessCapability {
    var value: AccessRequest.Capability {
        switch self {
        case .read: .read
        case .type: .type
        case .run: .run
        case .open: .open
        case .close: .close
        }
    }
}

extension AccessKindRecord {
    var value: AccessRequest.Kind {
        switch self {
        case .scripts: .scripts
        case .script: .script
        case .unsigned: .unsigned
        case .adHoc: .adHoc
        case .developerId: .developerID
        case .apple: .apple
        }
    }
}

extension AccessSignerKind {
    var value: AccessRequest.SignerKind {
        switch self {
        case .developerId: .developerID
        case .appleSystem: .appleSystem
        case .adHoc: .adHoc
        case .other: .other
        case .unsigned: .unsigned
        case .unreadable: .unreadable
        }
    }
}
