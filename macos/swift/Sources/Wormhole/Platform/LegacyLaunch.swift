// What a launch brings forward from a version before the rename (`app/launch.rs` and `main.rs` of the GPUI app): the data folder's move,
// the agent hooks installed under the codename, and the default-terminal role held by the old bundle id. The Swift app never opens a
// folder named `Wormhole` (`Isolation`): its dev folders are `Wormhole-swift*` and the beta's is `Wormhole Beta`, all new, so none of it
// applies yet. It is built where the GPUI app makes it so it runs the day the app opens the 1.x data folder, and never against another:
// in the beta's folder it would rewrite the person's real agent hooks and could take the default-terminal role from the 1.x app.

import Foundation

#if WITH_UNIFFI
    import AppKit
    import PlatformModel
    import UniformTypeIdentifiers
    import WormholeBindings
#endif

enum LegacyLaunch {
    /// The bundle identifier versions up to 0.2 ran under.
    static let legacyBundleID = "dev.hyperficial.andromeda"

    /// The 1.x app's data folder (`DATA_DIR_NAME` in `packaging/app.env`, which a test compares with it): the only one that can hold
    /// anything from before the rename.
    static let dataFolderName = "Wormhole" // text: a folder name

    static func applies(to dataDirectory: URL) -> Bool { dataDirectory.lastPathComponent == dataFolderName }

    /// Whether the app that runs a kind of file is the old bundle (the role the renamed app takes over).
    static func heldByLegacy(_ handlerBundleID: String?) -> Bool { handlerBundleID?.caseInsensitiveCompare(legacyBundleID) == .orderedSame }

    /// Before anything opens a file in the data folder: the old folder becomes the new one, once.
    static func moveDataFolder(_ dataDirectory: URL) {
        #if WITH_UNIFFI
            guard applies(to: dataDirectory) else { return }
            report(legacyMoveDataFolder())
        #endif
    }

    /// Before the windows: the old hooks become today's, and the default-terminal role of the old bundle id is taken over (both off
    /// the main thread's way: the hooks are file edits, the role a Launch Services call).
    @MainActor static func afterLaunch(dataDirectory: URL, policy: PlatformPolicy) {
        #if WITH_UNIFFI
            guard applies(to: dataDirectory) else { return }
            Task.detached { report(legacyReplaceAgentHooks(agentHome: nil)) }
            Task { await DefaultTerminal.takeOverFromLegacy(policy: policy) }
        #endif
    }

    #if WITH_UNIFFI
        private static func report(_ done: WormholeBindings.LegacyUpgrades) {
            if let moved = done.movedDataFolder { FileHandle.standardError.write(Data("moved the earlier version's data to \(moved)\n".utf8)) } // text: a stderr line
            for agent in done.hooksReplaced { FileHandle.standardError.write(Data("replaced the hooks an earlier version installed for \(agent)\n".utf8)) } // text: a stderr line
            for problem in done.problems { FileHandle.standardError.write(Data("\(problem)\n".utf8)) }
        }
    #endif
}

#if WITH_UNIFFI
    extension DefaultTerminal {
        /// When an earlier version was the default terminal, the renamed app takes the role over: macOS keeps the choice by bundle
        /// identifier, so it would otherwise still name the app that is gone.
        static func takeOverFromLegacy(policy: PlatformPolicy) async {
            guard Bundle.main.bundleURL.pathExtension == "app" else { return }
            let held = defaultTerminalScriptTypes().compactMap { UTType($0) }.contains { kind in
                LegacyLaunch.heldByLegacy(NSWorkspace.shared.urlForApplication(toOpen: kind).flatMap { Bundle(url: $0)?.bundleIdentifier })
            }
            if held { _ = await make(policy: policy) }
        }
    }
#endif
