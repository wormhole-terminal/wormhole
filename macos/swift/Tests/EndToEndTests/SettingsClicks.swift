import Foundation
import Testing

/// A Settings switch clicked where it is drawn, for the scenarios that drive a page by clicks: the row is brought on show
/// (`settings-reveal`), stands still (`settledRow`), and its control is clicked (`settings-press`). A click that misses (the
/// page was still scrolling to the row when the driver aimed) changes nothing, so the switch is clicked again; a click that
/// lands shows in the setting at once, so a switch is never flipped twice.
extension Instance {
    /// Clicks the switch of row `row` until setting `key` is `on`. Whether it got there.
    @discardableResult
    func clickSwitch(_ row: String, key: String, becomes on: Bool) async -> Bool {
        let wanted = "bool(\(on))"
        for _ in 0..<3 {
            await send(["settings-reveal \(row)"])
            await settledRow(row)
            await until("settings-press \(row)") { !$0.hasPrefix("error") }
            for _ in 0..<12 {
                if await setting(key) == wanted { return true }
                await send(["delay 150"])
            }
        }
        fail("the switch of \(row) was clicked and \(key) never became \(on)")
        return false
    }
}
