// The app's actions and their keys: one table, read by the menu bar, the shortcut recorder and the context
// menus. It is core's (`keys/keymap/table.rs`). Linked to the Rust core, `PlatformBridge.installKeymap()` reads it
// live (`keymapEntries()`); a build without the FFI reads `keymap.json`, which `macos/scripts/dump-keymap.py` writes
// from the same source, and a test in `PlatformBridgeTests` keeps the two equal.

import Foundation
import Synchronization

public struct Keymap: Sendable {
    public let entries: [KeymapEntry]
    /// The keys macOS keeps for itself, and what for.
    public let systemKeys: [(spec: KeySpec, what: String)]
    /// The menu bar's layout: which actions each menu holds.
    public let layout: [MenuLayout]
    private let byID: [ActionID: Int]

    private struct File: Decodable {
        struct SystemKey: Decodable { var glyphs: String; var what: String }
        var entries: [KeymapEntry]
        var layout: [MenuLayout]
        var systemKeys: [SystemKey]
    }

    public init(entries: [KeymapEntry], systemKeys: [(spec: KeySpec, what: String)], layout: [MenuLayout] = []) {
        self.entries = entries
        self.systemKeys = systemKeys
        self.layout = layout
        byID = Dictionary(entries.enumerated().map { ($1.id, $0) }, uniquingKeysWith: { first, _ in first })
    }

    public init(json: Data) throws {
        let file = try JSONDecoder().decode(File.self, from: json)
        entries = file.entries
        layout = file.layout
        systemKeys = file.systemKeys.compactMap { key in KeySpec(glyphs: key.glyphs).map { ($0, key.what) } }
        byID = Dictionary(file.entries.enumerated().map { ($1.id, $0) }, uniquingKeysWith: { first, _ in first })
    }

    private static let installed = Mutex<Keymap?>(nil)

    /// The table the app reads: core's own once `install` ran (the Rust core linked), else `bundled`.
    public static var shared: Keymap { installed.withLock { $0 } ?? bundled }

    /// Makes `keymap` the one everything reads. Call once, at launch, before a window asks.
    public static func install(_ keymap: Keymap) { installed.withLock { $0 = keymap } }

    /// The dumped table that ships with the app: what a build without the Rust core reads.
    public static let bundled: Keymap = {
        guard let url = Bundle.module.url(forResource: "keymap", withExtension: "json"),
            let data = try? Data(contentsOf: url), let keymap = try? Keymap(json: data)
        else { preconditionFailure("keymap.json is missing or unreadable: run macos/scripts/dump-keymap.py") }
        return keymap
    }()

    public func entry(_ id: ActionID) -> KeymapEntry? { byID[id].map { entries[$0] } }

    /// The key menus and the sheet show for `id`.
    public func key(_ id: ActionID) -> KeySpec? { entry(id)?.key }

    /// A tooltip that names its key: "Hide sidebar (⌃⌘S)".
    public func withKey(_ text: String, _ id: ActionID) -> String {
        key(id).map { "\(text) (\($0.glyphText))" } ?? text
    }

    /// The action `spec` runs where `scope` applies, live or not.
    public func action(for spec: KeySpec, scope: KeyScope) -> KeymapEntry? {
        entries.first { $0.scope.overlaps(scope) && $0.uses(spec) }
    }
}
