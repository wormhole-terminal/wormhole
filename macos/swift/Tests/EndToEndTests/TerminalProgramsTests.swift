import Foundation
import Testing

/// What a program asks of the terminal and what it is answered. With `/bin/cat` the bytes typed come back as its output (`emit`),
/// and the terminal's reply goes back to the program's input, where the line discipline echoes it on the screen (`^[` for
/// ESC): so a reply is read with `screen`, then dropped (`discardLine`) before `cat` would write it back.
extension EndToEnd {
    private static let esc = "\u{1b}", bel = "\u{07}"

    /// The last `rgb:rrrr/gggg/bbbb` on the screen.
    private static func lastColour(_ screen: String) -> String? {
        guard let range = screen.range(of: "rgb:", options: .backwards) else { return nil }
        return String(screen[range.lowerBound...].prefix(18))
    }

    /// Asks for the background (OSC 11) and gives the answer once it is on the screen.
    @MainActor private static func background(_ app: Instance) async -> String? {
        await app.clearScreen()
        await app.discardLine()
        await app.emit("\(esc)]11;?\(bel)")
        let shown = await app.screen { lastColour($0) != nil }
        await app.discardLine()
        return lastColour(shown)
    }

    @Test("colour queries are answered in the drawn theme's colours, the appearance is reported, and a program's own colours reset")
    @MainActor func colourQueries() async throws {
        try await Instance.with { app in
            await app.send(["appearance dark"])
            let dark = await Self.background(app)
            app.expect(dark != nil, "OSC 11 is answered")
            await app.emit("\(Self.esc)[?996n")
            await app.screen { $0.contains("997;1n") }
            await app.discardLine()

            await app.send(["appearance light"])
            let light = await Self.background(app)
            app.expect(light != nil && light != dark, "the light theme's background is answered: \(dark ?? "-") then \(light ?? "-")")
            await app.emit("\(Self.esc)[?996n")
            await app.screen { $0.contains("997;2n") }
            await app.discardLine()

            // Mode 2031: the program hears the change unasked.
            await app.clearScreen()
            await app.emit("\(Self.esc)[?2031h")
            await app.send(["appearance dark"])
            await app.screen { $0.contains("997;1n") }
            await app.discardLine()

            // A program's own background is what OSC 11 answers until OSC 111 gives the theme's back.
            await app.emit("\(Self.esc)]11;#203040\(Self.bel)")
            let own = await Self.background(app)
            app.expect(own?.lowercased().hasPrefix("rgb:2020/3030/4040") == true, "the program's colour is answered: \(own ?? "-")")
            await app.emit("\(Self.esc)]111\(Self.bel)")
            let back = await Self.background(app)
            app.expect(back == dark, "OSC 111 gave the theme's background back: \(back ?? "-") (theme \(dark ?? "-"))")
        }
    }

    @Test("window operations are refused while allow_window_ops is off, then resize, report, minimise and zoom the window")
    @MainActor func windowOperations() async throws {
        try await Instance.with { app in
            let e = Self.esc
            await app.send(["wait-idle 3000"])
            let before = Instance.value("grid", in: await app.reply("state"))
            await app.emit("\(e)[8;24;80t")
            await app.send(["delay 500"])
            app.expect(Instance.value("grid", in: await app.reply("state")) == before, "nothing resizes while window operations are off")

            // `set` writes config.json; the app reads it back through its watcher a moment later.
            await app.send(["set allow_window_ops=true"])
            await app.until("get allow_window_ops") { $0.contains("true") }
            await app.discardLine()
            // A terminal takes the switch when it starts (`SpawnOptions.allow_window_ops`, in the GPUI app too): the one
            // that was open keeps refusing, a new one may act on the window.
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.emit("\(e)[8;24;80t")
            await app.until("state") { Instance.value("grid", in: $0) == "80x24" }
            // (80 x 24 and 90 x 28: sizes a window fits on any display the suite runs on with both side columns open; the
            // core's frame for a resize stays on the screen.)
            // CSI 18 t reports the size in cells.
            await app.clearScreen()
            await app.discardLine()
            await app.emit("\(e)[18t")
            await app.screen { $0.contains("8;24;80t") }
            await app.discardLine()

            // Two resizes at once: the second is worked out from the first's size, not the old one.
            await app.emit("\(e)[8;24;80t\(e)[8;28;90t")
            await app.until("state") { Instance.value("grid", in: $0) == "90x28" }

            // CSI 13 t reports where the window is; CSI 3 t moves it, and the report follows.
            await app.clearScreen()
            await app.discardLine()
            await app.emit("\(e)[13t")
            let first = await app.screen { $0.contains("[3;") }
            await app.discardLine()
            await app.emit("\(e)[3;100;80t")
            await app.clearScreen()
            await app.discardLine()
            await app.emit("\(e)[13t")
            let moved = await app.screen { $0.contains("[3;") }
            await app.discardLine()
            let report = { (text: String) in text.range(of: "[3;", options: .backwards).map { String(text[$0.lowerBound...].prefix(16)) } }
            app.expect(report(first) != report(moved), "the window moved: \(report(first) ?? "-") then \(report(moved) ?? "-")")

            // CSI 2 t minimises, CSI 1 t brings it back; CSI 9;1 t zooms (maximised), CSI 9;0 t restores.
            await app.emit("\(e)[2t")
            await app.until("sessions") { $0.contains(" minimised") }
            await app.emit("\(e)[1t")
            await app.until("sessions") { !$0.contains(" minimised") }
            await app.emit("\(e)[9;1t")
            await app.until("sessions") { $0.contains("frame=maximized") }
            await app.emit("\(e)[9;0t")
            await app.until("sessions") { $0.contains("frame=windowed") }
        }
    }

