// The one ⋯ button Open in… and the splits fold into when the row is narrow: the apps to open the folder in,
// then Split right and Split down, each as in its own button.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

struct MoreMenu: View {
    let plan: TitlePlan
    let actions: TitleActions
    let backend: any OpenInBackend

    var body: some View {
        let facts = plan.facts
        let choice = backend.choice()
        Menu {
            if plan.draws(.openIn), let folder = facts.folder {
                Section(PanesText.openFolderIn) {
                    ForEach(backend.apps(), id: \.bundle) { app in
                        Toggle(app.name, isOn: Binding(get: { app.bundle == choice.app.bundle }, set: { _ in backend.open(folder, in: app) }))
                    }
                }
            }
            if plan.draws(.splits) {
                Divider()
                Button(PanesText.splitRight) { actions.split(.row) }.disabled(!actions.canSplit(.row))
                Button(PanesText.splitDown) { actions.split(.column) }.disabled(!actions.canSplit(.column))
            }
        } label: {
            Image(systemName: "ellipsis").font(.system(size: Metric.titleGlyph))
                .frame(width: Metric.titleControl, height: Metric.titleControl)
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .frame(width: Metric.hitMin, height: Metric.hitMin)
        .help(more(plan)).accessibilityLabel(more(plan))
    }

    private func more(_ plan: TitlePlan) -> String {
        switch (plan.draws(.openIn), plan.draws(.splits)) {
        case (true, true): PanesText.moreOpenInSplit
        case (true, false): PanesText.moreOpenIn
        default: PanesText.moreSplit
        }
    }
}
