// text-file: the control folder's `diff-header`, `diff-step` and `diff-spans` (dev tooling), not interface text
// The open diff as text, from the store and session `DiffView` and `DiffHeader` draw from (Changes' while that tab is on
// show, else History's): the header's words and the rows' colour runs. Nothing here decides. The shape is
// `DesignSystem.Readout`'s:
//
//     diff-header changes "src/lib.rs" position="2 of 5" commit="abc1234" counts="+3 −1" split wrap gone
//     badge "Renamed from old.rs"
//     out-of-date "…"
//     submodule old="abc1234" new="def5678" ahead="3" behind="0" nothing="…" inside="…"
//
//     diff-spans unified rows="42" colored-by-core
//     hunk 0 "@@ -1,3 +1,4 @@ fn x"
//     line 1 added old="-" new="2" text="fn y() {}" spans="0-2:keyword,3-4:function"

import DesignSystem
import PanelModel

@MainActor
public enum DiffReadout {
    /// The store whose diff is on show: History's while its tab is, else Changes'.
    private static func open(_ panel: PanelController) -> (name: String, file: ChangedFile?, session: DiffSession, position: (index: Int, count: Int)?, commit: String?) {
        if panel.tab == .history {
            let history = panel.history
            return ("history", history.current, history.diff, history.position, history.shortOpen)
        }
        let changes = panel.changes
        return ("changes", changes.current, changes.diff, changes.position, nil)
    }

    /// `diff-header`: what the header row says.
    public static func header(_ panel: PanelController) -> String {
        let shown = open(panel)
        guard let file = shown.file else { return Readout.line("diff-header", [shown.name], quoted: ["none"]) }
        let diff = shown.session.parsed
        var pairs: [(String, String)] = []
        if let position = shown.position { pairs.append(("position", "\(position.index + 1) of \(position.count)")) }
        // The longest line in columns at the tab width now: the sideways scroll's extent.
        if let widest = diff?.widest { pairs.append(("widest", String(widest))) }
        if let commit = shown.commit { pairs.append(("commit", commit)) }
        // `DiffHeader`: the counts only for lines (not a binary file, an LFS pointer or a patch not loaded).
        if case let .counts(added, removed) = file.lines, diff?.isLFS != true, diff?.isBinary != true, diff?.tooLarge == nil {
            pairs.append(("counts", "+\(CountText.grouped(added)) \u{2212}\(CountText.grouped(removed))"))
        }
        var lines = [
            Readout.line(
                "diff-header", [shown.name], quoted: [file.path], pairs: pairs,
                flags: [Readout.flag("split", shown.session.split), Readout.flag("wrap", shown.session.wrap), Readout.flag("gone", shown.session.gone)])
        ]
        for badge in DiffHeader.badges(file: file, diff: diff, gone: shown.session.gone) { lines.append(Readout.line("badge", quoted: [badge.text])) }
        if let why = shown.session.refreshError { lines.append(Readout.line("out-of-date", quoted: [why])) }
        if let commits = diff?.submodule {
            var facts = [
                ("old", commits.old ?? "-"), ("new", commits.new ?? "-"), ("ahead", String(commits.aheadTotal)), ("behind", String(commits.behindTotal)),
                ("listed", String(commits.ahead.count + commits.behind.count)),
            ]
            if let nothing = commits.nothing { facts.append(("nothing", nothing)) }
            if let inside = commits.inside { facts.append(("inside", inside)) }
            lines.append(Readout.line("submodule", pairs: facts))
        }
        return lines.joined(separator: "\n")
    }

    /// `diff-step next|prev`: the header's arrows, as they step (round the list's ends). Nil when done.
    public static func step(_ panel: PanelController, _ direction: String) -> String? {
        guard let delta = ["next": 1, "prev": -1, "previous": -1][direction] else { return "diff-step next|prev" }
        if panel.tab == .history { panel.history.step(delta) } else { panel.changes.step(delta) }
        return nil
    }

    /// `diff-spans [from] [count]`: the unified rows as the session cut them (what the view draws), each code line with its
    /// colour runs as `start-end:token` in Characters of its text.
    public static func spans(_ panel: PanelController, from: Int, count: Int) -> String {
        let session = open(panel).session
        guard let built = session.built, let diff = session.parsed else { return Readout.line("diff-spans", quoted: ["none"], pairs: [("phase", "\(session.phase)")]) }
        var lines = [
            Readout.line(
                "diff-spans", ["unified"], pairs: [("rows", String(built.unified.count))],
                flags: [Readout.flag("colored-by-core", diff.coloredByCore), Readout.flag("no-highlight", diff.noHighlight)])
        ]
        for (index, row) in built.unified.enumerated().dropFirst(max(0, from)).prefix(max(0, count)) {
            switch row {
            case .hunk(let header): lines.append(Readout.line("hunk", [String(index)], quoted: [header]))
            case .unified(let cell): lines.append(line(index, cell))
            case .split(let old, let new):
                for cell in [old, new].compactMap({ $0 }) { lines.append(line(index, cell)) }
            }
        }
        return lines.joined(separator: "\n")
    }

    private static func line(_ index: Int, _ cell: DiffCell) -> String {
        let spans = cell.spans.map { "\($0.range.lowerBound)-\($0.range.upperBound):\($0.key)" }.joined(separator: ",")
        return Readout.line(
            "line", [String(index), "\(cell.kind)"],
            pairs: [("old", cell.old.map(String.init) ?? "-"), ("new", cell.new.map(String.init) ?? "-"), ("text", cell.text), ("spans", spans)])
    }
}
