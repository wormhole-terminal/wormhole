import Foundation
import Testing

/// What a program is told in a window of another profile than the one Wormhole is in: the colours of that window's own
/// profile, not the shown one's, and a change of that profile's theme alone. With `cat`: the terminal's answer to OSC 11 is
/// echoed on the screen; `window-look` says the background each terminal is drawn with.
extension EndToEnd {
    /// The terminal of window `window` asked for its background on a clean line: the `rgb:` it answered, as `#rrggbb`.
    @MainActor private static func answered(_ app: Instance, window: Int) async -> String? {
        await app.send(["type \u{15}\u{1b}[2J\u{1b}[H\\n", "wait-idle 1500", "type \u{15}", "type \u{1b}]11;?\u{07}\\n"], window: window)
        let screen = await app.until("screen", window: window) { $0.contains("rgb:") }
        await app.send(["type \u{15}"], window: window)
        guard let range = screen.range(of: "rgb:", options: .backwards) else { return nil }
        let parts = screen[range.upperBound...].prefix(14).split(separator: "/").map { String($0.prefix(2)) }
        return parts.count == 3 ? "#" + parts.joined().lowercased() : nil
    }

    @Test("a program in a window of another profile is told that profile's background, and hears a change of that profile's theme alone")
    @MainActor func otherProfileTerminalAnswersItsOwnColours() async throws {
        try await Instance.with { app in
            await app.send([#"set theme_mode="dark""#])
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            let theme = "Dracula"
            await app.send([#"set theme_mode="dark""#, #"set theme="\#(theme)""#])
            let file = app.scratch.data.appendingPathComponent("profiles/\(work)/config.json")
            app.expect(await app.watch(file.deletingLastPathComponent(), timeout: 15) { ((try? String(contentsOf: file, encoding: .utf8)) ?? "").contains(theme) }, "Work's file has its theme")
            await app.send(["profile-switch default"])
            await app.until("sessions") { $0.contains("profile=default visible=true") }
            await app.send(["profile-open \(work)"])
            await app.until("sessions") { $0.split(separator: "\n").contains { $0.contains("profile=\(work) ") && $0.contains("visible=true") } }
            guard let other = await app.window(inProfile: work), let mine = await app.window(inProfile: "default") else {
                return app.fail("a window of each profile in sight:\n\(await app.reply("sessions"))")
            }
            await app.until("window-look", window: other) { Readout($0).first("window-look")?["theme"] == theme }

            // Each terminal answers with the background it is drawn with, and they differ.
            let drawnThere = Readout(await app.reply("window-look", window: other)).first("window-look")?["terminal"]
            let drawnHere = Readout(await app.reply("window-look", window: mine)).first("window-look")?["terminal"]
            let there = await Self.answered(app, window: other), here = await Self.answered(app, window: mine)
            app.expect(there != nil && there == drawnThere, "Work's terminal answers Work's background: \(there ?? "-") drawn \(drawnThere ?? "-")")
            app.expect(here != nil && here == drawnHere, "Default's terminal answers Default's: \(here ?? "-") drawn \(drawnHere ?? "-")")
            app.expect(there != here, "the two profiles' backgrounds differ: \(there ?? "-") and \(here ?? "-")")

            // Work's theme changed in its own file: its terminal answers the new one, Default's what it did.
            var json = (try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any]) ?? [:]
            json["theme"] = "Nord"
            try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]).write(to: file, options: .atomic)
            await app.until("window-look", window: other) { Readout($0).first("window-look")?["theme"] == "Nord" }
            let changed = Readout(await app.reply("window-look", window: other)).first("window-look")?["terminal"]
            var again: String?
            let start = Date()
            repeat { again = await Self.answered(app, window: other) } while again != changed && Date().timeIntervalSince(start) < 15
            app.expect(again == changed && again != there, "Work's terminal answers its new theme's background: \(again ?? "-") (was \(there ?? "-"))")
            app.expect(await Self.answered(app, window: mine) == here, "Default's terminal answers as before")
        }
    }
}
