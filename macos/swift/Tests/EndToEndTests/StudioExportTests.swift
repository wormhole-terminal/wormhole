import Foundation
import Testing

/// Export JSON… and Copy JSON in the Theme Studio. A test instance's save panel and pasteboard are files of its own
/// (`studio-out`: the panel's answer is `<data dir>/studio-out/<the name it suggests>`, the copy is `pasteboard.txt` there), so
/// nothing is written where the person keeps files and their clipboard is never touched.
extension EndToEnd {
    @Test("Export writes the theme under its name in the older format, says where, and the next export starts in that folder after a relaunch too")
    @MainActor func studioExport() async throws {
        try await Instance.with { app in
            let out = app.scratch.data.appendingPathComponent("studio-out")
            await app.send(["studio"])
            await app.until("studio-select") { !$0.hasPrefix("error") }
            await app.later("studio-open new", answer: "studio-open new")
            await app.send(["studio-rename Dusk Mine", "studio-select primary"])
            app.expect(await app.reply("studio-set #ff8800"), contains: "ok=true")

            // Copy JSON: the theme as text where a paste finds it, the person's clipboard untouched; nothing was exported yet,
            // so the save panel would start in the home folder (studio/draft.rs:88).
            let copied = await app.later("studio-out copy-json", answer: "studio-out copy-json")
            app.expect(copied, contains: "copiedJSON")
            app.expect(copied, contains: "from=-")
            let text = (try? String(contentsOf: out.appendingPathComponent("pasteboard.txt"), encoding: .utf8)) ?? ""
            app.expect(text.lowercased().contains("ff8800"), "the copied JSON holds the edit:\n\(text)")
            app.expect(await app.reply("pasteboard"), contains: "pasteboard empty")

            // Export: the name it suggests is the theme's in lower case with dashes (studio/draft.rs:76), the file is the
            // theme as every release reads it, with no version (draft.rs:68), and the notice says where it went (save.rs:304).
            let exported = await app.later("studio-out export", answer: "studio-out export")
            app.expect(exported, contains: "dusk-mine.json")
            app.expect(exported, contains: "exported(")
            let file = out.appendingPathComponent("dusk-mine.json")
            guard let data = try? Data(contentsOf: file), let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                return app.fail("no JSON object at \(file.path)")
            }
            app.expect(json["name"] as? String == "Dusk Mine", "the file names the theme:\n\(json)")
            app.expect(json["version"] == nil, "written in the older format, which has no version:\n\(json)")
            app.expect(String(decoding: data, as: UTF8.self).lowercased().contains("ff8800"), "the file holds the edit")
            // The folder it went to is remembered (save.rs:313): the next export's panel starts there.
            let from = Instance.value("from", in: exported) ?? "-"
            app.expect(from.hasSuffix("/studio-out"), "the next export starts where this one went:\n\(exported)")

            // Remembered by the core's store, so a relaunch keeps it.
            try await app.restart()
            await app.send(["studio"])
            await app.until("studio-select") { !$0.hasPrefix("error") }
            let after = await app.later("studio-out copy-json", answer: "studio-out copy-json")
            app.expect(Instance.value("from", in: after) == from, "after a relaunch the export still starts in \(from):\n\(after)")
        }
    }
}
