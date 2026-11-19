import Foundation
import Testing

/// The secondary windows and the first run, through their models: Session Hosts, Licenses, Settings → Updates in a build with no
/// feed, the first run's Look step, the welcome program's token, and the pictures the tooling draws off screen.
extension EndToEnd {
    @Test("Session Hosts with no host, Licenses searched and chosen, and Check for Updates opening Settings on Updates")
    @MainActor func hostsLicensesUpdates() async throws {
        try await Instance.with { app in
            await app.send(["hosts"])
            await app.until("windows") { $0.contains("Session Hosts visible=true") }
            app.expect(await app.reply("hosts-list"), contains: "no hosts")
            app.expect(await app.reply("hosts-toggle"), contains: "error: no host to toggle")

            await app.send(["licenses"])
            let found = await app.until("licenses-search serde") { !$0.hasPrefix("error") }
            app.expect(found, contains: "selected=serde")
            app.expect(await app.reply("licenses-select serde"), contains: "licenses-select serde ")
            app.expect(await app.reply("licenses-search zz-no-such-package"), contains: "0 of ")
            // Choosing a package the search hides clears the search.
            app.expect(await app.reply("licenses-select serde"), contains: "licenses-select serde ")
            app.expect(await app.reply("licenses-search"), contains: "selected=")

            // A build with no feed: Check for Updates opens Settings on Updates, which says it does not update itself.
            await app.send(["updates"])
            await app.until("settings-rows") { $0.contains("update-channel | ") }
            app.expect(await app.later("update", answer: "update "), contains: "development build")
            app.expect(await app.reply("update-installed 9.9.9"), contains: "error")
        }
    }

    @Test("the first run's Look step walks its questions back, over and to the end, and the welcome refuses a wrong token")
    @MainActor func firstRunAndWelcome() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("onboarding-step look"), contains: "onboarding-step look")
            let answered = await app.until("onboarding-tailor 1") { !$0.hasPrefix("error") }
            app.expect(answered, contains: "onboarding-tailor 1 -> ")
            app.expect(await app.reply("onboarding-tailor back"), contains: "onboarding-tailor back -> ")
            app.expect(await app.reply("onboarding-tailor over"), contains: "onboarding-tailor over -> ")
            app.expect(await app.reply("onboarding-tailor ok"), contains: "onboarding-tailor ok -> ")
            app.expect(await app.reply("onboarding-tailor nonsense"), contains: "error")
            await app.send(["onboarding-step close"])

            let probe = await app.reply(#"welcome-probe - not-a-token {"op":"state"}"#)
            app.expect(probe, contains: "error")
            app.expect(await app.reply("welcome-probe - token not-json"), contains: "error: the operation is not JSON")
        }
    }

    @Test("the status item's nine glyphs and frames of an intro are drawn to the control folder")
    @MainActor func toolingPictures() async throws {
        try await Instance.with { app in
            let icons = await app.reply("menubar-icons bar")
            for state in ["idle", "working", "working-0", "working-3", "needs-you", "failed", "finished"] {
                let file = app.scratch.control.appendingPathComponent("bar-\(state).tiff")
                app.expect(FileManager.default.fileExists(atPath: file.path), "\(file.lastPathComponent) is drawn:\n\(icons)")
            }
            app.expect(await app.reply("menubar-icons ../out"), contains: "status-item-idle.tiff")
            let frames = await app.reply("intro-frames voyage 4")
            app.expect(frames, contains: "intro-frames voyage 4 frames")
            app.expect(await app.reply("intro-frames nothing"), contains: "error")
        }
    }
}
