// Whether this run may touch the system. A test instance (`--background`, WORMHOLE_BACKGROUND=1) must never
// register a global hotkey, show a Dock icon or badge, or post a banner: every AppKit wrapper in PlatformUI
// asks this one value, so no feature has to remember (GPUI's `platform::background_mode`).

import Foundation

public struct PlatformPolicy: Sendable, Equatable {
    /// A test instance kept out of the system.
    public var background: Bool

    public init(background: Bool) {
        self.background = background
    }

    /// The Dock, the global hotkey and the notification centre are the person's, not a test's.
    public var touchesSystem: Bool { !background }

    /// What WORMHOLE_BACKGROUND says: only a yes turns it on (`background_mode::requested`).
    public static func requested(_ value: String?) -> Bool {
        guard let value else { return false }
        return ["1", "true", "yes", "on"].contains(value.trimmingCharacters(in: .whitespaces).lowercased())
    }

    public static func current(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        arguments: [String] = CommandLine.arguments
    ) -> PlatformPolicy {
        PlatformPolicy(background: requested(environment["WORMHOLE_BACKGROUND"]) || arguments.contains("--background")) // text: an environment variable name
    }
}
