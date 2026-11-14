import Foundation
import Testing

/// Small things several scenarios do, over `Instance`'s own commands.
extension Instance {
    /// ⌘W on the selected terminal. The core decides whether closing asks (a program running in it does); when the close
    /// question is held for the control folder, it is answered Close, so the scenario reads the same either way.
    func closeSelected() async {
        await send(["key cmd-w"])
        let asked = await reply("question")
        if asked.contains("title=") { await send(["question confirm"]) }
    }

    /// Sends an agent's hook through the app's own hook client until terminal `terminal`'s row says `activity` (`rows`' word:
    /// `working`, `needsInput`, `finished`). The program's name is seen by the app a moment after the terminal opens, and a hook
    /// that comes first is undone when it is, so the hook is sent again between the asks, which pause by the app's own `delay`.
    func report(agent: String, event: String, terminal: Int, becomes activity: String, timeout: TimeInterval = 25) async throws {
        let start = Date()
        var last = ""
        repeat {
            try hook(agent: agent, event: event, terminal: terminal)
            last = (await send(["rows", "delay 250"])).first ?? ""
            if Listing(last).row(terminal)?.activity == activity { return }
        } while Date().timeIntervalSince(start) < timeout
        fail("terminal \(terminal) never became \(activity) after \(event); the last rows were:\n\(last)")
    }

    /// The app's `config.json` (the active profile's, wherever the core keeps it under the data folder), nil before it is written.
    func configFile() -> URL? {
        guard let walker = FileManager.default.enumerator(at: scratch.data, includingPropertiesForKeys: nil) else { return nil }
        for case let url as URL in walker where url.lastPathComponent == "config.json" && url.pathComponents.count - scratch.data.pathComponents.count <= 3 {
            return url
        }
        return nil
    }

    func configText() -> String? { configFile().flatMap { try? String(contentsOf: $0, encoding: .utf8) } }

    /// Waits (pausing by the app's own `delay`) until `config.json` satisfies `done`.
    func waitForConfig(timeout: TimeInterval = 15, _ done: (String) -> Bool) async -> String {
        let start = Date()
        var text = configText() ?? ""
        while !done(text), Date().timeIntervalSince(start) < timeout {
            await send(["delay 200"])
            text = configText() ?? ""
        }
        expect(done(text), "config.json never said what was expected; it is:\n\(text)")
        return text
    }

    /// The first `name=` value of a `state` line (`window=123`).
    static func value(_ name: String, in line: String) -> String? {
        guard let range = line.range(of: name + "=") else { return nil }
        return String(line[range.upperBound...].prefix { $0 != " " && $0 != "\n" })
    }

    /// Feature flags set as Settings → Feature flags sets them, with the map unlocked so a release build reads it too (a
    /// development build has every flag on unless the map says otherwise). Waits until `config.json` holds them.
    func flags(_ wanted: [String: Bool]) async {
        let pairs = (["unlocked": true].merging(wanted) { $1 }).sorted { $0.key < $1.key }.map { "\"\($0.key)\":\($0.value)" }
        await send(["set feature_flags={\(pairs.joined(separator: ","))}"])
        _ = await waitForConfig { text in wanted.allSatisfy { text.range(of: "\"\($0.key)\"\\s*:\\s*\($0.value)", options: .regularExpression) != nil } }
    }

    /// A setting as `get` prints it: `bool(true)`, `number(14.0)`, `string("Nord")`.
    func setting(_ key: String) async -> String {
        let said = await reply("get \(key)")
        return said.components(separatedBy: " = ").dropFirst().joined(separator: " = ")
    }

    /// A terminal in `folder`, selected, once the sidebar lists it beside the ones there were.
    @discardableResult
    func terminal(in folder: URL) async -> Listing {
        let before = (await rows()).live.count
        await send(["new \(folder.path)"])
        return Listing(await until("rows") { Listing($0).live.count == before + 1 })
    }

    /// The selected terminal's screen once it shows `text`.
    @discardableResult
    func screen(showing text: String, timeout: TimeInterval = 20) async -> String {
        await until("screen", timeout: timeout) { $0.contains(text) }
    }

    /// For a command that answers once its work is done, after its batch has moved on (`project …`, `update …`, `probe …`,
    /// `studio-save`, `studio-out`): its answer, from the line that starts with `prefix` up to the next mark, once it is there.
    @discardableResult
    func later(_ command: String, answer prefix: String, window: Int? = nil, timeout: TimeInterval = 30) async -> String {
        let out = outFile(window)
        let before = (try? Data(contentsOf: out).count) ?? 0
        await send([command], window: window)
        let done = await watch(out, timeout: timeout) { Self.answer(in: out, after: before, startingWith: prefix) != nil }
        let said = Self.answer(in: out, after: before, startingWith: prefix) ?? ""
        if !done { fail("no answer starting \"\(prefix)\" to `\(command)` in \(Int(timeout)) s") }
        return said
    }

    nonisolated static func answer(in file: URL, after offset: Int, startingWith prefix: String) -> String? {
        guard let data = try? Data(contentsOf: file), data.count >= offset else { return nil }
        let lines = String(decoding: data.dropFirst(offset), as: UTF8.self).split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        guard let start = lines.firstIndex(where: { $0.hasPrefix(prefix) }) else { return nil }
        return lines[start...].prefix { !$0.hasPrefix("sync e2e") }.joined(separator: "\n")
    }

    /// How many times `text` is on the selected terminal's screen now.
    func count(_ text: String) async -> Int { (await reply("screen")).components(separatedBy: text).count - 1 }
}
