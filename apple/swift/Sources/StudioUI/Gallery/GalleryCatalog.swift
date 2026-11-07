// The gallery's page, in order: every component of the Swift design system on sample data, the real views of
// `SidebarUI`, `PanelUI` and `PaletteUI` and the primitives of `DesignSystem`, then the colour basics. A component
// added to those modules joins here (and in `SpecimenContent`), or the gallery stops showing every component, which
// the GPUI app's Studio holds itself to.

enum GalleryCatalog {
    static let specimens: [Specimen] = [
        Specimen("sidebar", "Sidebar", "bands, rows in every state, glyphs", area: .sidebar, column: .left, tokens: GalleryUses.sidebar),
        Specimen("notes", "Notes and tasks", "a note, the steps, the checkboxes", area: .panel, column: .right, tokens: GalleryUses.notes),
        Specimen("changes", "Changes", "section bands, files, counts", area: .panel, column: .right, tokens: GalleryUses.changes),
        Specimen("history", "History", "commits, refs, the open one", area: .panel, column: .right, tokens: GalleryUses.history),
        Specimen("diff", "Diff", "header, hunk, context, removed, added, filler", area: .panel, column: .right, tokens: GalleryUses.diff),
        Specimen("chrome", "Chrome", "icon buttons, badges, status glyphs, rows", area: .chrome, column: .left, tokens: GalleryUses.chrome),
        Specimen("shape", "Shape", "radii, the focus ring, everything chip-shaped", area: .chrome, column: .left, tokens: SpecimenTokens.shape),
        Specimen("controls", "Controls and banners", "buttons, fields, switches, sliders, banners", area: .settings, column: .right, tokens: SpecimenTokens.controls),
        Specimen("palette", "Command palette", "the quick panel and the shortcut sheet, on the scrim", area: .ambient, column: .both, tokens: GalleryUses.palette),
        Specimen("terminal", "Terminal", "16 ANSI, text, selection, cursor", area: nil, column: .left, tokens: ["terminal.*"]),
        Specimen("code", "Code", "every syntax colour on the terminal's background", area: nil, column: .right, tokens: ["syntax.*"]),
        Specimen("type", "Type", "the roles of the interface font", area: nil, column: .left, tokens: ["text.primary", "text.muted", "text.faint"]),
    ]
}
