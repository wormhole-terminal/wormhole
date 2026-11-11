// The settings window over the Rust core: one call makes the model with the real backend, host, search and shortcut
// rules. `SettingsBridge.makeModel()` replaces `SettingsModel(backend: MockSettingsBackend(), host: MockSettingsHost())`.
// The process must be an isolated experiment instance (`WORMHOLE_DATA_DIR` ending in `Wormhole-swift…`), or the
// handle refuses to open.

import Foundation
import SettingsModel
import WormholeBindings

public enum SettingsBridge {
    /// The pieces of a window's settings, for a caller that wants them apart.
    public struct Kit {
        public let schema: SettingsSchema
        public let backend: RustSettingsBackend
        public let host: RustSettingsHost
        public let searcher: RustSettingsSearch
        public let shortcuts: RustShortcuts
    }

    /// Opens `profile`'s settings (the Default profile for `nil`) and starts following its file. Reads, and may
    /// create, `config.json`: call it off the main thread when the window is not waiting for it.
    @MainActor
    public static func make(
        profile: String? = nil, platform: RustSettingsHost.Platform = .init(),
        importHome: String? = ProcessInfo.processInfo.environment["WORMHOLE_IMPORT_HOME"] // text: environment variable
    ) throws -> Kit {
        let relay = WakeRelay()
        let handle = try SettingsHandle.open(profile: profile, listener: relay)
        let schema = try SettingsSchema(json: Data(settingsSchemaJson().utf8))
        let backend = RustSettingsBackend(handle: handle, relay: relay, bundledFonts: platform.bundledFonts())
        return Kit(
            schema: schema, backend: backend,
            host: RustSettingsHost(handle: handle, backend: backend, platform: platform, importHome: importHome),
            searcher: RustSettingsSearch(handle: handle, schema: schema), shortcuts: RustShortcuts())
    }

    /// The window's model over the real core.
    @MainActor
    public static func makeModel(
        profile: String? = nil, platform: RustSettingsHost.Platform = .init(), selection: String = "appearance"
    ) throws -> SettingsModel {
        let kit = try make(profile: profile, platform: platform)
        return SettingsModel(
            schema: kit.schema, backend: kit.backend, host: kit.host, searcher: kit.searcher,
            shortcuts: kit.shortcuts, selection: selection)
    }
}
