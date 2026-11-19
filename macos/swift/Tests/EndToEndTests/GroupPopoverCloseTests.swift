import Foundation
import Testing

/// The group's popover closing: what it keeps, and that the next one asked for is not taken down with it.
extension EndToEnd {
    @Test("a name typed in the group's popover is kept when the popover is closed without ↩, and the popover is gone")
    @MainActor func groupPopoverClosedKeepsTheTypedName() async throws {
        try await Instance.with { app in
            guard let first = (await app.rows()).live.first, let group = await Self.grouped(app, [first]) else { return }
            await app.send(["group-popover \(group.id)"])
            await app.until("popover") { $0.contains("colours:") }
            await app.send(["popover-type Build"])
            await app.until("popover") { $0.contains("field \"Build\"") }
            await app.send(["popover-close"])
            // AppKit's close keeps the field's text unless ⎋ dropped it (`platform/macos/popover.rs` `finish`).
            let rows = await app.until("rows") { Self.groupLine($0, group.id).contains("\"Build\"") }
            app.expect(Self.groupLine(rows, group.id), contains: "\"Build\"")
            app.expect(await app.until("popover") { $0 == "no popover" }, contains: "no popover")
        }
    }

    @Test("a group's popover asked for again the moment the one before closes is shown, and says what it shows", arguments: [0, 60, 150, 400])
    @MainActor func groupPopoverAskedForAgainWhileClosing(after pause: Int) async throws {
        try await Instance.with { app in
            guard let first = (await app.rows()).live.first, let group = await Self.grouped(app, [first]) else { return }
            await app.send(["group-popover \(group.id)"])
            await app.until("popover") { $0.contains("colours:") }
            await app.send(["popover-colour red"])
            await app.until("rows") { Self.groupLine($0, group.id).contains("coloured red") }
            // The close and the next ask in one go, `pause` ms apart: the first popover is still on its way out.
            await app.send(["popover-close"] + (pause > 0 ? ["delay \(pause)"] : []) + ["group-popover \(group.id)"])
            let again = await app.until("popover") { $0.contains("Red [x]") }
            let window = await app.until("branch-chip state") { $0.contains("list=open") }
            app.expect(again.contains("Red [x]") && window.contains("list=open"), "the popover asked for \(pause) ms after the close:\n\(again)\n\(window)")
            // It stays: what was closing does not take it along (two seconds is longer than a popover's way out).
            await app.send(["delay 2000"])
            let later = await app.send(["popover", "branch-chip state"])
            app.expect(later.first?.contains("Red [x]") == true && later.last?.contains("list=open") == true, "two seconds on:\n\(later.joined(separator: "\n"))")
        }
    }
}
