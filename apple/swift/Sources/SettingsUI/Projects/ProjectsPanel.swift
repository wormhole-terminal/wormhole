// The Project folders page (`sections/projects.rs`): a picker, then either the defaults every project starts from and the
// list of trusted projects, or one project's trust and its own Default | On | Off for each feature. The page is its own
// cards, so it sits on the page bare (`ControlLayout.bare`) and not inside one.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ProjectsPanel: View {
    let model: SettingsModel

    var body: some View {
        if let projects = model.projectsModel { ProjectsBody(projects: projects) } else { PanelNote(text: SettingsText.notBuilt) }
    }
}

private struct ProjectsBody: View {
    let projects: ProjectsModel
    /// The project select's width (the GPUI page's 360).
    private static let selectWidth: CGFloat = 360

    var body: some View {
        VStack(alignment: .leading, spacing: Space._6) {
            SettingGroup(title: ProjectsText.project) {
                HStack {
                    FlatSelect(options: options, selected: .string(projects.selected ?? ""), label: ProjectsText.project, width: Self.selectWidth) {
                        pick($0.string)
                    }
                    Spacer(minLength: 0)
                    if projects.selected != nil { LinkButton(title: ProjectsText.allProjects) { pick(nil) } }
                }
                .padding(.vertical, Space._2)
            }
            if let page = projects.page {
                ProjectPageView(projects: projects, page: page)
            } else if let missing = projects.selected {
                Banner(kind: .warning, text: ProjectsText.missing(missing))
            } else {
                Defaults(projects: projects)
            }
        }
        .task { await projects.refresh() }
    }

    private var options: [SettingOption] {
        [SettingOption(value: .string(""), label: ProjectsText.allProjectsDefaults)]
            + projects.known.map { SettingOption(value: .string($0.root), label: $0.name) }
    }

    private func pick(_ root: String?) { Task { await projects.select(root.flatMap { $0.isEmpty ? nil : $0 }) } }
}

/// "All projects": the switch of each feature, and the projects trusted so far.
private struct Defaults: View {
    let projects: ProjectsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        SettingGroup(title: ProjectsText.byDefault, blurb: ProjectsText.defaultsNote) {
            ForEach(projects.defaults) { feature in
                SettingRow(label: feature.label, description: feature.description, visibleHint: true, state: RowState(), onReset: {}) {
                    FlatSwitch(isOn: feature.on, label: feature.label) { on in Task { await projects.setDefault(feature.key, on: on) } }
                }
            }
        }
        SettingGroup(title: ProjectsText.trustedProjects, aside: ProjectsText.trustedAside) {
            if projects.trusted.isEmpty {
                Text(ProjectsText.none).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textFaint).padding(.vertical, Space._3)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            ForEach(projects.trusted) { project in
                HStack(spacing: Space._2) {
                    Text(project.detail).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textPrimary)
                        .lineLimit(1).truncationMode(.middle).frame(maxWidth: .infinity, alignment: .leading)
                    LinkButton(title: ProjectsText.settings) { Task { await projects.select(project.root) } }
                    LinkButton(title: ProjectsText.revoke) { Task { await projects.revoke(project.root) } }
                }
                .frame(minHeight: SettingsMetric.rowMinHeight + Space._2)
            }
        }
    }
}
