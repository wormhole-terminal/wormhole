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

    /// A command that answers when it is done, beside the batch (the Studio's open, save and use may ask a question first):
    /// its answer is the line that starts with the command and a colon, whenever it comes.
    func late(_ command: String, timeout: TimeInterval = 20) async -> String {
        let out = outFile(nil)
        let before = (try? Data(contentsOf: out).count) ?? 0
        await send([command])
        let prefix = command + ": "
        let answered: @Sendable () -> String? = {
            guard let data = try? Data(contentsOf: out), data.count >= before else { return nil }
            return String(decoding: data.dropFirst(before), as: UTF8.self).split(separator: "\n").first { $0.hasPrefix(prefix) }.map(String.init)
        }
        let came = await watch(out, timeout: timeout) { answered() != nil }
        expect(came, "no answer to `\(command)` in \(Int(timeout)) s")
        return answered() ?? ""
    }

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
}
