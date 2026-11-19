import Foundation
import Testing

/// Read-outs that answered for their argument or for a copy of a rule now read the view's own state (X12): `studio-complete`
/// types into the value field and reads the list the field says it shows, `profile-chip` reads the function the chip draws
/// from. (`theme-cards` and its box: `ThemeCardsTests`; `cursor`: `CursorDrawTests` and `CursorTests`.)
extension EndToEnd {
    @Test("studio-complete reads the value field: what was typed stays in it and is read again; a metric has no field to read")
    @MainActor func studioCompleteReadsTheField() async throws {
        try await Instance.with { app in
            await app.send(["studio"])
            await app.until("studio-select primary") { !$0.hasPrefix("error") }
            let typed = Readout(await app.reply("studio-complete mix(pri"))
            app.expect(typed.all("item").contains { $0.label.hasPrefix("primary") }, "primary names:\n\(typed.text)")
            // Nothing typed: the field still holds `mix(pri` and shows the same list (a read-out of its own argument would
            // list nothing for an empty text).
            let again = await app.reply("studio-complete")
            app.expect(again == typed.text, "the field is read as it stands:\n\(again)\nwas:\n\(typed.text)")
            // A metric is edited with a stepper, not a value field: there is no list to read, and none is made up.
            await app.until("studio-select metrics.radius") { !$0.hasPrefix("error") }
            app.expect(await app.reply("studio-complete mi"), contains: "error: no value field")
        }
    }

    @Test("profile-chip says what the chip's own rule draws: hidden with one profile, the profile it names with two")
    @MainActor func profileChipReadsTheChipsRule() async throws {
        try await Instance.with { app in
            let one = Readout(await app.until("profile-chip") { Readout($0).first("profile-chip") != nil })
            app.expect(one.first("profile-chip")?.has("hidden") == true, "no chip with one profile:\n\(one.text)")
            app.expect(one.first("profile-chip")?["named"] == nil, "it names none:\n\(one.text)")
            await app.send(["profile-new Work"])
            let two = Readout(await app.until("profile-chip") { Readout($0).first("profile-chip")?["named"] != nil })
            app.expect(two.first("profile-chip")?["named"] == "Work", "the chip names the profile Wormhole is in:\n\(two.text)")
            app.expect(two.first("profile-chip")?.has("hidden") == false, "and is not hidden:\n\(two.text)")
        }
    }
}
