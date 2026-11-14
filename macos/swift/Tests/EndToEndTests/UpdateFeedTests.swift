import Foundation
import Testing

/// The updater against a feed, as far as a check goes (nothing is downloaded or installed): the instance is started with
/// `--update-feed` naming a `FeedServer` on this machine, whose `latest.json` files are in the scenario's scratch folder.
/// Settings → Updates is read as its page draws it (`update-status`) and its buttons are clicked where they are (`settings-press`).
extension EndToEnd {
    /// A `latest.json` offering `version` for both Mac architectures. The archive's address is the feed's own machine and is
    /// never asked for; the signature has minisign's shape (the feed is refused without one) and signs nothing.
    private static func feed(version: String, notes: String = "", base: String) -> String {
        let signature = [
            "untrusted comment: a test's", Data([0x45, 0x64] + [UInt8](repeating: 0, count: 72)).base64EncodedString(), "trusted comment: a test's",
            Data(repeating: 0, count: 64).base64EncodedString(),
        ].joined(separator: "\n")
        let asset: [String: Any] = ["url": "\(base)/never-asked.tar.gz", "sha256": String(repeating: "ab", count: 32), "signature": signature, "size": 1234]
        let json: [String: Any] = [
            "version": version, "pub_date": "2026-11-01T12:00:00Z", "notes": notes, "platforms": ["darwin-aarch64": asset, "darwin-x86_64": asset],
        ]
        return String(decoding: (try? JSONSerialization.data(withJSONObject: json)) ?? Data(), as: UTF8.self)
    }

    /// Writes `text` as `<channel>/latest.json` of the served folder (nil: the feed is not there).
    private static func publish(_ text: String?, channel: String = "stable", in folder: URL) throws {
        let file = folder.appendingPathComponent(channel).appendingPathComponent("latest.json")
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let text { try Data(text.utf8).write(to: file, options: .atomic) } else { try? FileManager.default.removeItem(at: file) }
    }

