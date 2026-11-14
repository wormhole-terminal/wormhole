import Foundation
import Testing

/// `perf-profile`, the dev tooling's timing of a window's draws: on an idle instance for a second, the report names each part
/// of a draw and writes its table and its JSON beside the control files. No time is asserted: a number here would measure the
/// machine the suite runs on.
extension EndToEnd {
    /// The parts of a draw the Swift renderer times (`PerfProfile.swift`), each a line of the table and a key of `spans_ms`.
    static let perfSpans = ["fetch", "apply", "assemble", "build", "encode", "total", "wait", "cpu", "gpu"]

    @Test("perf-profile for one second names every part of a draw, in its table and its JSON file")
    @MainActor func perfProfileNamesItsParts() async throws {
        try await Instance.with { app in
            // The report goes to `<name>.txt` and `<name>.json` in the control folder (GPUI devctl/profiling.rs:9).
            let table = await app.reply("perf-profile 1 idle")
            app.expect(table.hasPrefix("perf-profile draws="), "the table's head:\n\(table)")
            app.expect((Instance.value("draws", in: table).flatMap { Int($0) } ?? 0) > 3, "it drew for a second:\n\(table)")
            let lines = table.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
            for span in Self.perfSpans { app.expect(lines.contains { $0.hasPrefix(span + " ") }, "a line for \(span):\n\(table)") }
            app.expect(table, contains: "allocations per draw by part:")

            let json = app.scratch.control.appendingPathComponent("idle.json")
            app.expect(await app.waitFor(json), "the JSON report: \(json.path)")
            guard let data = try? Data(contentsOf: json), let report = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                let spans = report["spans_ms"] as? [String: Any]
            else { return app.fail("no spans_ms in \(json.path)") }
            app.expect(Set(spans.keys) == Set(Self.perfSpans), "the JSON's parts: \(spans.keys.sorted())")
            app.expect(FileManager.default.fileExists(atPath: app.scratch.control.appendingPathComponent("idle.txt").path), "the table's file")
        }
    }
}
