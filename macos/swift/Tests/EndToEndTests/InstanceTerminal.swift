import CoreGraphics
import Foundation
import Testing

/// What the terminal scenarios do over `Instance`'s commands: a line at the shell, bytes a program writes, the prompt marks a
/// shell integration writes, and where a cell is on the window for the pointer.
extension Instance {
    /// A line typed at the selected terminal's shell and Return (`type` writes the bytes to the program; its `\n` is Return).
    /// A literal backslash-n inside `line` would become Return too: a scenario writes a newline as a separate command instead.
    func shell(_ line: String) async { await send(["type \(line)\\n"]) }

    /// With `/bin/cat` for a shell: the bytes come back as the program's output once Return is typed (the line discipline echoes
    /// them first as text, `^[` for ESC), so a script makes the terminal read any sequence. Escapes are real characters here.
    func emit(_ bytes: String) async { await send(["type \(bytes)\\n"]) }

    /// With `/bin/cat` for a shell: an empty screen with the cursor at its top, by the sequences a program clears with (ED 2,
    /// CUP), which `cat` writes back. ⌘K is not that: it erases the history and sends ^L, which only a shell answers by
    /// redrawing, so a `cat` screen keeps its lines (`Pty::clear_history`: "the screen stays", in the GPUI app too).
    func clearScreen() async {
        await emit("\u{1b}[2J\u{1b}[H")
        await send(["wait-idle 2000"])
    }

    /// ⌃U to the line discipline: what is typed (or a reply the terminal wrote to the program) and not yet sent is dropped.
    func discardLine() async { await send(["type \u{15}"]) }

    /// `/bin/zsh -f` marks its prompts as Wormhole's shell integration does (OSC 133 A prompt, B command, C output, D;status end),
    /// so the terminal has command blocks. Every command after this one is a block.
    func markPrompts() async {
        await shell(#"preexec() { print -n '\e]133;C\a' }"#)
        await shell(#"precmd() { print -n "\e]133;D;$?\a\e]133;A\a" }"#)
        await shell(#"PS1=$'$ %{\e]133;B\a%}'"#)
        await until("blocks") { !$0.isEmpty }
    }

    /// The selected terminal's visible rows once `done` holds for them.
    @discardableResult
    func screen(timeout: TimeInterval = 20, _ done: (String) -> Bool) async -> String { await until("screen", timeout: timeout, done) }

    /// Each terminal's text area as the window's accessibility tree places it (`a11y`: `TextArea "…" … @x,y wxh`), in the window's
    /// points from its top left: what the pointer commands take.
    func terminalFrames(window: Int? = nil) async -> [CGRect] {
        let tree = await reply("a11y", window: window)
        return tree.split(separator: "\n").filter { $0.trimmingCharacters(in: .whitespaces).hasPrefix("TextArea") }.compactMap { line in
            guard let at = line.range(of: " @", options: .backwards) else { return nil }
            let numbers = line[at.upperBound...].split { $0 == "," || $0 == " " || $0 == "x" }.compactMap { Double($0) }
            return numbers.count == 4 ? CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3]) : nil
        }
    }

    /// The middle of a cell of the selected terminal in the window's points: its text area, the 4 pt padding, and the cell's
    /// size from `state` (`cell=WxHpx scale=S`). Nil when one of them is not there.
    func cellPoint(row: Int, column: Int) async -> CGPoint? {
        let state = await reply("state")
        guard let frame = await terminalFrames().first, let cell = Self.value("cell", in: state)?.replacingOccurrences(of: "px", with: ""),
            let scale = Self.value("scale", in: state).flatMap({ Double($0) }), scale > 0
        else { return nil }
        let sides = cell.split(separator: "x").compactMap { Double($0) }
        guard sides.count == 2 else { return nil }
        let padding = 4.0
        return CGPoint(
            x: frame.minX + padding + (Double(column) + 0.5) * sides[0] / scale, y: frame.minY + padding + (Double(row) + 0.5) * sides[1] / scale)
    }

    /// The window's terminals, oldest first, once there are `count` of them; nil (the failure recorded) when they never are, so a
    /// scenario stops instead of reading past the end of a list.
    @discardableResult
    func live(_ count: Int, window: Int? = nil) async -> [Int]? {
        let ids = Listing(await until("rows", window: window) { Listing($0).live.count == count }).live
        return ids.count == count ? ids : nil
    }

    /// The question a test instance holds (`question`): its title, detail and buttons, or "no question is up".
    func question() async -> String { await reply("question") }

    /// Waits for a held question and gives it.
    @discardableResult
    func asked(timeout: TimeInterval = 15) async -> String { await until("question", timeout: timeout) { $0.contains("title=") } }

    /// The windows as `sessions` lists them, one line each (`window=<id> profile=<p> … front=<bool> terminals=[…] … frame=…`).
    func windowLines() async -> [String] { (await reply("sessions")).split(separator: "\n").map(String.init).filter { $0.hasPrefix("window=") } }

    /// The number `window:` takes for the first window in sight of `profile`: its place among the windows in sight, which is
    /// what `cmd<n>` counts (`sessions` lists the parked ones too; `DRIVING.md`, "Which window a command file reaches").
    func window(inProfile profile: String) async -> Int? {
        (await windowLines()).filter { $0.contains("visible=true") }.firstIndex { $0.contains("profile=\(profile) ") }.map { $0 + 1 }
    }

    /// Feature flags for a profile other than the default one, written into its own `config.json` (the map unlocked, as
    /// `flags` writes it); the window `window` of that profile is waited for until it has them.
    func flags(_ wanted: [String: Bool], profile: String, window: Int? = nil) async throws {
        let file = scratch.data.appendingPathComponent("profiles/\(profile)/config.json")
        var json = (try? JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any]) ?? [:]
        var map = json["feature_flags"] as? [String: Any] ?? [:]
        map["unlocked"] = true
        for (key, on) in wanted { map[key] = on }
        json["feature_flags"] = map
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]).write(to: file, options: .atomic)
        await until("flags", window: window) { said in
            let on = Set(said.dropFirst("flags on=".count).split(separator: ",").map(String.init))
            return wanted.allSatisfy { on.contains($0.key) == $0.value }
        }
    }

    /// A reply without the isolation marks Fluent puts around a number or a name (U+2068, U+2069), so words compare as read.
    nonisolated static func plain(_ text: String) -> String { text.filter { $0 != "\u{2068}" && $0 != "\u{2069}" } }

    /// The block listing's lines (`n: lines a..=b[ output c..=d] | state | command`) whose command is `command`.
    nonisolated static func blocks(_ listing: String, running command: String) -> [(number: Int, line: String)] {
        listing.split(separator: "\n").compactMap { line in
            let parts = line.components(separatedBy: " | ")
            guard parts.count >= 3, parts.last?.trimmingCharacters(in: .whitespaces) == command, let number = Int(line.prefix { $0 != ":" }) else { return nil }
            return (number, plain(String(line)))
        }
    }

    /// The first line of the block's output (`output c..=d`), nil when it has none.
    nonisolated static func outputStart(_ line: String) -> Int? {
        guard let range = line.range(of: " output ") else { return nil }
        return Int(line[range.upperBound...].prefix { $0.isNumber })
    }

    /// The text of the last pasteboard reply (`pasteboard "…"`), unquoted as far as a plain text needs.
    func pasteboard() async -> String {
        let line = await reply("pasteboard")
        guard line.hasPrefix("pasteboard \"") else { return "" }
        return String(line.dropFirst("pasteboard \"".count).dropLast())
    }
}
