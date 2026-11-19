// `ProjectsBackend` over the core's `ProjectService`, the one the Project tab uses (so a trust given here is what the tab
// shows). Reads touch the disk and approving waits for the owner (Touch ID, or the password): off the main thread.

import Foundation
import SettingsModel
import WormholeBindings
import WorkspaceModel

struct RustProjectsBackend: ProjectsBackend {
    let service: ProjectService
    /// The folders of the open terminals, which the page offers the projects of.
    let folders: @MainActor @Sendable () -> [String]
    let terminal: @MainActor @Sendable (String) -> Void
    let ownerAvailable: @MainActor @Sendable () -> Bool

    func defaults() async -> [ProjectDefault] { await run { $0.projectDefaults().map(RecordMapping.projectDefault) } }
    func trusted() async -> [ProjectChoice] { await run { $0.trustedProjects().map(RecordMapping.projectChoice) } }

    func known() async -> [ProjectChoice] {
        let open = await folders().map { HomeFolder.expanding($0) }
        return await run { $0.knownProjects(folders: open).map(RecordMapping.projectChoice) }
    }

    func page(_ root: String) async -> ProjectPage? { await run { $0.projectPage(root: root).map(RecordMapping.projectPage) } }

    func setDefault(_ key: String, on: Bool) async { await run { _ = try? $0.setProjectDefault(feature: key, on: on) } }

    func setChoice(_ root: String, feature: String, to choice: Bool?) async {
        await run { service in
            if let choice { _ = try? service.setFeature(root: root, feature: feature, on: choice) } else {
                _ = try? service.clearFeatureChoice(root: root, feature: feature)
            }
        }
    }

    func trust(_ root: String) async { await run { _ = $0.approve(root: root) } }
    func revoke(_ root: String) async { await run { $0.revoke(root: root) } }
    @MainActor func openTerminal(_ root: String) { terminal(root) }
    @MainActor var canApprove: Bool { ownerAvailable() }

    /// The core's watch on the project's `.wormhole` folder; the token keeps it alive.
    func follow(_ root: String, changed: @escaping @Sendable () -> Void) -> AnyObject? {
        settingsWatchProject(root: HomeFolder.expanding(root), listener: ProjectChanges(changed))
    }

    /// The core's watch on the profile's approvals file; the token keeps it alive.
    func followTrust(changed: @escaping @Sendable () -> Void) -> AnyObject? {
        service.watchTrust(listener: ProjectChanges(changed))
    }

    /// The folder as the core has its bytes (`project_dir_bytes`): a root whose name is not text comes as a key, which only
    /// the core reads back.
    func projectFolder(_ root: String, dirName: String) async -> URL? {
        guard let bytes = await run({ $0.projectDirBytes(root: root) }) else { return nil }
        let path = bytes.map { CChar(bitPattern: $0) } + [0]
        return path.withUnsafeBufferPointer { buffer in
            buffer.baseAddress.map { URL(fileURLWithFileSystemRepresentation: $0, isDirectory: true, relativeTo: nil) }
        }
    }

    private func run<T: Sendable>(_ work: @escaping @Sendable (ProjectService) -> T) async -> T {
        let service = service
        return await Task.detached { work(service) }.value
    }
}

/// Hears `ProjectListener.changed()`, which Rust calls on a thread of its own, and passes it on.
private final class ProjectChanges: ProjectListener, Sendable {
    private let onChange: @Sendable () -> Void
    init(_ onChange: @escaping @Sendable () -> Void) { self.onChange = onChange }
    func changed() { onChange() }
}