    @Test("a program's copy (OSC 52) reaches the clipboard with a chip naming it, waits behind a clipboard question, and goes to the bar without the chip")
    @MainActor func programClipboard() async throws {
        try await Instance.with { app in
            let e = Self.esc, b = Self.bel
            await app.flags(["notices": true])
            await app.emit("\(e)]52;c;aGk=\(b)")
            await app.until("pasteboard") { $0 == "pasteboard \"hi\"" }
            let chip = Instance.plain(await app.until("notice chip") { !$0.contains("shown=none") })
            app.expect(chip, contains: "copied 2")

            // Without the notice chip the words go to the terminal's own bar.
            await app.flags(["notices": false])
            await app.emit("\(e)]52;c;Ynll\(b)")
            await app.until("pasteboard") { $0 == "pasteboard \"bye\"" }
            await app.until("pane-bar") { $0.hasPrefix("notice") }

            // A program reading the clipboard is asked about first (`clipboard_read` is ask); a copy made meanwhile waits for it.
            await app.send(["delay 4500"])
            await app.emit("\(e)]52;c;?\(b)")
            await app.until("pane-bar") { $0.hasPrefix("question") }
            await app.emit("\(e)]52;c;b2s=\(b)")
            await app.until("pasteboard") { $0 == "pasteboard \"ok\"" }
            app.expect(await app.reply("pane-bar"), contains: "question")
            await app.send(["clipboard-answer deny"])
            await app.until("pane-bar") { $0.hasPrefix("notice") }
        }
    }

    @Test("without the chip, a copy by a program in a pane that is not selected goes to that pane's bar")
    @MainActor func copyInAnotherPane() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.flags(["notices": false])
            guard let first = await app.live(1)?.first else { return }
            await app.send(["key cmd-d"])
            guard let two = await app.live(2), let second = two.first(where: { $0 != first }) else { return }
            await app.shell(#"/bin/sleep 2; printf '\e]52;c;aGk=\a'"#)
            await app.send(["select \(first)"])
            await app.until("pasteboard") { $0 == "pasteboard \"hi\"" }
            app.expect(!(await app.reply("pane-bar")).hasPrefix("notice"), "the selected pane's bar says nothing of it")
            await app.send(["select \(second)"])
            app.expect(await app.reply("pane-bar"), contains: "notice")
        }
    }

    @Test("a smaller scrollback setting holds for terminals opened after it; those opened before keep theirs")
    @MainActor func scrollbackSize() async throws {
        try await Instance.with(shell: .zsh) { app in
            /// The first number on the screen once the view is scrolled to the top of the history.
            @MainActor func top() async -> Int? {
                await app.send(["scroll 100000", "delay 200"])
                let screen = await app.reply("screen")
                return screen.split(separator: "\n").lazy.compactMap { Int($0.trimmingCharacters(in: .whitespaces)) }.first
            }
            await app.send(["set scrollback_lines=1000", "key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("/usr/bin/seq 1 5000")
            await app.screen(timeout: 30) { $0.contains("5000") }
            let newer = await top()
            app.expect((newer ?? 0) > 3800 && (newer ?? 0) < 4200, "the oldest line kept is about 4000: \(newer.map(String.init) ?? "none")")

            guard let first = Listing(await app.reply("rows")).live.first else { return }
            await app.send(["select \(first)"])
            await app.shell("/usr/bin/seq 1 5000")
            await app.screen(timeout: 30) { $0.contains("5000") }
            let older = await top()
            app.expect((older ?? 5000) < 3800, "the terminal opened before keeps its own size: \(older.map(String.init) ?? "none")")
        }
    }
}
