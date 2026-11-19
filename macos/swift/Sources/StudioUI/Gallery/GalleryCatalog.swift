// The gallery's page, in order: every component of the Swift design system on sample data, the real views of
// `SidebarUI`, `PanelUI` and `PaletteUI` and the primitives of `DesignSystem`, then the colour basics. A component
// added to those modules joins here (and in `SpecimenContent`), or the gallery stops showing every component, which
// the GPUI app's Studio holds itself to.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

enum GalleryCatalog {
    static let specimens: [Specimen] = [
        Specimen("sidebar", "Sidebar", "bands, rows in every state, glyphs", area: .sidebar, column: .left),
        Specimen("statusbar", "Status bar", "the terminal that needs you, the counts, the state and the size, wide and narrow", area: .sidebar, column: .both),
        Specimen("icontabs", "Icon tabs", "the right panel's tabs, the active one with its count", area: .panel, column: .right),
        Specimen("notes", "Notes and tasks", "a note, the steps, the checkboxes", area: .panel, column: .right),
        Specimen("changes", "Changes", "section bands, files, counts", area: .panel, column: .right),
        Specimen("history", "History", "commits, refs, the open one", area: .panel, column: .right),
        Specimen("project", "Project", "the trust callout, two-line rows, the folder's colour", area: .panel, column: .right),
        Specimen("files", "Files", "the tree's folder and file rows, the path button", area: .panel, column: .right),
        Specimen("diff", "Diff", "header, hunk, context, removed, added, filler", area: .panel, column: .right),
        Specimen("chrome", "Chrome", "icon buttons, badges, status glyphs, rows, a hit area", area: .chrome, column: .left),
        Specimen("tiles", "Tiles and tabs", "status tiles at three sizes and the list's in every state, a status change, a count turning, the tab bar", area: .sidebar, column: .left),
        Specimen("drops", "Drag and drop", "a drop that makes a split, a drop between two cards, the chain mark", area: .sidebar, column: .left),
        Specimen("find", "Find", "the find bar, matches and the current one", area: .chrome, column: .left),
        Specimen("panes", "Panes", "pane headers, focused and not, the dividers, a pane as a terminal is dragged over it", area: .chrome, column: .left),
        Specimen("chips", "Tabs and chips", "the title row's chips and path, the context meter, badges, key caps", area: .chrome, column: .left),
        Specimen("labels", "Labels", "titles, section labels, empty notes, toolbars", area: .chrome, column: .left),
        Specimen("shape", "Shape", "radii, the focus ring, everything chip-shaped", area: .chrome, column: .left),
        Specimen("settings", "Settings", "the nav, setting rows in their states, recorders, the arranged list", area: .settings, column: .both),
        Specimen("controls", "Profile and controls", "slider, stepper, segments, profile chip, banners", area: .settings, column: .right),
        Specimen("buttons", "Buttons", "variants and sizes", area: .settings, column: .right),
        Specimen("inputs", "Inputs", "fields, checkboxes, switches, radios", area: .settings, column: .right),
        Specimen("menus", "Menus", "the select, the pop-up button, an opened list with its search row", area: .settings, column: .right),
        Specimen("windows", "Windows", "session hosts, the first run's card, Licenses, About", area: .ambient, column: .both),
        Specimen("system", "System controls", "tooltip, menu, question and scroll bar, drawn by macOS", area: .settings, column: .right),
        Specimen("sketch", "Window sketch", "the first run's 'How much to show' presets, Zen to Everything", area: .ambient, column: .both),
        Specimen("palette", "Command palette", "the palette and the shortcut sheet on the scrim, the menu bar's quick panel", area: .ambient, column: .both),
        Specimen("readouts", "Readouts", "a resize handle and its width, the grid's size, a command block", area: .ambient, column: .both),
        Specimen("fonts", "Font picker", "search, the families in their own faces, the preview and what the font draws", area: .settings, column: .right),
        Specimen("terminal", "Terminal", "16 ANSI, text, selection, cursor", area: nil, column: .left),
        Specimen("code", "Code", "every syntax colour on the terminal's background", area: nil, column: .right),
        Specimen("metrics", "Sizes and motion", "every size, inset and duration", area: nil, column: .left),
        Specimen("type", "Type", "the roles of the interface font", area: nil, column: .left),
    ]
}
