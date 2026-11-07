// What the right-click menu of a changed file offers (`features/git/path_menu.rs`, `history-menu-*`): open it in the
// editor, show it in Finder, the copies of its path and its own log in History. Used by the Changes rows and by the files
// of an opened commit.

import PanelModel
import SwiftUI

extension EnvironmentValues {
    /// The repository's folder, which a file's relative path is under.
    @Entry var panelRoot: String?
    /// Shows a path's log in History (File History).
    @Entry var showFileHistory: (String) -> Void = { _ in }
}

private struct ChangedFileMenu: ViewModifier {
    let file: ChangedFile
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root
    @Environment(\.showFileHistory) private var history

    func body(content: Content) -> some View {
        content.contextMenu {
            if let editor = host?.editorName {
                Button(panelText("files-row-open-in-editor", "Open in \(editor)")) { host?.open(absolute, line: nil) }
            }
            Button(panelText("history-menu-reveal", "Reveal in Finder")) { host?.reveal(absolute) }
            Divider()
            Button(panelText("history-menu-copy-path", "Copy Path")) { host?.copy(absolute) }
            Button(panelText("history-menu-copy-relative-path", "Copy Relative Path")) { host?.copy(file.path) }
            Divider()
            Button(panelText("history-menu-file-history", "File History")) { history(file.path) }
        }
    }

    private var absolute: String { root.map { $0 + "/" + file.path } ?? file.path }
}

extension View {
    func changedFileMenu(_ file: ChangedFile) -> some View { modifier(ChangedFileMenu(file: file)) }
}
