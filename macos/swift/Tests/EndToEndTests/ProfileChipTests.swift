import Foundation
import Testing

/// The profile chip in the sidebar's title row: drawn only while there are two profiles to choose between, naming the one
/// Wormhole is in; its menu's profile switches Wormhole to it. Read from the views (`marked profile.chip`) and from what the chip
/// is given (`profile-chip`); the menu itself is AppKit's and is not opened here (its tracking loop would hold the instance).
extension EndToEnd {
    @Test("the chip shows once a second profile exists, names the current one, and its menu's choice switches profile")
    @MainActor func profileChipDrawnAndSwitches() async throws {
        try await Instance.with { app in
            await app.send(["inspect on"])
            await app.until("marked") { Self.frames($0).contains { $0.id == "sidebar.row" } }
            // One profile: no chip (sidebar/profile_chip.rs:65, drawn only with two or more).
            app.expect(!Self.frames(await app.reply("marked")).contains { $0.id == "profile.chip" }, "no chip with one profile")
            let one = await app.reply("profiles")
            app.expect(one.split(separator: "\n").count == 1 && one.hasPrefix("*"), "one profile, the one Wormhole is in:\n\(one)")

            await app.send(["profile-new Work"])
            let chip = await app.until("marked") { Self.frames($0).contains { $0.id == "profile.chip" } }
            guard let frame = Self.frames(chip).first(where: { $0.id == "profile.chip" })?.rect else { return }
            app.expect(frame.minY < 40, "the chip is in the 38-point title row:\n\(chip)")
            let given = Readout(await app.reply("profile-chip"))
            app.expect(given.all("choice").count == 2, "both profiles on offer:\n\(given.text)")
            app.expect(given.all("choice").first { $0.has("current") }?.label == "Default", "the chip names Default:\n\(given.text)")
            guard let work = given.all("choice").firstIndex(where: { $0.label == "Work" }) else { return }

            // The menu's row for Work runs Switch Profile (profile_chip.rs:172): Wormhole is in Work after it.
            await app.send(["profile-chip switch \(work + 1)"])
            await app.until("profiles") { $0.split(separator: "\n").contains { $0.hasPrefix("*") && $0.contains("\"Work\"") } }
        }
    }
}
