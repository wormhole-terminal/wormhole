// `ProbesBackend` over the core's `ProbesDesk`. Every call runs off the main thread: the core reads the approval keys,
// which may block or ask the system, and approving waits for the owner (Touch ID, or the password).

import SettingsModel
import WormholeBindings

struct RustProbesBackend: ProbesBackend {
    let desk: ProbesDesk

    func list() async -> ProbeList { await run { RecordMapping.probeList($0.list()) } }

    func sheet(_ id: String) async throws -> ProbeSheet {
        try await run { desk in Result { RecordMapping.probeSheet(try desk.sheet(id: id)) }.mapError(Sentence.init) }.get()
    }

    func needsTouchID(_ id: String, granted: Set<Int>) async -> Bool {
        await run { $0.needsTouchId(id: id, granted: granted.map { UInt32($0) }) }
    }

    func approve(_ id: String, granted: Set<Int>) async -> ProbeApproval {
        await run { RecordMapping.approval($0.approve(id: id, granted: granted.map { UInt32($0) })) }
    }

    func turnOff(_ id: String) async throws {
        try await run { desk in Result { try desk.turnOff(id: id) }.mapError(Sentence.init) }.get()
    }

    private func run<T: Sendable>(_ work: @escaping @Sendable (ProbesDesk) -> T) async -> T {
        let desk = desk
        return await Task.detached { work(desk) }.value
    }
}
