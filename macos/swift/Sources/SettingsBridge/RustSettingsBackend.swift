// `SettingsBackend` over the Rust `SettingsHandle`: the values are core's `Settings` as JSON, a write is core's
// `settings_index::set` then `Config::update` (merged, sanitised, saved in place), and a change made elsewhere (a hand
// edit of `config.json`, another window) arrives as the handle's one wake. Reads back what stuck.

import FontPickerModel
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
    private let fonts: FontFolder
    public var fontFolder: FontFolderState { fonts.state }

    /// `bundledFonts` are registered with CoreText before anything is drawn, with the files of the profile's fonts folder.
    init(handle: SettingsHandle, relay: WakeRelay, bundledFonts: [URL] = []) {
        self.handle = handle
        values = JSONValue.map(from: handle.values())
        fonts = FontFolder(bundled: bundledFonts)
        _ = fonts.take(from: handle)
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

    /// The file changed under the handle, or something it resolves with did (a theme file edited on disk, a font added, the
    /// profile's feature switches): take the new values and tell the observers either way. The handle wakes only when it
    /// read something again, and what moved is not always a value here.
    func woke() {
        _ = handle.drain()
        _ = fonts.take(from: handle)
        reload(woken: true)
    }

    /// Reads the values again, after a change the handle made that did not go through `set` (Reset all). After a wake the
    /// observers hear of it with the values the same too: the faces, the theme file or the switches they go with are not.
    func reload(woken: Bool = false) {
        let fresh = JSONValue.map(from: handle.values())
        guard fresh != values || woken else { return }
        values = fresh
        // In the order they were added: the model's own observer comes first and the window's follow it.
        observers.sorted { $0.key < $1.key }.forEach { $0.value() }
    }

    /// What the page's banner says about the file (does not read, last save failed), nil when there is nothing.
    public func banner() -> String? { handle.health().error }
    public var unreadable: Bool { handle.health().unreadable }
    public func changedRows() -> [(section: String, entry: String)]? { handle.changedRows().map { ($0.section, $0.entry) } }
}
