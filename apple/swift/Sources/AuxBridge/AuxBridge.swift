// The auxiliary windows' backends over the Rust core, through `wormhole-ffi` (`apple/FFI-CONTRACT.md`, "Auxiliary
// windows"). One factory each, so the app wires a window with one line; linked builds only (the mock backends of
// `AuxModel` stay for previews and for a build without the library).

import AuxModel
import Foundation
import PlatformModel

public enum AuxBridge {
    /// The updater for a release build; `hooks` carry what only the app can do (quit, idle terminals).
    @MainActor
    public static func makeUpdateBackend(setup: FFIUpdateBackend.Setup, hooks: FFIUpdateBackend.Hooks) throws -> FFIUpdateBackend {
        try FFIUpdateBackend(setup: setup, hooks: hooks)
    }

    @MainActor
    public static func makeHostsBackend(dataDir: URL? = nil) throws -> FFIHostsBackend { try FFIHostsBackend(dataDir: dataDir) }

    @MainActor
    public static func makeTrustBackend(root: URL, authenticator: any Authenticating) throws -> FFITrustBackend {
        try FFITrustBackend(root: root, authenticator: authenticator)
    }

    @MainActor
    public static func makeAccessBackend(resolver: FFIAccessBackend.Resolver = .init()) throws -> FFIAccessBackend {
        try FFIAccessBackend(resolver: resolver)
    }

    public static func makeLicenseSource() -> FFILicenseSource { FFILicenseSource() }
}
