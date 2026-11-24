// The right panel's icons, which `PanelIcon` draws in Lucide where the GPUI app does, and the symbols that would take their
// place (`SymbolProposals`). The views ask for a symbol by name and `PanelIcon` maps it to Lucide, so a change here is that
// table's to make.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem

enum SymbolProposalsPanel {
    static let group = SymbolGroup(title: "Right panel", rows: [
        SymbolProposal(id: "chevron-left", meaning: "Previous file (diff header)", source: .lucide(.chevronLeft), symbol: "chevron.left"),
        SymbolProposal(id: "chevron-up", meaning: "Previous match, fold (find, commit)", source: .lucide(.chevronUp), symbol: "chevron.up"),
        SymbolProposal(
            id: "chevrons-updown", meaning: "A picker's chevrons (revision picker)", source: .lucide(.chevronsUpDown),
            symbol: "chevron.up.chevron.down"),
        SymbolProposal(
            id: "view-options", meaning: "View options (changes)", source: .lucide(.slidersHorizontal), symbol: "slider.horizontal.3"),
        SymbolProposal(id: "wrap", meaning: "Wrap lines (diff)", source: .lucide(.textWrap), symbol: "text.append"),
        SymbolProposal(
            id: "open-external", meaning: "Open in the app (files, diff)", source: .lucide(.externalLink), symbol: "arrow.up.forward.square",
            alternate: "arrow.up.forward.square.fill", motions: [.pulse(.bottom)]),
        SymbolProposal(
            id: "stash", meaning: "Stash the prompt into a new note (notes)", source: .lucide(.inbox), symbol: "tray.and.arrow.down",
            alternate: "tray.and.arrow.down.fill", motions: [.pulse(.bottom)]),
        SymbolProposal(id: "file", meaning: "A file (viewer)", source: .lucide(.file), symbol: "doc"),
        SymbolProposal(id: "file-text", meaning: "A text file (project's prompts)", source: .lucide(.fileText), symbol: "doc.text"),
        SymbolProposal(
            id: "file-add", meaning: "A new file (project's notes)", source: .lucide(.filePlus), symbol: "doc.badge.plus"),
        SymbolProposal(
            id: "info", meaning: "Information (callout)", source: .lucide(.info), symbol: "info.circle", alternate: "info.circle.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(
            id: "shield", meaning: "Warning (callout)", source: .lucide(.shield), symbol: "shield", alternate: "shield.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(id: "shield-check", meaning: "Trusted (project notice)", source: .lucide(.shieldCheck), symbol: "checkmark.shield"),
        SymbolProposal(id: "alert", meaning: "Danger (callout)", source: .lucide(.triangleAlert), symbol: "exclamationmark.triangle"),
        SymbolProposal(
            id: "arrow-up-right", meaning: "Open the link (project, rows)", source: .lucide(.arrowUpRight), symbol: "arrow.up.right"),
        SymbolProposal(id: "arrow-right", meaning: "Renamed to (changes, submodules)", source: .lucide(.arrowRight), symbol: "arrow.right"),
        SymbolProposal(
            id: "run", meaning: "Run (project's actions)", source: .lucide(.play), symbol: "play", alternate: "play.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(id: "more", meaning: "More (commit details, pane, title row)", source: .lucide(.ellipsis), symbol: "ellipsis"),
        SymbolProposal(id: "comment-add", meaning: "A note on a diff line", source: .lucide(.messageSquarePlus), symbol: "plus.bubble"),
        SymbolProposal(id: "ban", meaning: "No colour (folder colour picker)", source: .lucide(.ban), symbol: "nosign"),
        SymbolProposal(
            id: "submodule", meaning: "A submodule's folder (changes)", source: .lucide(.folderGit2), symbol: "folder.badge.gearshape",
            alternate: "folder.fill.badge.gearshape", motions: [.pulse(.bottom)]),
        SymbolProposal(
            id: "folder-add", meaning: "A new folder (changes)", source: .lucide(.folderInput), symbol: "folder.badge.plus",
            alternate: "folder.fill.badge.plus", motions: [.pulse(.bottom)]),
        SymbolProposal(id: "shell", meaning: "A terminal (layout, project)", source: .lucide(.squareTerminal), symbol: "terminal"),
        SymbolProposal(
            id: "tag", meaning: "A tag (revision picker)", source: .lucide(.tag), symbol: "tag", alternate: "tag.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(
            id: "settings", meaning: "Settings (project)", source: .lucide(.settings), symbol: "gearshape", alternate: "gearshape.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(
            id: "palette", meaning: "Theme (project)", source: .lucide(.palette), symbol: "paintpalette", alternate: "paintpalette.fill",
            motions: [.pulse(.bottom)]),
        SymbolProposal(id: "archive", meaning: "Stashes (revision picker)", source: .lucide(.archive), symbol: "tray.full"),
    ])
}
