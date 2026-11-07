// Make this app the one macOS opens scripts and programs with (`platform/default_app.rs`): the kinds of file are core's
// (`terminal::default_terminal::SCRIPT_TYPES`: a `.command` script and a program without an extension), the choice is
// LaunchServices'. It needs a bundle to name, so a bare executable (a `swift run`, a test instance) cannot be anyone's
// default, and an instance that must not touch the system (`--background`) does not ask.

#if WITH_UNIFFI
    import AppKit
    import PlatformModel
    import UniformTypeIdentifiers
    import WormholeBindings

    @MainActor
    enum DefaultTerminal {
        /// Asks the system, once per kind. The sentence for what went wrong (core's catalog), or nil when it is done.
        static func make(policy: PlatformPolicy) async -> String? {
            let bundle = Bundle.main.bundleURL
            guard bundle.pathExtension == "app" else { return defaultTerminalFailed(error: "not running from Wormhole.app") }
            guard policy.touchesSystem else { return defaultTerminalFailed(error: "this instance never changes the system") }
            for identifier in defaultTerminalScriptTypes() {
                guard let kind = UTType(identifier) else { continue }
                do { try await NSWorkspace.shared.setDefaultApplication(at: bundle, toOpen: kind) } catch {
                    return defaultTerminalFailed(error: error.localizedDescription)
                }
            }
            return nil
        }
    }
#endif
