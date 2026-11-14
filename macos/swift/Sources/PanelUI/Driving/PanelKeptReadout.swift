// text-file: the control folder's `panel-kept` (dev tooling), not interface text
// What the right panel's stores keep per terminal and per folder, so a check sees it dropped when a terminal goes: the
// terminals Files, Changes and History keep a state for (not the selected one, which is live), the folders' repository roots,
// and whether Files reads and watches its folder now.
//
//     panel-kept files="3" changes="3" history="" roots="/…/repo" watching shown

import DesignSystem

@MainActor
public enum PanelKeptReadout {
    public static func text(_ panel: PanelController) -> String {
        func ids(_ list: [Int]) -> String { list.map(String.init).joined(separator: ",") }
        return Readout.line(
            "panel-kept",
            pairs: [
                ("files", ids(panel.files.keptTerminals)), ("changes", ids(panel.changes.keptTerminals)), ("history", ids(panel.history.keptTerminals)),
                ("roots", panel.roots.keys.sorted().joined(separator: ",")),
            ],
            flags: [Readout.flag("watching", panel.files.isWatching), Readout.flag("shown", panel.files.isShown)])
    }
}
