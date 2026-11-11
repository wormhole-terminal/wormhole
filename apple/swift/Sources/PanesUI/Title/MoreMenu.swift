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
        Menu {
            if plan.draws(.openIn), let folder = facts.folder {
                let open = OpenInMenu(folder: folder, backend: backend)
                Section(PanesText.openFolderIn) {
                    ForEach(open.items, id: \.app.bundle) { item in
                        Toggle(item.app.name, isOn: Binding(get: { item.checked }, set: { _ in open.pick(item.app) }))
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
