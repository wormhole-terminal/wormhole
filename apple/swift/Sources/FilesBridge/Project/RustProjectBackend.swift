// `ProjectBackend` over `wormhole-core`'s project domain. Trust is enforced in Rust: the ask for an action, a prompt
// or the layout comes back as effects only where the file is there, the person has it on and the folder's content is
// approved, and `approve` asks the owner first. This class only asks, hands the effects to the window and shows the
// answer.

import Foundation
import PanelModel
import WormholeBindings
import WorkspaceModel

public final class RustProjectBackend: ProjectBackend, @unchecked Sendable {
    public let service: ProjectService
    private let effects: any PanelEffectPerforming

    public let biometricsAvailable: Bool

    public init(service: ProjectService, effects: any PanelEffectPerforming, biometricsAvailable: Bool) {
        self.service = service
        self.effects = effects
        self.biometricsAvailable = biometricsAvailable
    }

    public func worktreeRefusal(of repo: String) async -> String? {
        let service = service
        return await offMain { service.worktreeAllowed(repo: repo) }
    }

    public func worktreeSetup(of repo: String) async -> String? {
        let service = service
        return await offMain { service.worktreeSetup(repo: repo) }
    }

    public func project(at folder: String) async -> ProjectState? {
        let service = service
        return await offMain { service.project(folder: folder).map(ProjectState.init) }
    }

    /// What the project of `folder` sets over the person's settings, for Settings to layer: nil where there is no project;
    /// its `json` is nil unless the project's Settings switch is on and trust allows it.
    public func settingsOverrides(at folder: String) async -> ProjectSettingsRecord? {
        let service = service
        return await offMain { service.settingsOverrides(folder: folder) }
    }

    public func folderPaint(of folder: String) async -> FolderPaint {
        let service = service
        return await offMain { FolderPaint(service.folderPaint(folder: folder)) }
    }

    public func setFolderColor(_ color: FolderColor?, of folder: String) async {
        await setColor(color?.rawValue, of: folder)
    }

    public func setFolderColor(hex: String, of folder: String) async { await setColor(hex, of: folder) }

    public func createProject(at folder: String) async -> String? {
        let service = service
        return await offMain {
            do { _ = try service.createProject(folder: folder); return nil } catch { return Self.spoken(error) }
        }
    }

    public func setFeature(_ feature: ProjectFeature, on: Bool, at root: String) async {
        let service = service
        await report { try service.setFeature(root: root, feature: feature.rawValue, on: on) }
    }

    public func approve(at root: String) async -> Bool {
        let service = service
        return await offMain { service.approve(root: root) == .approved }
    }

    public func revoke(at root: String) async {
        let service = service
        await offMain { service.revoke(root: root) }
    }

    public func run(action id: String, at root: String) async {
        let service = service
        await perform { try service.runAction(root: root, id: id) }
    }

    public func send(prompt id: String, at root: String) async {
        let service = service
        await perform { try service.sendPrompt(root: root, id: id) }
    }

    public func text(ofPrompt id: String, at root: String) async -> String? {
        let service = service
        return await offMain { try? service.promptText(root: root, id: id) }
    }

    public func openLayout(at root: String) async {
        let service = service
        await perform { try service.openLayout(root: root) }
    }

    public func changes(at folder: String) -> AsyncStream<Void> {
        let service = service
        return WakeStream.make { relay in
            let watch = service.watch(folder: folder, listener: relay)
            return WatchHandle(drain: { watch.drain() }, stop: { watch.stop() })
        }
    }

    private func setColor(_ color: String?, of folder: String) async {
        let service = service
        await report { try service.setFolderColor(folder: folder, color: color) }
    }

    /// Runs a call that changes something; a refusal goes to the notice chip.
    private func report(_ call: @escaping @Sendable () throws -> Void) async {
        let failure = await offMain { () -> String? in
            do { try call(); return nil } catch { return Self.spoken(error) }
        }
        if let failure { await effects.problem(failure) }
    }

    /// Asks Rust for effects and hands what it allows to the window.
    private func perform(_ call: @escaping @Sendable () throws -> [ProjectEffect]) async {
        let answer = await offMain { () -> Result<[ProjectEffect], Error> in Result { try call() } }
        switch answer {
        case let .success(found): await effects.perform(found.compactMap(PanelEffect.init))
        case let .failure(error): await effects.problem(Self.spoken(error))
        }
    }

    private static func spoken(_ error: Error) -> String { (error as? ProjectFailure)?.spoken ?? "\(error)" }
}
