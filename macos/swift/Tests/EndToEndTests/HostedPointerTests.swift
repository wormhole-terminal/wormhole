import Foundation
import Testing

/// The wheel and the selection in a terminal that runs in a session host. Its view is there before the host has answered
/// (`UniFFIFrameSource(waiting:)`), and it took its way to the engine's pointer and selection once, when it started: for a
/// hosted terminal that was nothing, so the wheel did not scroll, nothing could be selected or copied and ⌘K erased no
/// history (found by `ClearHistoryTests/clearBufferReachesTheHost`). Read from the frame (`scrollback`), the rows drawn and
/// the instance's clipboard.
extension EndToEnd {
    /// `name=` of a `scrollback` reply as a number.
    private static func number(_ name: String, in reply: String) -> Int? { Instance.value(name, in: reply).flatMap { Int($0) } }

    @Test(
        "in a hosted terminal the wheel scrolls the history and a selection is made and copied, after a reattach too",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostedWheelAndSelection() async throws {
        try await Instance.with(hosts: true) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.send(["type " + (1...100).map { "early-\($0)\\n" }.joined()])
            await app.screen { $0.contains("early-100") }
            await app.until("scrollback") { (Self.number("history", in: $0) ?? 0) >= 100 }

            for round in ["attached at launch", "attached again"] {
                // The wheel, turned up: the view goes back into the history, to its first line; turned down, to the live screen.
                var up = ["", ""]
                let start = Date()
                repeat {
                    up = Array((await app.send(["scroll 700 400 -9000", "frames 2", "delay 150", "scrollback", "screen"])).suffix(2))
                } while (Self.number("offset", in: up[0]) ?? 0) == 0 && Date().timeIntervalSince(start) < 15
                app.expect((Self.number("offset", in: up[0]) ?? 0) > 0, "\(round): the wheel scrolled back: \(up[0])")
                app.expect(up[1].split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == "early-1" }, "\(round): the first line is drawn:\n\(up[1])")
                await app.send(["scroll 700 400 9000", "frames 2"])
                await app.until("scrollback") { Self.number("offset", in: $0) == 0 }

                // A row's first seven cells selected and copied: the text of those cells.
                let screen = await app.reply("screen")
                let row = String(screen.split(separator: "\n", omittingEmptySubsequences: false).dropFirst(3).first ?? "")
                await app.send(["pasteboard-set nothing-yet", "select-cells 3 0 3 6", "frames 2", "key cmd-c"])
                let copied = await app.pasteboard()
                app.expect(!row.isEmpty && copied == String(row.prefix(7)), "\(round): the selected cells were copied: \"\(copied)\" of \"\(row)\"")
                // Select All, copied: the screen's last line is in it.
                await app.send(["key cmd-a", "frames 2", "key cmd-c"])
                app.expect(await app.pasteboard(), contains: "early-100")

                guard round == "attached at launch" else { break }
                app.expect(await app.reply("detach"), contains: "detached 1")
                await app.until("sessions") { $0.contains("detached(hostID:") }
                await app.send(["reattach"])
                await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
                await app.screen { $0.contains("early-100") }
            }
        }
    }
}
