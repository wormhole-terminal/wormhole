// The words the row's tooltips use (`sidebar/row/face.rs`), through the words seam (`WordsModel`): each a Fluent
// message of `wormhole-text`'s `swift-window.ftl`.

import Foundation
import WordsModel

enum SidebarText {
    static var hiddenFromAgents: String {
        Words.text("sidebar-hidden-from-agents", fallback: "Hidden from agents") + "\n"
            + Words.text("sidebar-hidden-from-agents-detail", fallback: "Programs that control Wormhole can't see it")
    }

    /// "name · worktree of repo" over the terminal's folder; `main` is the repository's main working tree.
    static func worktree(_ name: String, of main: String, at folder: String?) -> String {
        let repo = URL(fileURLWithPath: main).lastPathComponent
        return Words.text("sidebar-worktree-of", ["name": .text(name), "repo": .text(repo)], fallback: "\(name) · worktree of \(repo)") + "\n" + (folder ?? "")
    }

    /// The name over "place · detail": what a cut-off row says in full.
    static func cut(_ name: String, place: String?, detail: String) -> String {
        name + "\n" + [place, detail.isEmpty ? nil : detail].compactMap { $0 }.joined(separator: " · ")
    }
}
