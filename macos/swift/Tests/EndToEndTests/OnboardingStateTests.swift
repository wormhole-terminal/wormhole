import Foundation
import Testing

/// The first run's card read with `onboarding-state` (`Readout`'s shape), driven by `onboarding-step`, `onboarding-pick` (a
/// folder dropped on the card) and `onboarding-press` (its buttons). TO-VERIFY.md lines 723, 724, 725. The trust card (722,
/// 1116) is on the agents step only, which a test instance never reaches: its agent search is empty (`WORMHOLE_AGENT_PATH=`),
/// so the third step is the primer.
extension EndToEnd {
    @MainActor static func onboarding(_ app: Instance, at step: String) async -> Readout {
        await app.send(["onboarding-step \(step)"])
        return Readout(await app.until("onboarding-state") { Readout($0).first("onboarding-state")?.words.first == step })
    }

    @Test("the folder step lists the places with Home as ~, marks a dropped folder, and its buttons say Back, Continue, Skip setup (723)")
    @MainActor func onboardingFolderStep() async throws {
        try await Instance.with { app in
            let state = await Self.onboarding(app, at: "folder")
            app.expect(state.first("onboarding-state")?["number"] == "1", "step 1:\n\(state.text)")
            app.expect(state.first("card")?["counter"]?.contains("3") == true, "the counter counts three steps:\n\(state.text)")
            app.expect(state.line("button", "main")?.label == "Continue", "the main button goes on:\n\(state.text)")
            app.expect(state.line("button", "skip")?.label == "Skip setup", "Skip setup:\n\(state.text)")
            let places = state.all("place").filter { $0.words.first == "place" }
            app.expect(places.count >= 3 && places.contains { $0["detail"] == "~" }, "Home, Desktop, Documents:\n\(state.text)")
            // Recent folders, when the profile has any, come first.
            if let firstRecent = state.lines.firstIndex(where: { $0.kind == "place" && $0.words.first == "recent" }),
                let firstPlace = state.lines.firstIndex(where: { $0.kind == "place" && $0.words.first == "place" })
            {
                app.expect(firstRecent < firstPlace, "Recents before Places:\n\(state.text)")
            }

            let folder = app.scratch.home.appendingPathComponent("dropped")
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            await app.send(["onboarding-pick \(folder.path)"])
            let picked = Readout(await app.until("onboarding-state") { Readout($0).first("onboarding-state")?["folder"]?.hasSuffix("dropped") == true })
            app.expect(!picked.all("place").contains { $0.has("picked") }, "a folder that is no place is marked on Open folder, not on a place:\n\(picked.text)")
            await app.send(["onboarding-step close"])
        }
    }

    @Test("with no agent on PATH the third step is the primer, its things to try with their keys, Jot a note with Notes on (724)")
    @MainActor func onboardingPrimer() async throws {
        try await Instance.with { app in
            let state = await Self.onboarding(app, at: "primer")
            app.expect(state.first("onboarding-state")?.has("last") == true, "the primer is the last step:\n\(state.text)")
            app.expect(state.line("button", "main")?.label == "Start", "its main button starts:\n\(state.text)")
            let things = state.all("primer")
            app.expect(things.count >= 4 && things.allSatisfy { !($0["keys"] ?? "").isEmpty }, "the things to try with their keys:\n\(state.text)")
            app.expect(things.contains { $0.label.hasPrefix("Jot a note") }, "Notes is on: Jot a note is listed:\n\(state.text)")
            await app.send(["onboarding-step close"])
        }
    }

    @Test("a folder picked on the first run and Start: the window's one untouched terminal moves there (725)")
    @MainActor func onboardingFinishInFolder() async throws {
        try await Instance.with { app in
            let before = await app.rows()
            app.expect(before.live.count == 1, "one untouched terminal:\n\(before.text)")
            let folder = app.scratch.home.appendingPathComponent("first-folder")
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            _ = await Self.onboarding(app, at: "folder")
            await app.send(["onboarding-pick \(folder.path)"])
            await app.until("onboarding-state") { Readout($0).first("onboarding-state")?["folder"]?.hasSuffix("first-folder") == true }
            // Continue to the look, to the third step (the primer once the search is done), then Start.
            await app.send(["onboarding-press main"])
            await app.until("onboarding-state") { Readout($0).first("onboarding-state")?.words.first == "look" }
            await app.send(["onboarding-press main"])
            await app.until("onboarding-state") { Readout($0).first("onboarding-state")?.has("last") == true }
            await app.send(["onboarding-press main"])
            let after = Listing(await app.until("rows") { Listing($0).rows.contains { $0.folder.hasSuffix("first-folder") } })
            app.expect(after.live.count == 1, "the terminal was replaced, not added to:\n\(after.text)")
        }
    }
}
