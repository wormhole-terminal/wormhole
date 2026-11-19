// The title row's two menus, item by item: the row's own (`window/title/parts.rs`: every hideable part with a check while it
// is shown, Reset title row, Customize) and the folder's (`window/title/path_menu.rs`: the path as shown heads it, Open in
// Finder, Copy path, Insert into terminal, then the path's folders, innermost first, down to the volume, each opening in
// Finder). One list, drawn by the right-click's SwiftUI menu (`TitleMenuItems`) and by the keys' native one (`TitleKeyMenu`,
// which the control folder's `title-menu` and `path-menu` read), so what a script reads is what a right-click shows.

import AppKit
import DesignSystem
import PanesModel

/// One line of a title row menu.
enum TitleMenuEntry {
    case heading(String)
    case divider
    /// `checked`: nil for an item that never has a check mark; `icon`: the path whose Finder icon the item wears.
    case item(String, enabled: Bool = true, checked: Bool? = nil, icon: String? = nil, run: () -> Void)
}

@MainActor
enum TitleMenuEntries {
    /// The row's own menu; the checks are the person's own row (`ownRow`), not a preset Zen mode shows.
    static func row(_ shown: TitleRow, _ actions: TitleActions) -> [TitleMenuEntry] {
        let row = actions.ownRow() ?? shown
        var entries: [TitleMenuEntry] = [.heading(PanesText.showInTitleRow)]
        for part in TitlePart.allCases where part.hideable {
            let on = row.shows(part)
            entries.append(.item(PanesText.partName(part), enabled: actions.canEditRow, checked: on) { actions.setPart(part, !on) })
        }
        entries.append(.divider)
        entries.append(.item(PanesText.resetTitleRow, enabled: actions.canEditRow && !row.isDefault, run: actions.resetRow))
        entries.append(.item(PanesText.customizeTitleRow, enabled: actions.canEditRow, run: actions.customizeRow))
        return entries
    }

    /// The folder's menu: `shown` is the folder as the row writes it, `folder` its absolute path (nil: no folders to list).
    static func path(_ folder: String?, shown: String, _ actions: TitleActions) -> [TitleMenuEntry] {
        var entries: [TitleMenuEntry] = [
            .heading(shown), .divider,
            .item(PanesText.openInFinder, run: actions.revealFolder),
            .item(PanesText.copyPath, run: actions.copyPath),
            .item(PanesText.insertIntoTerminal, enabled: actions.canInsert, run: actions.insertPath),
        ]
        guard let folder else { return entries }
        entries.append(.divider)
        for step in PathChain.of(folder, bootVolume: bootVolume) {
            // Through `ExternalOpen`, so a test instance records the launch instead of opening the person's Finder.
            entries.append(.item(step.name, icon: step.path) { ExternalOpen.open(URL(fileURLWithPath: step.path)) })
        }
        return entries
    }

    /// The icon Finder gives a folder, at menu size (the house for the home folder, a disk for the volume).
    static func icon(of path: String) -> NSImage {
        let icon = NSWorkspace.shared.icon(forFile: path)
        icon.size = NSSize(width: 16, height: 16)
        return icon
    }

    /// The name of the volume the system starts from ("Macintosh HD").
    private static var bootVolume: String {
        (try? URL(fileURLWithPath: "/").resourceValues(forKeys: [.volumeNameKey]).volumeName) ?? "Macintosh HD" // text: macOS's default name of the startup volume
    }
}
