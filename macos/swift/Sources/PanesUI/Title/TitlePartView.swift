// What one part of the row draws, once the plan says there is room for it.

import AppKit
import PanesModel
import SwiftUI

struct TitlePartView: View {
    let part: TitlePart
    let plan: TitlePlan
    let actions: TitleActions
    let backend: any OpenInBackend
    let icon: (OpenInApp) -> NSImage?

    var body: some View {
        let facts = plan.facts
        switch part {
        case .folder: FolderCrumb(plan: plan, actions: actions)
        case .branch: BranchCrumb(plan: plan, actions: actions)
        case .branchStatus: facts.branchStatus.map { BranchStatusMark(status: $0, action: actions.showChanges) }
        case .terminal: TerminalCrumb(plan: plan, tile: actions.tile?(facts.terminalID, TitleMetrics.tile))
        case .terminalStatus: facts.words.map { StatusWords(words: $0, activity: facts.activity) }
        case .agentGauge: facts.gauge.map(GaugeChip.init)
        case .ports: PortChips(ports: facts.ports, step: plan.fit.portsStep)
        case .pullRequest: facts.pullRequest.map(PullRequestChip.init)
        case .openIn, .splits: buttons
        case .space: EmptyView()
        }
    }

    @ViewBuilder private var buttons: some View {
        if plan.fit.folded {
            MoreMenu(plan: plan, actions: actions, backend: backend)
        } else if part == .splits {
            SplitButtons(actions: actions)
        } else if let folder = plan.facts.folder {
            OpenInButtons(folder: folder, backend: backend, icon: icon)
        }
    }
}
