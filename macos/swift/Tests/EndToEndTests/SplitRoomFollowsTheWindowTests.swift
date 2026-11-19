import Foundation
import Testing

/// Whether the terminal's menu and the title row offer Split Right follows the room the pane area has, as the window changes size:
/// the core answers from the area, and a view that read the answer only when something else moved kept the answer for the size
/// before. A window 944 points wide (what a runner's screen gives a first window) leaves 234 for the pane between the two
/// columns, too little to split; made wider, the split is offered again with nothing else having happened.
extension EndToEnd {
    @Test("Split Right is off in a window too narrow to split and on again once the window is wide enough, with nothing else moved")
    @MainActor func splitRoomFollowsTheWindow() async throws {
        try await Instance.with(shell: .zsh) { app in
            func splitRight() async -> String? {
                guard let frame = await app.terminalFrames().first else { return nil }
                let menu = await app.reply("right-click \(Int(frame.midX)) \(Int(frame.midY))")
                await app.send(["menu-close"])
                return menu.split(separator: "\n").map(String.init).first { $0.contains(". Split Right") }
            }
            await app.send(["size 944x700", "frames 2"])
            await app.until("geometry") { $0.contains("content=(944.0, 700.0)") }
            let narrow = await app.until("panes") { $0.contains("pane ") }
            let tight = await splitRight()
            app.expect(tight?.contains("[off]") == true, "Split Right is off with the pane this narrow (\(narrow)): \(tight ?? "no such item")")

            await app.size(window: 1)
            let wide = await splitRight()
            app.expect(wide != nil && wide?.contains("[off]") == false, "Split Right is on in the 1600-wide window: \(wide ?? "no such item")")
            // And it splits.
            await app.send(["key cmd-d"])
            await app.until("panes") { $0.split(separator: "\n").filter { $0.hasPrefix("pane ") }.count == 2 }
        }
    }
}
