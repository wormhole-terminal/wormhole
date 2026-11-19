// text-file: the control folder's `usage` (dev tooling), not interface text
// The Usage tab as its store holds it (`UsageStore.reports`, `now`): each agent's report with its Updated line and each
// limit's meter words, in the words the tab draws them with (`PlanLimit.shown`, `UsageText.updated`). The shape is
// `DesignSystem.Readout`'s:
//
//     usage now="1763100000" reports="1"
//     report claude "Claude Code" updated="Updated just now"
//       limit "5-hour limit" share="0.42" words="42% · resets 3h" level="normal"

import DesignSystem
import Foundation
import PanelModel

@MainActor
public enum UsageReadout {
    public static func text(_ usage: UsageStore) -> String {
        let now = usage.now
        var lines = [Readout.line("usage", pairs: [("now", String(now)), ("reports", String(usage.reports.count))])]
        for report in usage.reports {
            lines.append(Readout.line("report", [report.agent], quoted: [report.name], pairs: [("updated", UsageText.updated(report.updated, now: now))]))
            for limit in report.limits {
                let shown = limit.shown(now: now)
                lines.append(
                    Readout.under(
                        "limit", quoted: [limit.label],
                        pairs: [("share", String(format: "%.2f", shown.share)), ("words", shown.words), ("level", "\(PlanLimit.level(of: shown.share))")]))
            }
        }
        return lines.joined(separator: "\n")
    }
}
