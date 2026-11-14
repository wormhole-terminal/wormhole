import Foundation
import Testing

/// The Theme Studio's gallery read from what its views draw (`colour-count` over the Studio window's own picture), never from
/// the draft's table: a token set to a colour no theme has shows up in the specimens that use it and leaves with Undo, in the
/// sidebar's specimens and the right panel's; selecting a token outlines what it is drawn on; a click on an ANSI slot selects
/// its token.
extension EndToEnd {
    /// A colour no bundled theme has, so every pixel of it in the gallery comes from the edit.
    static let galleryMark = "#13f2a7"
    /// The gallery's column in a Studio window of 1344 x 900 (`StudioMetrics`: the list 300 wide on its left, the editor 320
    /// on its right), below the title row: the list's swatch and the editor's well, which show the value too, stay outside.
    static let gallery = "300 80 724 820"

    /// The Studio on a new theme of its own, at its default size, the pointer's commands acting on its window.
    @MainActor static func studioOnNewTheme(_ app: Instance) async {
        await app.send(["studio"])
        await app.until("studio-select") { !$0.hasPrefix("error") }
        await app.later("studio-open new", answer: "studio-open new")
        await app.send(["studio-resize 1344 900", "pointer-window Theme Studio"])
    }

    @MainActor static func pixels(_ said: String) -> Int { Instance.value("pixels", in: said).flatMap { Int($0) } ?? 0 }

    /// Turns the gallery's wheel a screen at a time until `colour` is drawn in it: the `colour-count` line that saw it.
    @MainActor static func scrollGallery(_ app: Instance, until colour: String) async -> String {
        var said = ""
        for y in stride(from: 0, through: 9000, by: 600) {
            await app.send(["studio-scroll \(y)"])
            said = await app.reply("colour-count \(colour) \(gallery)")
            if pixels(said) > 0 { return said }
        }
        return said
    }

    @Test("a token's new colour is drawn by the sidebar's specimens, and Undo takes it out of them again")
    @MainActor func galleryFollowsATokenEdit() async throws {
        try await Instance.with { app in
            await Self.studioOnNewTheme(app)
            // GPUI draws every specimen with the draft's tokens (studio/gallery/view.rs:241).
            await app.send(["studio-area sidebar"])
            app.expect(Self.pixels(await app.reply("colour-count \(Self.galleryMark) \(Self.gallery)")) == 0, "no specimen is that colour before the edit")
            await app.send(["studio-select sidebar.row.selected"])
            app.expect(await app.reply("studio-set \(Self.galleryMark)"), contains: "ok=true")
            // Nothing selected: no outline stands on the rows while they are counted.
            await app.send(["studio-select"])
            let drawn = await app.until("colour-count \(Self.galleryMark) \(Self.gallery)") { Self.pixels($0) > 200 }
            app.expect(Self.pixels(drawn) > 200, "the selected rows and tiles of the sidebar's specimens fill with the new colour:\n\(drawn)")
            await app.send(["studio-undo"])
            await app.until("colour-count \(Self.galleryMark) \(Self.gallery)") { Self.pixels($0) == 0 }
        }
    }

    @Test("the right panel's specimens follow an edit too: the diff's added lines take the new background")
    @MainActor func galleryRightPanelFollows() async throws {
        try await Instance.with { app in
            await Self.studioOnNewTheme(app)
            await app.send(["studio-area panel", "studio-select diff.added.background"])
            app.expect(await app.reply("studio-set \(Self.galleryMark)"), contains: "ok=true")
            await app.send(["studio-select"])
            // The Diff specimen is the panel's last: the wheel brings it into view, and it is built as it comes.
            let found = await Self.scrollGallery(app, until: Self.galleryMark)
            app.expect(Self.pixels(found) > 100, "the Diff specimen's added lines are drawn on the new background:\n\(found)")
        }
    }

    @Test("selecting a token outlines, in the primary, what it is drawn on")
    @MainActor func galleryOutlinesWhatTheTokenDraws() async throws {
        try await Instance.with { app in
            await Self.studioOnNewTheme(app)
            // The outline is a dashed line in the primary round each element the token is drawn on (style/controls.rs:475).
            await app.send(["studio-area sidebar", "studio-select primary"])
            app.expect(await app.reply("studio-set \(Self.galleryMark)"), contains: "ok=true")
            await app.send(["studio-select"])
            let before = Self.pixels(await app.reply("colour-count \(Self.galleryMark) \(Self.gallery)"))
            await app.send(["studio-select sidebar.row.selected"])
            let after = await app.until("colour-count \(Self.galleryMark) \(Self.gallery)") { Self.pixels($0) > before + 50 }
            app.expect(Self.pixels(after) > before + 50, "the rows the token is drawn on are outlined (\(before) pixels of the primary before):\n\(after)")
            await app.send(["studio-select"])
            await app.until("colour-count \(Self.galleryMark) \(Self.gallery)") { Self.pixels($0) <= before }
        }
    }

    @Test("a click on an ANSI slot of the Terminal specimen selects that slot's token")
    @MainActor func galleryClickSelectsAnAnsiSlot() async throws {
        try await Instance.with { app in
            await Self.studioOnNewTheme(app)
            // A click on a palette slot selects its token (studio/gallery/terminal.rs:83). The slot is found by the colour it
            // was given, and the click lands where it is drawn.
            await app.send(["studio-area all", "studio-select terminal.ansi.5"])
            app.expect(await app.reply("studio-set \(Self.galleryMark)"), contains: "ok=true")
            await app.send(["studio-select"])
            app.expect(await app.reply("state"), contains: "token=-")
            let found = await Self.scrollGallery(app, until: Self.galleryMark)
            guard Self.pixels(found) > 0 else { return app.fail("the slot never came into view:\n\(found)") }
            app.expect(await app.reply("colour-count \(Self.galleryMark) \(Self.gallery) click"), contains: "box=")
            await app.until("state") { $0.contains("token=terminal.ansi.5 ") }
        }
    }
}
