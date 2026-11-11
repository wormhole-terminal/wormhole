// What the Profile, Agent access and project-marked rows read from the core: the profiles (made and deleted here,
// core's `ProfileList`), the programs let in (the core's desk, revoked through the hook server) and the settings of the
// project in front, which are layered over the person's (`SettingsHandle.setProjectOverrides`).

import SettingsModel
import WormholeBindings

extension RustSettingsHost {
    func refreshProfiles() {
        records.profiles = ((try? settingsProfiles()) ?? []).map(RecordMapping.choice)
        refreshProfile()
    }

    public func makeProfile(name: String, copyOf: String?) -> String? {
        do {
            let id = try settingsCreateProfile(name: name, copyOf: copyOf)
            refreshProfiles()
            // Deferred: the sheet that asked is still on its way out, and the switch rebuilds this window.
            let enter = platform.profileCreated
            Task { @MainActor in enter(id) }
            return nil
        } catch {
            return SettingsFailure.sentence(error)
        }
    }

    public func removeProfile(_ id: String) -> String? {
        do {
            platform.willDeleteProfile(id)
            try settingsDeleteProfile(id: id)
            refreshProfiles()
            return nil
        } catch {
            return SettingsFailure.sentence(error)
        }
    }

    func refreshApprovals() { records.approvals = platform.approvals().map(RecordMapping.approval) }

    public func revoke(_ approval: ApprovalRecord) {
        platform.revokeApproval(approval.number, approval.isStored)
        refreshApprovals()
    }

    /// The project of the terminal in front sets these (`nil`: none, or its settings are off or not trusted): layered
    /// over the person's, with the rows it sets marked, until another terminal comes to the front.
    public func follow(project: ProjectSettingsRecord?) {
        // Keys a project may not change are dropped by the core, and so are not marked.
        try? handle.setProjectOverrides(overrides: project?.json)
        let entries = project?.json == nil ? [] : handle.overriddenEntries()
        records.project = project.flatMap { RecordMapping.project($0, entries: entries) }
        backend.reload()
    }
}
