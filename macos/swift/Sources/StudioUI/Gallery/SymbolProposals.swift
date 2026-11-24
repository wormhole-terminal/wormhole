// The icons of the window, the sidebar, the actions and the find bar that are not SF Symbols today, with the symbol each would
// become (`SymbolProposal`). The right panel's own are `SymbolProposalsPanel`. A brand's logo is not an icon and is not here.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem

enum SymbolProposals {
    static let groups: [SymbolGroup] = [window, tabs, sidebar, actions, find, SymbolProposalsPanel.group]

    private static let window = SymbolGroup(title: "Window and title rows", rows: [
        SymbolProposal(
            id: "toggle-sidebar", meaning: "Show or hide the sidebar (title row, left)", source: .window(.sidebar),
            symbol: "sidebar.left", alternate: "rectangle.leadinghalf.inset.filled", size: .title, motions: [.strip(.leading), .wipe(.leading)]),
        SymbolProposal(
            id: "toggle-panel", meaning: "Show or hide the right panel (title row, right)", source: .window(.panel),
            symbol: "sidebar.right", alternate: "rectangle.trailinghalf.inset.filled", size: .title, motions: [.strip(.trailing), .wipe(.trailing)]),
        SymbolProposal(
            id: "split-right", meaning: "Split to the right (title row)", source: .lucide(.columns2),
            symbol: "rectangle.split.2x1", alternate: "rectangle.split.2x1.fill", size: .title, motions: [.paneFill(.right), .pulse(.leading)]),
        SymbolProposal(
            id: "split-down", meaning: "Split below (title row)", source: .lucide(.rows2),
            symbol: "rectangle.split.1x2", alternate: "rectangle.split.1x2.fill", size: .title, motions: [.paneFill(.below), .pulse(.top)]),
        SymbolProposal(
            id: "open-in-menu", meaning: "Open in another app, its menu's chevron (title row)", source: .lucide(.chevronDown),
            symbol: "chevron.down", motions: [.turn(degrees: 180)]),
        SymbolProposal(id: "branch", meaning: "The branch (title row's crumb)", source: .lucide(.gitBranch), symbol: "arrow.triangle.branch"),
    ])

    private static let tabs = SymbolGroup(title: "Right panel tabs", rows: [
        SymbolProposal(
            id: "tab-changes", meaning: "Changes", source: .lucide(.gitCompare), symbol: "plus.forwardslash.minus",
            size: .title, motions: [.written(.top)]),
        SymbolProposal(
            id: "tab-history", meaning: "History", source: .lucide(.commit), symbol: "clock.arrow.circlepath",
            size: .title, motions: [.around]),
        SymbolProposal(
            id: "tab-notes", meaning: "Notes and tasks", source: .lucide(.fileText), symbol: "note.text",
            size: .title, motions: [.lines]),
        SymbolProposal(
            id: "tab-usage", meaning: "Usage, the plan's limits", source: .lucide(.gauge),
            symbol: "gauge.with.dots.needle.67percent", size: .title, motions: [.needle]),
        SymbolProposal(
            id: "tab-project", meaning: "Project", source: .lucide(.folder), symbol: "folder", alternate: "folder.fill",
            size: .title, motions: [.flash]),
        SymbolProposal(
            id: "tab-files", meaning: "Files, the tree", source: .lucide(.folderTree), symbol: "list.bullet.indent",
            size: .title),
    ])

    private static let sidebar = SymbolGroup(title: "Sidebar rows and tiles", rows: [
        SymbolProposal(
            id: "pin", meaning: "Pinned terminal (row, group head)", source: .lucide(.pin), symbol: "pin", alternate: "pin.fill",
            motions: [.wipe(.bottom), .swap]),
        SymbolProposal(
            id: "lock", meaning: "Hidden from agents (row)", source: .lucide(.lock), symbol: "lock", alternate: "lock.open",
            motions: [.swap]),
        SymbolProposal(
            id: "worktree", meaning: "A worktree's fork (row), the branch's symbol again", source: .lucide(.gitFork),
            symbol: "arrow.triangle.branch"),
        SymbolProposal(id: "split-link", meaning: "In a split with other folders (row)", source: .lucide(.link), symbol: "link"),
        SymbolProposal(id: "notes-mark", meaning: "Open notes and tasks (row)", source: .lucide(.note), symbol: "note.text"),
        SymbolProposal(id: "tile-terminal", meaning: "The tile's terminal mark", source: .glyph(.terminal), symbol: "terminal"),
        SymbolProposal(
            id: "badge-needs-you", meaning: "Needs you (tile badge)", source: .needsYou, symbol: "exclamationmark", weight: .black),
        SymbolProposal(
            id: "badge-done", meaning: "Done (tile badge)", source: .glyph(.check), symbol: "checkmark", weight: .semibold,
            motions: [.written(.leading)]),
        SymbolProposal(
            id: "badge-failed", meaning: "Failed (tile badge)", source: .glyph(.failed), symbol: "xmark.circle",
            alternate: "xmark.circle.fill", motions: [.grow]),
        SymbolProposal(
            id: "badge-bell", meaning: "Bell (tile badge)", source: .glyph(.bell), symbol: "bell", alternate: "bell.fill",
            motions: [.wipe(.bottom)]),
        SymbolProposal(
            id: "disclosure", meaning: "A group's chevron, also the panel's rows", source: .glyph(.chevron), symbol: "chevron.right",
            motions: [.turn(degrees: 90)]),
        SymbolProposal(
            id: "row-close", meaning: "Close a terminal (row), also the panel's and the tips'", source: .glyph(.close), symbol: "xmark",
            alternate: "xmark.circle.fill", motions: [.grow]),
        SymbolProposal(
            id: "row-add", meaning: "New terminal in a group (band), also the panel's", source: .glyph(.plus), symbol: "plus",
            alternate: "plus.circle.fill", motions: [.quarter, .disc]),
    ])

    private static let actions = SymbolGroup(title: "Actions", rows: [
        SymbolProposal(
            id: "refresh", meaning: "Refresh (panel, the exited bar)", source: .lucide(.refreshCw), symbol: "arrow.clockwise",
            motions: [.spin(clockwise: true)]),
        SymbolProposal(
            id: "copy", meaning: "Copy (a command block's output, a path)", source: .lucide(.copy), symbol: "doc.on.doc",
            alternate: "checkmark", motions: [.copied, .swap]),
        SymbolProposal(
            id: "check", meaning: "Checked (a group's folder item, a menu's choice)", source: .lucide(.check), symbol: "checkmark",
            motions: [.written(.leading)]),
    ])

    private static let find = SymbolGroup(title: "Find bar", rows: [
        SymbolProposal(id: "find-search", meaning: "The search field's magnifier", source: .lucide(.search), symbol: "magnifyingglass"),
        SymbolProposal(
            id: "find-case", meaning: "Match case", source: .lucide(.caseSensitive), symbol: "textformat"),
        SymbolProposal(
            id: "find-word", meaning: "Whole word", source: .lucide(.wholeWord), symbol: "character.textbox"),
        SymbolProposal(
            id: "find-regex", meaning: "Regular expression", source: .lucide(.regex), symbol: nil,
            reason: "asterisk reads as a footnote, curlybraces as code"),
    ])
}
