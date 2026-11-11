// The app's hook socket and control line, running (`HookServer` of `wormhole-ffi`; `apple/FFI-CONTRACT.md`, "The
// hook socket and the control line"). The agents' hooks and `Wormhole list|send-text|…` reach the workspace
// without Swift; what only the app can do arrives as `HookAction`s.

import Foundation
import WormholeBindings

@MainActor
public final class HookServerHandle {
    private let server: HookServer
    private let wake = HookWake()

    /// What the server asks the app to do, one array per wake. Ends when `stop()` is called.
    public var actions: AsyncStream<[HookAction]> {
        let (stream, continuation) = AsyncStream<[HookAction]>.makeStream(bufferingPolicy: .unbounded)
        let wake = wake, server = server
        let task = Task.detached {
            for await _ in wake.stream {
                // Reading re-arms the wake, so a request that comes during this read wakes again.
                let events = server.drainEvents().map(HookAction.init)
                if !events.isEmpty { continuation.yield(events) }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in task.cancel() }
        return stream
    }

    init(workspace: Workspace, usage: UsageService, access: AccessDesk, dataDir: URL?) throws {
        let wake = wake
        server = try HookServer.start(
            workspace: workspace, usage: usage, access: access, listener: wake, dataDir: dataDir?.path)
    }

    /// Where the app listens: what `WORMHOLE_SOCKET` holds in this data folder's terminals.
    public var socketPath: String { server.socketPath() }

    /// The question a `.closeAsked` put to the person was answered (yes or no): another may be asked.
    public func closeQuestionOver() { server.closeQuestionOver() }

    /// Take approval `id` back and end the streams it let in (use this, not `AccessDesk.revoke`, so they end).
    @discardableResult
    public func revokeApproval(profile: String, _ id: UInt64) -> Bool { server.revokeApproval(profile: profile, id: id) }

    /// Take stored approval `id` (a database row of `AccessDesk.approvals()`) back and end the streams it let in.
    @discardableResult
    public func revokeStoredApproval(profile: String, _ id: UInt64) -> Bool {
        server.revokeStoredApproval(profile: profile, id: id)
    }

    /// Another profile's desk: what the requests reaching that profile's windows are judged by.
    public func addDesk(_ desk: AccessDesk) { server.addDesk(desk: desk) }

    /// Profile `profile`'s settings changed: the server reads its agent-access mode again.
    public func settingsChanged(profile: String) { server.settingsChanged(profile: profile) }

    /// The status items the Probes' hooks set (`HookAction.probesChanged` says to read them again).
    public func probeStatusItems() -> [ProbeStatusRecord] { server.probeStatusItems() }

    /// Stop serving: the socket goes, the lock is given up, the streams end.
    public func stop() {
        server.stop()
        wake.finish()
    }
}
