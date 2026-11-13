// The control folder's entry and its tools that take time or look inside (`WORMHOLE_DEVCTL`), after GPUI's devctl:
// `wait-idle`, `perf-profile`, `a11y`, `text`, `bounds`, and the session hosts seen from outside (`hosts-list`,
// `host-kill`). A line that takes time holds the rest of its batch (`DevCtl`).

import AppKit
import DesignSystem
import TerminalMetal
import WorkspaceModel

extension AppController {
    /// One control line: the pointer's and the tools' first (they may take time), then the commands that act at once.
    func run(line: String) async {
        let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
        let command = parts.first ?? "", argument = parts.count > 1 ? parts[1] : ""
        window?.contentView?.layoutSubtreeIfNeeded()
        // While inspect mode picks, `hover` and `click` are the inspector's (they highlight and pin), as in GPUI's devctl.
        if await runInspect(command: command, argument: argument) { return }
        if await runPointer(command: command, argument: argument) { return }
        if await runTooling(command: command, argument: argument) { return }
        if await runDevTools(command: command, argument: argument) { return }
        run(command: line)
    }

    private func runTooling(command: String, argument: String) async -> Bool {
        switch command {
        case "wait-idle": reply(await waitIdle(timeout: Int(argument) ?? 5000))
        case "perf-profile":
            let words = argument.split(separator: " ").map(String.init)
            guard let view = selectedView, let devctl else { reply("error: no terminal is shown"); return true }
            let seconds = words.first.flatMap(Double.init) ?? 5
            // Where the window's terminals run decides what the reader's numbers mean (a hosted one is parsed twice).
            let hosted = hub.state.live.filter { hub.store.hosting(of: $0)?.hostID != nil }.count
            let report = await PerfProfile.run(view: view, seconds: seconds, hosted: hosted, local: hub.state.live.count - hosted)
            let name = words.count > 1 ? words[1] : "perf-profile"
            devctl.append(report.json, to: "\(name).json")
            devctl.append(report.table, to: "\(name).txt")
            reply(report.table)
        case "a11y":
            if argument == "said" {
                reply(Announcements.recent.map { "said \($0.urgent ? "! " : "")\($0.text)" }.joined(separator: "\n"))
            } else {
                let shown = argument.isEmpty ? window : NSApp.windows.first { $0.title == argument }
                reply(shown.map(AccessibilityDump.tree) ?? "error: no window \(argument)")
            }
        case "text":
            // Typed into whatever has the keys (a field, the terminal), a key per character, as the keyboard would.
            argument.forEach(typeCharacter)
            reply("text \(argument)")
        case "bounds":
            let numbers = argument.split(separator: " ").compactMap { Double($0) }
            guard numbers.count >= 2, let window else { reply("error: bounds X Y [levels]"); return true }
            let point = injector.location(numbers[0], numbers[1], in: window)
            // The views the pointer reaches there, deepest first, then what VoiceOver finds there.
            let views = sequence(first: injector.view(at: point, in: window), next: { $0?.superview }).prefix(6).compactMap { $0 }
            reply("views: " + views.map { "\(type(of: $0))" }.joined(separator: " < ") + "\n"
                + AccessibilityDump.at(point, in: window, levels: numbers.count > 2 ? Int(numbers[2]) : 3))
        case "hosts-list", "host-kill":
            let backend = AuxBackends(app: self).hosts()
            if command == "host-kill" { await backend.end(argument) }
            let cards = await backend.report().cards
            reply((command == "host-kill" ? "host-kill \(argument)\n" : "") + (cards.isEmpty ? "no hosts" : cards.map(\.text).joined(separator: "\n\n")))
        default: return false
        }
        return true
    }

    /// GPUI's `wait-idle`: until no terminal of this window is drawing and the window draws the same picture three looks
    /// in a row (lists measuring, a diff being read, an animation), or until `timeout` milliseconds.
    private func waitIdle(timeout: Int) async -> String {
        let start = Date()
        var last: Int?, steady = 0, drawing = false
        while Date().timeIntervalSince(start) * 1000 < Double(timeout) {
            drawing = hub.state.live.contains { hub.terminals.existing($0)?.isLinkRunning == true }
            let picture = window?.contentView.flatMap(Self.fingerprint)
            steady = !drawing && picture != nil && picture == last ? steady + 1 : 0
            if steady >= 3 { return "idle" }
            last = picture
            await pause(50)
        }
        return "wait-idle failed: \(drawing ? "a terminal still draws" : "the window still changes") after \(timeout) ms"
    }

    /// A hash of the view as it draws now.
    private static func fingerprint(_ view: NSView) -> Int? {
        guard let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds), let bytes = bitmap.bitmapData else { return nil }
        view.cacheDisplay(in: view.bounds, to: bitmap)
        var hasher = Hasher()
        hasher.combine(bytes: UnsafeRawBufferPointer(start: bytes, count: bitmap.bytesPerRow * bitmap.pixelsHigh))
        return hasher.finalize()
    }
}
