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
        Specimen("tiles", "Tiles and tabs", "status tiles at three sizes, the tab bar", area: .sidebar, column: .left, tokens: GalleryUses.tiles),
        Specimen("find", "Find", "the find bar, matches and the current one", area: .chrome, column: .left, tokens: GalleryUses.find + ["terminal.*"]),
        Specimen("panes", "Panes", "pane headers, focused and not, and the dividers", area: .chrome, column: .left, tokens: GalleryUses.panes),
        Specimen("chips", "Tabs and chips", "the title row's chips, badges, key caps", area: .chrome, column: .left, tokens: GalleryUses.chips),
        Specimen("labels", "Labels", "titles, section labels, empty notes, toolbars", area: .chrome, column: .left, tokens: GalleryUses.labels),
        Specimen("shape", "Shape", "radii, the focus ring, everything chip-shaped", area: .chrome, column: .left, tokens: SpecimenTokens.shape),
        Specimen("controls", "Profile and controls", "slider, stepper, segments, profile chip, banners", area: .settings, column: .right, tokens: SpecimenTokens.controls),
        Specimen("buttons", "Buttons", "variants and sizes", area: .settings, column: .right, tokens: GalleryUses.buttons + ["primary", "primary.hover", "text.on_primary", "text.link"]),
        Specimen("inputs", "Inputs", "fields, checkboxes, switches, radios", area: .settings, column: .right, tokens: SpecimenTokens.inputs),
        Specimen("menus", "Menus", "the select and the pop-up button", area: .settings, column: .right, tokens: SpecimenTokens.menus),
        Specimen("windows", "Windows", "the secondary windows' cards, badges and buttons", area: .ambient, column: .both, tokens: GalleryUses.windows),
        Specimen("sketch", "Window sketch", "the first run's 'How much to show' presets, Zen to Everything", area: .ambient, column: .both, tokens: GalleryUses.sketch),
        Specimen("palette", "Command palette", "the quick panel and the shortcut sheet, on the scrim", area: .ambient, column: .both, tokens: GalleryUses.palette),
        Specimen("fonts", "Font picker", "search, the families in their own faces, the preview and what the font draws", area: .settings, column: .right, tokens: GalleryUses.fonts + ["terminal.*"]),
        Specimen("terminal", "Terminal", "16 ANSI, text, selection, cursor", area: nil, column: .left, tokens: ["terminal.*"]),
        Specimen("code", "Code", "every syntax colour on the terminal's background", area: nil, column: .right, tokens: ["syntax.*"]),
        Specimen("metrics", "Sizes and motion", "every size, inset and duration", area: nil, column: .left, tokens: []),
        Specimen("type", "Type", "the roles of the interface font", area: nil, column: .left, tokens: ["text.primary", "text.muted", "text.faint"]),
    ]
}
