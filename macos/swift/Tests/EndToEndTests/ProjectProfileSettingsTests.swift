import Foundation
import Testing

/// A trusted project's `config.json` over the settings of the profile Wormhole is in, once there is more than one profile
/// (`features/project/detect.rs:83-104`): it follows a profile switch and is judged by the trust of the window's own
/// profile. Read through `get` (the settings in force in the profile Wormhole is in) and each window's `project-state`;
/// the owner's question is answered by `WORMHOLE_AUTH=allow`. Audit 8: P07.
extension EndToEnd {
    @Test("after a profile switch a project's settings are in force only once that profile trusts it, and again back in the first (P07)")
    @MainActor func projectSettingsFollowTheProfile() async throws {
        try await Instance.with(auth: "allow", config: #"{"font_size": 23}"#) { app in
            let repo = try app.scratch.repository("sets-size")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 19}"#])
            await app.flags(["project_folders": true])
            // Making a profile enters it: from here the settings in force are Work's, and so is the trust that counts.
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            try await app.flags(["project_folders": true], profile: work)
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("sets-size") && $0.contains("status=untrusted") }
            app.expect(!(await app.setting("font_size")).contains("19"), "nothing of a project Work does not trust is in force")
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.until("get font_size") { $0.contains("19") }

            // Back in Default, which never trusted it: the person's own, until Default trusts it too.
            await app.send(["profile-switch default"])
            await app.until("sessions") { $0.contains("profile=default visible=true") }
            await app.until("get font_size") { $0.contains("23") }
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("sets-size") && $0.contains("status=untrusted") }
            app.expect(await app.setting("font_size"), contains: "23")
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.until("get font_size") { $0.contains("19") }
        }
    }
}
