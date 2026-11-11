// The context menus of the right panel's lists: a file of the git panel (`features/git/path_menu.rs`) and a
// commit of History (`features/git/history/menu.rs`). Choosing an item answers with a `ContextAction` of kind
// `panel.<name>`.

import Foundation
import WordsModel

public enum PanelMenus {
    public enum Kind: String, Sendable {
        case openFile, revealInFinder, copyPath, copyRelativePath, fileHistory
        case toggleBody, copyHash, copyShortHash, copyMessage, copySubject, copyAuthor, openParent
        public var key: String { "panel.\(rawValue)" }
    }

    /// A file in the Changes list or a commit's files. With several marked, one menu for all, where opening and
    /// revealing (one window each) are left out.
    public static func file(paths: [String], onDisk: Bool, opensInFinder: Bool, historyOn: Bool) -> [MenuNode] {
        var menu = ContextMenuList()
        let many = paths.count != 1
        if !many {
            // With Finder as the app, opening is revealing: one item, not two.
            if !opensInFinder { menu.item(Words.text("platform-menu-open", fallback: "Open"), Kind.openFile.key, argument: paths[0], enabled: onDisk) }
            menu.item(Words.text("history-menu-reveal", fallback: "Reveal in Finder"), Kind.revealInFinder.key, argument: paths[0], enabled: onDisk)
            menu.separator()
        }
        menu.item(Words.text("history-menu-copy-path", ["count": .whole(paths.count)], fallback: many ? "Copy Paths" : "Copy Path"), Kind.copyPath.key, argument: paths.joined(separator: "\n"))
        menu.item(
            Words.text("history-menu-copy-relative-path", ["count": .whole(paths.count)], fallback: many ? "Copy Relative Paths" : "Copy Relative Path"), Kind.copyRelativePath.key, argument: paths.joined(separator: "\n"))
        if historyOn {
            menu.separator()
            menu.item(
                Words.text("history-menu-file-history", ["count": .whole(paths.count)], fallback: many ? "History of These Files" : "File History"), Kind.fileHistory.key, argument: paths.joined(separator: "\n"))
        }
        return menu.finished
    }

    /// The commit `sha` in History, and with `toggle` (`true`: expanded) its opened details' own first item, and
    /// the parents the log lists as (hash, short hash).
    public static func commit(sha: String, toggle: Bool? = nil, parents: [(sha: String, short: String)] = []) -> [MenuNode] {
        var menu = ContextMenuList()
        if let expanded = toggle {
            menu.item(expanded ? Words.text("history-show-less", fallback: "Show less") : Words.text("history-show-more", fallback: "Show more"), Kind.toggleBody.key, argument: sha)
            menu.separator()
        }
        let copies: [(String, Kind)] = [
            (Words.text("history-menu-copy-hash", fallback: "Copy Hash"), .copyHash), (Words.text("history-menu-copy-short-hash", fallback: "Copy Short Hash"), .copyShortHash), (Words.text("history-menu-copy-message", fallback: "Copy Message"), .copyMessage),
            (Words.text("history-menu-copy-subject", fallback: "Copy Subject"), .copySubject), (Words.text("history-menu-copy-author", fallback: "Copy Author"), .copyAuthor),
        ]
        for (title, kind) in copies { menu.item(title, kind.key, argument: sha) }
        for (index, parent) in parents.enumerated() {
            if index == 0 { menu.separator() }
            menu.item(
                Words.text("history-menu-open-parent", ["short": .text(parent.short)], fallback: "Open Parent \(parent.short)"), Kind.openParent.key, argument: parent.sha)
        }
        return menu.finished
    }
}