    /// Settings open on Updates, with the page's updater (not the development build's note). The page's "Install updates
    /// automatically" switch is clicked off first: with it on (the default) a check installs what it finds instead of offering it.
    @MainActor private static func openUpdates(_ app: Instance) async {
        await app.send(["updates"])
        await app.until("settings-rows") { $0.contains("update-channel | ") }
        await app.until("update-status") { $0.contains("button update.check") }
        await app.settledRow("auto-update")
        await app.send(["settings-press auto-update"])
        _ = await app.waitForConfig { $0.range(of: #""auto_update"\s*:\s*false"#, options: .regularExpression) != nil }
        await app.until("get auto_update") { $0.contains("false") }
    }

    /// A click on a segment of the page's Channel control (Stable on the left, Unstable on the right), until the setting says so.
    @MainActor private static func choose(_ app: Instance, channel: String) async {
        await app.settledRow("update-channel")
        await app.send(["settings-press update-channel at \(channel == "stable" ? 0.25 : 0.75)"])
        await app.until("get update_channel") { $0.contains(channel) }
    }

    /// A click on the page's Check Now, then the page once it is past the check and `done` holds.
    @MainActor @discardableResult private static func check(_ app: Instance, _ done: @escaping (Readout) -> Bool) async -> Readout {
        await app.until("update-status") { $0.contains("button update.check") }
        await app.send(["settings-press update.check"])
        return Readout(await app.until("update-status") { said in
            let page = Readout(said)
            return page.first("update-status")?.has("checking") == false && done(page)
        })
    }

    /// A person's check (`update check`, what Check Now calls) once the updater is past any check of its own, waited for
    /// until the feed was asked again and the check is over.
    @MainActor private static func recheck(_ app: Instance, _ server: FeedServer) async {
        await app.until("update-status") { Readout($0).first("update-status")?.has("checking") == false }
        let asked = server.requests.count
        await app.send(["update check"])
        await app.until("update-status") { Readout($0).first("update-status")?.has("checking") == false && server.requests.count > asked }
    }

    @Test("Check Now against a feed: unreadable, up to date, an update with its notes and buttons; the request names the app and its version only")
    @MainActor func updateFeedPhases() async throws {
        let server = try FeedServer()
        try await Instance.with(feed: server.base) { app in
            let folder = app.scratch.root.appendingPathComponent("feed")
            server.serve(folder)
            await Self.openUpdates(app)

            // Nothing is published: the check fails and the page says why, with Check Now still there.
            let missing = await Self.check(app) { $0.first("update-status")?.has("failed") == true }
            app.expect(missing.first("update-status")?.label.isEmpty == false, "the page says why the check failed:\n\(missing.text)")
            app.expect(missing.line("button", "update.check") != nil, "Check Now stays:\n\(missing.text)")

            // What the server was asked: the stable channel's feed, by a request that says the app's name and version and
            // nothing else about this Mac (`update::http::user_agent`).
            guard let asked = server.requests.last else { return app.fail("the check asked the feed") }
            app.expect(asked.path == "stable/latest.json", "the stable feed was asked: \(asked.path)")
            let agent = asked.headers["user-agent"] ?? ""
            app.expect(agent.hasPrefix("Wormhole-Updater/"), "the request names the updater and its version: \(asked.headers)")
            let named = Set(asked.headers.keys)
            app.expect(named.isSubset(of: ["host", "user-agent", "accept", "accept-encoding", "connection"]), "no header beyond what any request has: \(asked.headers)")
            let version = String(agent.dropFirst("Wormhole-Updater/".count))

            // The feed offers the version that runs: up to date, in green words that say when it checked.
            try Self.publish(Self.feed(version: version, base: server.base), in: folder)
            let same = await Self.check(app) { $0.first("update-status")?.has("up-to-date") == true }
            app.expect(same.first("update-status")?.label ?? "", contains: "Wormhole is up to date.")
            app.expect(same.first("update-status")?.label ?? "", contains: "Checked just now.")
            app.expect(same.all("button").map(\.words) == [["update.check"]], "only Check Now:\n\(same.text)")

            // A newer version: offered by its number with what it brings, Skip This Version and Install Update in place of Check Now.
            try Self.publish(Self.feed(version: "99.0.0", notes: "- Faster folds\n* Fewer **bugs**\n\n", base: server.base), in: folder)
            await app.send(["update check"])
            let offered = Readout(await app.until("update-status") { Readout($0).first("update-status")?.has("available") == true })
            app.expect(offered.first("update-status")?.label == "Wormhole 99.0.0 is available.", "the page offers it:\n\(offered.text)")
            app.expect(offered.first("update-status")?["version"] == "99.0.0", "by its number:\n\(offered.text)")
            app.expect(offered.all("notes").map(\.label) == ["Faster folds", "Fewer **bugs**"], "its notes, a line each without the list mark it starts with:\n\(offered.text)")
            // Skip This Version and Install Update take Check Now's place (once the page's change has run its course).
            await app.until("update-status") { Readout($0).all("button").map(\.words) == [["update.skip"], ["update.install"]] }
            // Nothing was downloaded: the archive was never asked for.
            app.expect(!server.requests.contains { $0.path.contains("never-asked") }, "no download: \(server.requests.map(\.path))")

            // Skip This Version, clicked: the offer goes, and Check Now is back.
            await app.send(["settings-press update.skip"])
            await app.until("update-status") { !$0.contains("update-status available") && Readout($0).all("button").map(\.words) == [["update.check"]] }

            // A feed that is not a feed: the check fails and says so; the next good one is taken again.
            try Self.publish("{ not json", in: folder)
            let broken = await Self.check(app) { $0.first("update-status")?.has("failed") == true }
            app.expect(broken.first("update-status")?.label.isEmpty == false, "the page says the feed could not be read:\n\(broken.text)")
            try Self.publish(Self.feed(version: version, base: server.base), in: folder)
            await Self.check(app) { $0.first("update-status")?.has("up-to-date") == true }

            // Install updates automatically, on again: a check now installs what it finds instead of offering it. This build
            // runs from no app bundle, so there is nothing to replace: the page says so, and the archive is never asked for.
            await app.settledRow("auto-update")
            await app.send(["settings-press auto-update"])
            await app.until("get auto_update") { $0.contains("true") }
            try Self.publish(Self.feed(version: "99.0.0", base: server.base), in: folder)
            let unbundled = await Self.check(app) { $0.first("update-status")?.has("failed") == true }
            app.expect(unbundled.first("update-status")?.label == "Wormhole isn't running from an app bundle.", "the install is refused, in words:\n\(unbundled.text)")
            app.expect(!server.requests.contains { $0.path.contains("never-asked") }, "no download: \(server.requests.map(\.path))")
        }
    }

    @Test("the Preview channel reads its own feed beside the stable one; a prerelease is offered there and never on Stable")
    @MainActor func updateChannels() async throws {
        let server = try FeedServer()
        try await Instance.with(feed: server.base) { app in
            let folder = app.scratch.root.appendingPathComponent("feed")
            server.serve(folder)
            await Self.openUpdates(app)
            try Self.publish(Self.feed(version: "0.0.1", base: server.base), in: folder)
            await Self.check(app) { $0.first("update-status")?.has("up-to-date") == true }
            app.expect(server.requests.allSatisfy { $0.path == "stable/latest.json" }, "Stable asks its own feed only: \(server.requests.map(\.path))")

            // A prerelease in the stable feed is not for someone on Stable.
            try Self.publish(Self.feed(version: "99.1.0-preview.1", base: server.base), in: folder)
            await Self.check(app) { $0.first("update-status")?.has("up-to-date") == true }

            // Preview, clicked on the page: the page's updater changes channel, asks the Preview feed too and takes the prerelease.
            try Self.publish(Self.feed(version: "0.0.1", base: server.base), in: folder)
            try Self.publish(Self.feed(version: "99.2.0-preview.3", base: server.base), channel: "unstable", in: folder)
            // The change of channel checks by itself, and a person's check does again.
            let before = server.requests.count
            await Self.choose(app, channel: "unstable")
            await app.until("update-status") { Readout($0).first("update-status")?["channel"] == "preview" }
            let offered = Readout(await app.until("update-status") { Readout($0).first("update-status")?.has("available") == true })
            app.expect(offered.first("update-status")?["version"] == "99.2.0-preview.3", "the Preview build is offered:\n\(offered.text)")
            await Self.recheck(app, server)
            let asked = Set(server.requests.dropFirst(before).map(\.path))
            app.expect(asked == ["unstable/latest.json", "stable/latest.json"], "Preview reads both feeds: \(asked)")
            await app.until("update-status") { Readout($0).first("update-status")?["version"] == "99.2.0-preview.3" }

            // Back on Stable the offer is gone and only the stable feed is asked.
            await Self.choose(app, channel: "stable")
            await app.until("update-status") { Readout($0).first("update-status")?["channel"] == "stable" }
            await app.until("update-status") { Readout($0).first("update-status")?.has("up-to-date") == true }
            let after = server.requests.count
            await Self.recheck(app, server)
            await app.until("update-status") { Readout($0).first("update-status")?.has("up-to-date") == true }
            app.expect(server.requests.dropFirst(after).allSatisfy { $0.path == "stable/latest.json" }, "Stable asks its own feed only: \(server.requests.dropFirst(after).map(\.path))")
        }
    }

    @Test("an installed update: the page offers the restart, the chip goes to the window in front, its click asks while a command runs, closed it stays closed")
    @MainActor func updateReadyChipAndPage() async throws {
        let server = try FeedServer()
        try await Instance.with(shell: .zsh, feed: server.base) { app in
            server.serve(app.scratch.root.appendingPathComponent("feed"))
            await Self.openUpdates(app)
            // The install itself is pretended (`update-installed`: the updater's phase as a finished install leaves it); from
            // there on everything is the app's own: the follower's chip, the page's words and buttons.
            await app.send(["update-installed 99.0.0"])
            let ready = Readout(await app.until("update-status") { $0.contains("button update.restart") })
            app.expect(ready.first("update-status")?.label == "Wormhole 99.0.0 is ready. Restart to use it.", "the page says it is ready:\n\(ready.text)")
            app.expect(ready.first("update-status")?.has("installed") == true && ready.first("update-status")?.has("pending") == false, "installed, not waiting:\n\(ready.text)")
            // The updater's follower puts the chip in the window in front: sticky, naming the version.
            let chip = await app.until("notice chip", window: 1) { $0.contains("99.0.0") }
            app.expect(chip, contains: "sticky=true")

            // Waiting for idle terminals: the page says so and offers Don't Restart, whose click calls the wait off.
            await app.send(["update-installed 99.0.0 pending"])
            let waiting = Readout(await app.until("update-status") { $0.contains("button update.keep") })
            app.expect(waiting.first("update-status")?.has("pending") == true, "the page says it waits:\n\(waiting.text)")
            await app.send(["settings-press update.keep"])
            await app.until("update-status") { !$0.contains("button update.keep") && Readout($0).first("update-status")?.has("pending") == false }

            // A second window comes to the front: the chip moves there.
            guard let first = Instance.value("window", in: (await app.windowLines()).first ?? "") else { return app.fail("the first window's id") }
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").filter { $0.hasPrefix("window=") }.count == 2 }
            await app.until("notice chip", window: 2) { $0.contains("99.0.0") }
            await app.until("notice chip", window: 1) { $0.contains("shown=none") }

            // A click on the chip restarts; with a command running in a terminal it asks first, naming it. Cancelled, nothing ends.
            await app.shell("sleep 300")
            await app.until("rows", window: 2) { $0.contains("sleep") }
            await app.send(["notice-click body"], window: 2)
            app.expect(await app.asked(), contains: "sleep")
            await app.send(["question cancel"])

            // Its × closes it for this version in every window: the first window coming forward again does not show it.
            await app.send(["notice-click close"], window: 2)
            await app.until("notice chip", window: 2) { $0.contains("shown=none") }
            await app.send(["use-window \(first)", "delay 400"])
            app.expect(await app.reply("notice chip", window: 1), contains: "shown=none")
        }
    }
}
