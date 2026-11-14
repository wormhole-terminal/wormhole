import Foundation
import Testing

/// Reply (a banner's or the quick panel's, run as choosing it would: `ambient-run reply`) to a terminal that runs without
/// a view: a launch starts every restored terminal's program and makes a view only for the one it shows. GPUI types the
/// reply into the terminal wherever it is (`window/control/terminals.rs:231-239`). Audit 8: PL03.
extension EndToEnd {
    @Test("a reply reaches a terminal not shown since the launch, and is submitted there (PL03)")
    @MainActor func replyReachesATerminalNotShownYet() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            let (first, second) = (two[0], two[1])
            await app.send(["select \(second)"])
            try await app.restart()
            let back = Listing(await app.until("rows") { Listing($0).live.count == 2 && Listing($0).selected == second })
            app.expect(back.selected == second, "the terminal shown at the quit is the one shown again:\n\(back.text)")
            // The first terminal has not been shown in this run: its program runs, and nothing has made it a view.
            app.expect(await app.reply("ambient-run reply \(first) sent-unseen"), contains: "ambient-run reply")
            // `cat` says a submitted line again: there twice, the answer was typed and Return was pressed after it.
            await app.send(["select \(first)"])
            await app.until("screen") { $0.components(separatedBy: "sent-unseen").count >= 3 }
        }
    }
}
