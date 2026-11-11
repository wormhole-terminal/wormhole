// `SettingsBackend` over the Rust `SettingsHandle`: the values are core's `Settings` as JSON, a write is core's
// `settings_index::set` then `Config::update` (merged, sanitised, saved in place), and a change made elsewhere (a hand
// edit of `config.json`, another window) arrives as the handle's one wake. Reads back what stuck.

import Foundation
import SettingsModel
import WormholeBindings

@MainActor
public final class RustSettingsBackend: SettingsBackend {
    public let handle: SettingsHandle
    public private(set) var values: [String: JSONValue]
    /// Core's sentence for the last write it refused (`opacity: invalid type: string …`); nil after one that stuck.
    public private(set) var lastRefusal: String?
    private var observers: [Int: @MainActor () -> Void] = [:]
    private var nextObserver = 0

    init(handle: SettingsHandle, relay: WakeRelay) {
        self.handle = handle
        values = JSONValue.map(from: handle.values())
        relay.attach(self)
    }

    public func set(_ key: String, to value: JSONValue) {
        do {
            try handle.set(key: key, value: value.jsonText())
            lastRefusal = nil
        } catch {
            lastRefusal = SettingsFailure.sentence(error)
        }
        reload()
    }

    public func reset(_ keys: [String]) {
        handle.reset(keys: keys)
        reload()
    }

    public func observe(_ onChange: @escaping @MainActor () -> Void) -> SettingsObservation {
        let id = nextObserver
        nextObserver += 1
        observers[id] = onChange
        return SettingsObservation { [weak self] in
            // `deinit` may run anywhere; the observer table is main-actor state.
            Task { @MainActor in self?.observers[id] = nil }
        }
    }

    /// The file changed under the handle: take the new values.
    func woke() {
        _ = handle.drain()
        reload()
    }

    /// Reads the values again, after a change the handle made that did not go through `set` (Reset all).
    func reload() {
        let fresh = JSONValue.map(from: handle.values())
        guard fresh != values else { return }
        values = fresh
        // In the order they were added: the model's own observer comes first and the window's follow it.
        observers.sorted { $0.key < $1.key }.forEach { $0.value() }
    }

    /// What the page's banner says about the file (does not read, last save failed), nil when there is nothing.
    public func banner() -> String? { handle.health().error }
    public var unreadable: Bool { handle.health().unreadable }
    public func changedRows() -> [(section: String, entry: String)]? { handle.changedRows().map { ($0.section, $0.entry) } }
}
