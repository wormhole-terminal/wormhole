// `HostsBackend` over core's session-host survey (`HostsDesk`): the report with every fact said, End over the
// host's socket or SIGTERM, Forget for stale files, and a watch of the hosts folder. The calls that look at
// sockets block in Rust, so they run off the main actor.

import AuxModel
import Foundation
import WormholeBindings

@MainActor
public final class FFIHostsBackend: HostsBackend {
    private let desk: HostsDesk
    private let hub = WakeHub()

    /// `dataDir`: a folder named `Wormhole-swift*` (tests), or `nil` for the app's own, which must be one.
    public init(dataDir: URL? = nil) throws {
        desk = try HostsDesk(dataDir: dataDir?.path, listener: hub)
    }

    /// The terminals of the app's windows that hold a host (the window knows; the folder does not).
    public func setAttachments(_ attachments: [(hostID: String, window: Int, title: String)]) {
        desk.setAttachments(attachments: attachments.map { .init(hostId: $0.hostID, window: Int64($0.window), title: $0.title) })
    }

    public func setPersistentTerminals(_ on: Bool) { desk.setPersistentTerminals(on: on) }

    public func report() async -> HostsReport {
        let desk = desk
        let record = await Task.detached { desk.report() }.value
        return record.value
    }

    public func changes() -> AsyncStream<Void> {
        let desk = desk
        _ = desk.follow()
        return hub.stream(onFinish: { desk.unfollow() })
    }

    public func end(_ id: String) async {
        let desk = desk
        _ = await Task.detached { try? desk.end(id: id) }.value
    }

    public func forget(_ id: String) async {
        let desk = desk
        _ = await Task.detached { try? desk.forget(id: id) }.value
    }
}

extension HostsReportRecord {
    var value: HostsReport {
        let program: HostsReport.HostProgram =
            switch self.program {
            case .found: .found(path: programPath ?? "")
            case .disposableCopy: .disposableCopy
            case .missing: .missing
            }
        return HostsReport(
            appVersion: appVersion, protocolText: protocolText, folder: folder, flagOn: flagOn, program: program,
            cards: cards.map(\.value))
    }
}

extension HostCardRecord {
    var value: HostCard {
        let standing: HostStanding =
            switch self.standing {
            case .attached: .attached
            case .detached: .detached
            case .orphan: .orphan
            case .stale: .stale
            }
        return HostCard(
            id: id, standing: standing, note: note, facts: facts.map(\.fact), attachedTitle: attachedTitle,
            needsAttention: needsAttention)
    }
}

extension HostFactRecord {
    var fact: HostFact {
        let field: HostFact.Field =
            switch self.field {
            case .hostPid: .hostPid
            case .shellPid: .shellPid
            case .program: .program
            case .memory: .memory
            case .version: .version
            case .protocolRange: .protocolRange
            case .started: .started
            case .answer: .answer
            case .link: .link
            case .folder: .folder
            case .socket: .socket
            case .meta: .meta
            }
        let tone: HostFact.Tone =
            switch self.tone {
            case .normal: .normal
            case .muted: .muted
            case .warning: .warning
            case .danger: .danger
            }
        return HostFact(field, self.value, tone: tone)
    }
}
