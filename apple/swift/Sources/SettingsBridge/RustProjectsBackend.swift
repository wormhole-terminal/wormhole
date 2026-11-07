// `ProjectsBackend` over the core's `ProjectService`, the one the Project tab uses (so a trust given here is what the tab
// shows). Reads touch the disk and approving waits for the owner (Touch ID, or the password): off the main thread.

import Foundation
import SettingsModel
import WormholeBindings

struct RustProjectsBackend: ProjectsBackend {
    let service: ProjectService
    /// The folders of the open terminals, which the page offers the projects of.
    let folders: @MainActor @Sendable () -> [String]
    let terminal: @MainActor @Sendable (String) -> Void

    func defaults() async -> [ProjectDefault] { await run { $0.projectDefaults().map(RecordMapping.projectDefault) } }
    func trusted() async -> [ProjectChoice] { await run { $0.trustedProjects().map(RecordMapping.projectChoice) } }

    func known() async -> [ProjectChoice] {
        let open = await folders().map { ($0 as NSString).expandingTildeInPath }
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

    private func run<T: Sendable>(_ work: @escaping @Sendable (ProjectService) -> T) async -> T {
        let service = service
        return await Task.detached { work(service) }.value
    }
}
