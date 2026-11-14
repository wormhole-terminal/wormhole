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
    /// What the person typed to narrow the projects the select offers (`Search projects`).
    @State private var search = ""
    /// The project select's width (the GPUI page's 360).
    private static let selectWidth: CGFloat = 360
    private static let searchWidth: CGFloat = 160
    @Environment(\.appearsActive) private var appearsActive

    var body: some View {
        VStack(alignment: .leading, spacing: Space._6) {
            SettingGroup(title: ProjectsText.project) {
                HStack {
                    TextField(ProjectsText.search, text: $search).textFieldStyle(.roundedBorder).frame(width: Self.searchWidth).recordFrame("project.search")
                        // Return takes the first project the search leaves (`projects.rs`).
                        .onSubmit { if let root = options.dropFirst().first?.value.string { pick(root) } }
                    OptionPicker(options: options, selected: .string(projects.selected ?? ""), label: ProjectsText.project, width: Self.selectWidth) {
                        pick($0.string)
                    }
                    Spacer(minLength: 0)
                    if projects.selected != nil { LinkButton(title: ProjectsText.allProjects) { pick(nil) }.recordFrame("project.all") }
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
        .task {
            await projects.refresh()
            await projects.followTrust()
        }
        // The project's own files reach the page as an event (the core's watch on its `.wormhole` folder, started by `select`), and
        // so does a trust given or taken back elsewhere (the watch on the profile's approvals, while the page is shown). A watch
        // can drop events: the page also reads whenever the window comes forward. An idle window sleeps until something happens.
        .onChange(of: appearsActive) { _, active in if active { Task { await projects.refresh() } } }
    }

    private var options: [SettingOption] {
        [SettingOption(value: .string(""), label: ProjectsText.allProjectsDefaults)]
            + projects.known.filter { search.isEmpty || $0.name.localizedCaseInsensitiveContains(search) || $0.root.localizedCaseInsensitiveContains(search) }
            .map { SettingOption(value: .string($0.root), label: $0.name) }
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
                SettingRow(label: feature.label, description: feature.description, visibleHint: true, state: RowState(), frameId: "project.default.\(feature.key)", onReset: {}) {
                    SettingToggle(label: feature.label, isOn: feature.on) { on in Task { await projects.setDefault(feature.key, on: on) } }
                }
            }
        }
        SettingGroup(title: ProjectsText.trustedProjects, aside: ProjectsText.trustedAside) {
            if projects.trusted.isEmpty {
                Text(ProjectsText.none).textRole(.body).foregroundStyle(tokens.textFaint).padding(.vertical, Space._3)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            ForEach(projects.trusted) { project in
                HStack(spacing: Space._2) {
                    Text(project.detail).textRole(.body).foregroundStyle(tokens.textPrimary)
                        .lineLimit(1).truncationMode(.middle).frame(maxWidth: .infinity, alignment: .leading)
                    LinkButton(title: ProjectsText.settings) { Task { await projects.select(project.root) } }.recordFrame("project.settings.\(project.name)")
                    LinkButton(title: ProjectsText.revoke) { Task { await projects.revoke(project.root) } }.recordFrame("project.revoke.\(project.name)")
                }
                .frame(minHeight: SettingsMetric.rowMinHeight + Space._2)
            }
        }
        if !projects.canApprove { Banner(kind: .warning, text: ProjectsText.noApproval) }
    }
}
