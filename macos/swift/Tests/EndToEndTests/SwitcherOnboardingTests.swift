import Foundation
import Testing

/// The group switcher's presses (`switcher open`, `switcher-press …`) and a card's popover, and the first run's Look cards and
/// trust card (`onboarding-press theme <name>|trust|later`), with agents found on the instance's own agent path
/// (`Instance.with(agents:)`). TO-VERIFY.md lines 174, 276, 491, 492, 722, 1116.
extension EndToEnd {
    @Test("the switcher's New group makes a terminal in a group of its own (174)")
    @MainActor func switcherNewGroup() async throws {
        try await Instance.with { app in
            await app.send(["set terminals_in=\"tabs\"", "key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let first = two.live.first else { return }
            await app.send(["group \(first)"])
            let one = Listing(await app.until("rows") { Listing($0).groups.count == 1 })
            app.expect(await app.reply("switcher-press new-group"), contains: "switcher-press new-group")
            let after = Listing(await app.until("rows") { Listing($0).groups.count == 2 })
            app.expect(after.live.count == one.live.count + 1, "a new terminal:\n\(after.text)")
            app.expect(after.groups.contains { $0.count == 1 && !one.groups.map(\.id).contains($0.id) }, "in a group of its own:\n\(after.text)")
        }
    }

    @Test("Rename… of a group while its terminals are tabs opens its card's popover with the name in its field (491, 492)")
    @MainActor func switcherCardPopover() async throws {
        try await Instance.with { app in
            await app.send(["set terminals_in=\"tabs\"", "key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let first = two.live.first else { return }
            await app.send(["group \(first)"])
            guard let group = Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first else { return }
            await app.send(["switcher open", "wait-idle 3000", "group-menu \(group.id)", "menu-pick Rename…"])
            let popover = await app.until("popover") { $0.contains("field") }
            app.expect(popover, contains: "\"\(group.name)\"")
            app.expect(popover.split(separator: "\n").first { $0.hasPrefix("colours:") }?.contains("No colour") == true, "the colours, No colour first:\n\(popover)")
            app.expect(popover, contains: "- New terminal in group")
            await app.send(["popover-close"])
        }
    }

    @Test("a Look card of a light theme under Match system sets theme_light and leaves the mode (276)")
    @MainActor func onboardingLookCard() async throws {
        try await Instance.with { app in
            await app.send(["set theme_mode=\"system\""])
            let look = await Self.onboarding(app, at: "look")
            guard let light = look.all("theme").first(where: { $0.label.contains("Light") }) else { return app.fail("a light card:\n\(look.text)") }
            app.expect(await app.reply("onboarding-press theme \(light.label)"), contains: "onboarding-press theme")
            await app.until("get theme_light") { $0.contains(light.label) || $0.contains(light.words.first ?? "-") }
            app.expect(await app.setting("theme_mode"), contains: "system")
            await app.send(["onboarding-step close"])
        }
    }

    @Test("with an agent found, the agents step shows the trust card of the picked folder's project; Cancel brings the button back (722, 1116)")
    @MainActor func onboardingTrustCard() async throws {
        try await Instance.with(agents: ["claude"]) { app in
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("first-run-project")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 15}"#])
            try FileManager.default.createDirectory(at: repo.appendingPathComponent("sub"), withIntermediateDirectories: true)
            _ = await Self.onboarding(app, at: "folder")
            // A subfolder of the project: the card is the project's.
            await app.send(["onboarding-pick \(repo.appendingPathComponent("sub").path)", "onboarding-press main", "onboarding-press main"])
            let agents = Readout(await app.until("onboarding-state") { Readout($0).first("trust") != nil })
            app.expect(agents.first("onboarding-state")?.words.first == "agents", "the agents step:\n\(agents.text)")
            app.expect(agents.all("agent").contains { $0.words.first == "claude" }, "the agent on the path:\n\(agents.text)")
            let card = agents.first("trust")
            app.expect(card?["root"]?.hasSuffix("first-run-project") == true, "the project's card:\n\(agents.text)")
            app.expect(card?["button"]?.hasPrefix("Trust with Touch ID") == true, "its button:\n\(agents.text)")
            // WORMHOLE_AUTH=cancel: the question is cancelled and the button comes back.
            await app.send(["onboarding-press trust"])
            await app.until("onboarding-state") { Readout($0).first("trust")?.has("approving") == false }
            app.expect(Readout(await app.reply("onboarding-state")).first("trust")?["button"]?.hasPrefix("Trust with Touch ID") == true, "the card stays, its button back")
            await app.send(["onboarding-press later"])
            await app.until("onboarding-state") { Readout($0).first("trust") == nil }
            await app.send(["onboarding-step close"])
        }
    }

    @Test("an approved trust card goes once the project is trusted (1116)")
    @MainActor func onboardingTrustApproved() async throws {
        try await Instance.with(auth: "allow", agents: ["claude"]) { app in
            await app.flags(["project_folders": true])
            let repo = try app.scratch.repository("approved-project")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 15}"#])
            _ = await Self.onboarding(app, at: "folder")
            await app.send(["onboarding-pick \(repo.path)", "onboarding-press main", "onboarding-press main"])
            await app.until("onboarding-state") { Readout($0).first("trust") != nil }
            await app.send(["onboarding-press trust"])
            await app.until("onboarding-state") { Readout($0).first("onboarding-state")?.words.first == "agents" && Readout($0).first("trust") == nil }
            await app.send(["onboarding-step close"])
        }
    }
}
