import Foundation
import Testing

/// Where the Notices feature is off, a warning about the app is not a chip: it is the banner over the pane, with Dismiss, until
/// it is dismissed. Its Dismiss is found where the window drew it (`marked banner.dismiss`) and clicked.
extension EndToEnd {
    @Test("with Notices off a sticky warning is the pane's banner, and its Dismiss takes it away")
    @MainActor func noticeBannerWhenNoticesOff() async throws {
        try await Instance.with { app in
            await app.flags(["notices": false])
            // The session host's warning, the one that stays (terminal_view/hosted.rs:341: the banner where the chip is off).
            await app.send(["notice sticky no-host-to-be-found"])
            let chip = await app.until("notice chip") { $0.contains("hostNotice=no-host-to-be-found") }
            app.expect(chip, contains: "shown=none")
            app.expect(await app.reply("pane-bar"), contains: "banner no-host-to-be-found")
            await app.send(["inspect on"])
            let drawn = Self.frames(await app.until("marked banner.dismiss") { !Self.frames($0).isEmpty })
            guard let dismiss = drawn.first?.rect else { return }
            await app.send(["inspect off"])
            app.expect(await app.reply("click \(Int(dismiss.midX)) \(Int(dismiss.midY))"), contains: "click")
            await app.until("pane-bar") { $0 == "none" }
        }
    }
}
