// text-file: the control folder's `changes-rows` (dev tooling), not interface text
// The Changes list as `ChangesListView` draws it: the rows `ChangeRows.build` makes of the store's files with the layout in
// force (tree or list, untracked apart or not, the folds), cut into a card per section as the view cuts them. Nothing here
// decides. The shape is `DesignSystem.Readout`'s:
//
//     changes-rows tree grouped
//     band changes count="3"
//       dir "src/a" label="src/a" depth="0"
//       file "src/a/one.rs" depth="1" kind="M"
//     band untracked count="201" folded
//       note "201 of them are in gen/, which isn't ignored. Folded because there are more than 200."
//       file "vendor/lib" depth="0" kind="M" submodule="1 new commit" enter

import DesignSystem
import PanelModel

@MainActor
public enum ChangesRowsReadout {
    public static func text(_ changes: ChangesStore) -> String {
        let layout = changes.layout
        var lines = [Readout.line("changes-rows", [layout.tree ? "tree" : "list", layout.groupUntracked ? "grouped" : "together"])]
        for section in ChangeSection.cut(layout.build(changes.files)) {
            if case let .band(group, count, folded, _) = section.band {
                lines.append(Readout.line("band", [group.rawValue], pairs: [("count", String(count))], flags: [Readout.flag("folded", folded)]))
            }
            for row in section.rows {
                switch row {
                case let .foldNote(text): lines.append(Readout.under("note", quoted: [text]))
                case let .directory(_, path, label, depth, collapsed):
                    lines.append(Readout.under("dir", quoted: [path], pairs: [("label", label), ("depth", String(depth))], flags: [Readout.flag("collapsed", collapsed)]))
                case let .file(file, depth, _):
                    var pairs = [("depth", String(depth)), ("kind", file.kind.letter)]
                    if let words = file.submodule { pairs.append(("submodule", words)) }
                    lines.append(Readout.under("file", quoted: [file.path], pairs: pairs, flags: [Readout.flag("enter", file.submodule != nil && file.submoduleEnterable)]))
                case .band, .spacer: break
                }
            }
        }
        return lines.joined(separator: "\n")
    }
}
