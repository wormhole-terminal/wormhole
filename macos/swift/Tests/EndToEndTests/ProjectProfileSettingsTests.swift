import Foundation
import Testing

/// A trusted project's `config.json` over the settings of the profile Wormhole is in, once there is more than one profile
/// (`features/project/detect.rs:83-104`): it follows a profile switch, it is judged by the trust of the window's own
/// profile, and a window of another profile in front lays nothing over the shown one. Read through `get` (the settings in
/// force in the profile Wormhole is in) and each window's `project-state`; the owner's question is answered by
/// `WORMHOLE_AUTH=allow`. Audit 8: P07, W12.
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

    @Test("a window of another profile in front lays its project over nothing: the shown profile's settings follow its own window (W12)")
    @MainActor func oddWindowInFrontLaysNoProjectSettings() async throws {
        try await Instance.with(auth: "allow", config: #"{"font_size": 23}"#) { app in
            let nineteen = try app.scratch.repository("sets-nineteen")
            try app.scratch.project(nineteen, files: ["config.json": #"{"font_size": 19}"#])
            let twentyOne = try app.scratch.repository("sets-twenty-one")
            try app.scratch.project(twentyOne, files: ["config.json": #"{"font_size": 21}"#])
            await app.flags(["project_folders": true])
            guard let plain = (await app.rows()).selected else { return app.fail("a first terminal") }
            // Default trusts both projects; the one that sets 21 is in front there.
            var inProject: [Int] = []
            for repo in [nineteen, twentyOne] {
                guard let id = (await app.terminal(in: repo)).selected else { return app.fail("a terminal in \(repo.lastPathComponent)") }
                inProject.append(id)
                await app.send(["tab project"])
                await app.until("project-state") { $0.contains(repo.lastPathComponent) && $0.contains("status=untrusted") }
                await app.send(["project-approve"])
                await app.until("project-state") { $0.contains(repo.lastPathComponent) && $0.contains("status=trusted") }
            }
            await app.until("get font_size") { $0.contains("21") }

            // A window of Work beside Default's, in front, in the project that sets 19, which Work trusts too.
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            try await app.flags(["project_folders": true], profile: work)
            await app.send(["profile-switch default"])
            await app.until("sessions") { $0.contains("profile=default visible=true") }
            await app.until("get font_size") { $0.contains("21") }
            await app.send(["profile-open \(work)"])
            await app.until("sessions") { $0.split(separator: "\n").contains { $0.contains("profile=\(work) ") && $0.contains("visible=true") } }
            guard let odd = await app.window(inProfile: work), let own = await app.window(inProfile: "default") else {
                return app.fail("a Work window beside Default's:\n\(await app.reply("sessions"))")
            }
            await app.send(["new \(nineteen.path)"], window: odd)
            await app.send(["tab project"], window: odd)
            await app.until("project-state", window: odd) { $0.contains("sets-nineteen") && $0.contains("status=untrusted") }
            await app.send(["project-approve"], window: odd)
            await app.until("project-state", window: odd) { $0.contains("status=trusted") }
            let front = (await app.windowLines()).first { $0.contains("front=true") } ?? ""
            app.expect(front.contains("profile=\(work) "), "the Work window is the one in front:\n\(front)")

            // Default's settings go on following Default's own window: its plain terminal, then the project that sets 21.
            // Never the 19 of the project the Work window in front is in.
            await app.send(["select \(plain)"], window: own)
            await app.until("get font_size") { $0.contains("23") }
            await app.send(["select \(inProject[1])"], window: own)
            await app.until("get font_size") { $0.contains("21") }
            app.expect(!(await app.setting("font_size")).contains("19"), "the Work window's project sets nothing in Default")
        }
    }
}
