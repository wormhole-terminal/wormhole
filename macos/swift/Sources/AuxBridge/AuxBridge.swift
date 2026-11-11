// The auxiliary windows' backends over the Rust core, through `wormhole-ffi` (`macos/FFI-CONTRACT.md`, "Auxiliary
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
    public static func makeHostsBackend(dataDir: URL? = nil, facts: FFIHostsBackend.Facts? = nil) throws -> FFIHostsBackend {
        try facts.map { try FFIHostsBackend(dataDir: dataDir, facts: $0) } ?? FFIHostsBackend(dataDir: dataDir)
    }

    @MainActor
    public static func makeTrustBackend(root: URL, profile: String? = nil, authenticator: any Authenticating) throws -> FFITrustBackend {
        try FFITrustBackend(root: root, profile: profile, authenticator: authenticator)
    }

    @MainActor
    public static func makeAccessBackend(profile: String? = nil, resolver: FFIAccessBackend.Resolver = .init()) throws -> FFIAccessBackend {
        try FFIAccessBackend(profile: profile, resolver: resolver)
    }

    public static func makeLicenseSource() -> FFILicenseSource { FFILicenseSource() }
}
