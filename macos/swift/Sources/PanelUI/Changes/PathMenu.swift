// The right-click menu of a path in the git panel (`features/git/path_menu.rs`): open it in the chosen editor (not with Finder
// chosen, whose open is the reveal), show it in Finder, copy its path, show its history. A deleted file has nothing to open or
// reveal. Several marked files share one menu, where opening and revealing (a window each) are left out. A Changes row, a
// folder row and a commit's file use it.

import PanelModel
import SwiftUI

extension EnvironmentValues {
    /// The repository's folder, which a file's relative path is under.
    @Entry var panelRoot: String?
    /// Shows the log of these paths in History (File History).
    @Entry var showFileHistory: ([String]) -> Void = { _ in }
}

private struct PathMenu: ViewModifier {
    let paths: [String]
    /// Whether the file is there now; nil asks the disk (a folder, a commit's file).
    let onDisk: Bool?
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root
    @Environment(\.showFileHistory) private var history

    func body(content: Content) -> some View {
        content.contextMenu {
            let count = paths.count
            if count == 1, let path = paths.first {
                let there = onDisk ?? host?.exists(absolute(path)) ?? true
                if let open = host?.openTooltip(line: nil), host?.editorName != nil {
                    Button(open) { host?.open(resolved(path), line: nil) }.disabled(!there)
                }
                Button(panelText("history-menu-reveal", "Reveal in Finder")) { host?.reveal(resolved(path)) }.disabled(!there)
                Divider()
            }
            Button(panelText("history-menu-copy-path", ["count": .whole(count)], count == 1 ? "Copy Path" : "Copy Paths")) { copy(paths.map(absolute)) }
            Button(panelText("history-menu-copy-relative-path", ["count": .whole(count)], count == 1 ? "Copy Relative Path" : "Copy Relative Paths")) { copy(paths) }
            if host?.offers(tab: .history) ?? true {
                Divider()
                Button(panelText("history-menu-file-history", ["count": .whole(count)], count == 1 ? "File History" : "History of These Files")) { history(paths) }
            }
        }
    }

    private func absolute(_ path: String) -> String { root.map { $0 + "/" + path } ?? path }

    /// What is opened or revealed is the path as core resolved it inside the repository, never a link out of it.
    private func resolved(_ path: String) -> String { root.flatMap { RepoFiles.resolve(root: $0, relative: path) } ?? absolute(path) }

    private func copy(_ lines: [String]) {
        if lines.count == 1 { host?.copyPath(lines[0]) } else {
            host?.copy(lines.joined(separator: "\n"))
            host?.notify(panelText("files-spoken-path-copied", "Path copied"))
        }
    }
}

extension View {
    func pathMenu(_ paths: [String], onDisk: Bool? = nil) -> some View { modifier(PathMenu(paths: paths, onDisk: onDisk)) }
}
