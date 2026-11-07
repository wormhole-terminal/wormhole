// What the Extensions page does: the list as last read, the approval sheet, and what is waiting. A switch turned on
// opens the sheet (the core reads the Probe once for it); turned off, it needs no question. Approving sends back the
// permissions left ticked, and the core decides whether that needs a click or the owner's authentication.

import Observation

@MainActor @Observable
public final class ProbesModel {
    /// A sheet that is open: the permissions left ticked, and what is happening to it.
    public struct Open: Equatable {
        public var sheet: ProbeSheet
        public var granted: Set<Int>
        public var needsTouchID: Bool
        /// The system's question is up, or the approval is being written: nothing on the sheet changes.
        public var busy = false
        public var error: String?
    }

    private let backend: any ProbesBackend
    /// `nil` until the first read finishes.
    public private(set) var list: ProbeList?
    public private(set) var open: Open?
    /// Why a Probe could not be turned off or read for its sheet, on the page until the next try.
    public private(set) var notice: String?

    public init(backend: any ProbesBackend) { self.backend = backend }

    public func refresh() async { list = await backend.list() }

    /// The switch of a row: on opens the sheet, off turns it off.
    public func switched(_ row: ProbeRow, on: Bool) async {
        if on { await openSheet(row.id) } else { await turnOff(row.id) }
    }

    public func openSheet(_ id: String) async {
        notice = nil
        do {
            let sheet = try await backend.sheet(id)
            let granted = Set(sheet.permissions.filter(\.granted).map(\.id))
            open = Open(sheet: sheet, granted: granted, needsTouchID: await backend.needsTouchID(id, granted: granted))
        } catch {
            notice = "\(error)"
            await refresh()
        }
    }

    public func turnOff(_ id: String) async {
        notice = nil
        do { try await backend.turnOff(id) } catch { notice = "\(error)" }
        await refresh()
    }

    public func toggle(_ permission: ProbePermission) async {
        guard var current = open, !current.busy else { return }
        if !current.granted.insert(permission.id).inserted { current.granted.remove(permission.id) }
        open = current
        let touch = await backend.needsTouchID(current.sheet.id, granted: current.granted)
        open?.needsTouchID = touch
    }

    public func close() {
        if open?.busy != true { open = nil }
    }

    public func approve() async {
        guard var current = open, !current.busy else { return }
        current.busy = true
        current.error = nil
        open = current
        switch await backend.approve(current.sheet.id, granted: current.granted) {
        case .approved: open = nil
        case .cancelled: open?.busy = false
        case .failed(let message): open?.busy = false; open?.error = message
        }
        await refresh()
    }
}
