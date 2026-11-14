// The sidebar (`render_nav`) as the system draws one: a `List` in the sidebar style, so the material, the selection,
// the focus ring, arrow keys, type-select and each group's collapse are the system's. The profile is its first row (as
// the account is in System Settings), Changed comes next without a heading, then the pages under their group's name.
// While a search runs each page shows how many results it holds and dims when it holds none; nothing is selected.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct SettingsNav: View {
    @Environment(\.tokens) private var tokens
    let model: SettingsModel
    var focus: FocusState<SettingsFocus?>.Binding

    var body: some View {
        List(selection: Binding(get: { model.showsResults ? nil : model.selection }, set: { open($0) })) {
            profileRow
            row(id: SettingsModel.changedID, title: SettingsText.changedTitle, badge: changedBadge)
            groupRows
            // The developer page of every control, behind the `components` flag.
            if model.showsComponents {
                Section { row(id: SettingsModel.componentsID, title: Words.text("settings-section-components", fallback: "Components"), badge: nil) }
            }
        }
        .listStyle(.sidebar)
        .marked("settings.nav")
        .focused(focus, equals: .nav)
        // A page that comes or goes (the feature flags, unlocked with ⌃⌥⌘F) slides in or out; Reduce Motion cuts.
        .motion(Motion.layout, value: model.navSections.map(\.id))
        .accessibilityLabel(SettingsText.settings)
    }

    // The list's parts as properties of their own: as one expression the body is more than the type checker gets through.
    private var profileName: String { model.profile.form?.name ?? SettingsText.profileDefault }

    private var changedBadge: String? { model.isSearching || model.changedCount == 0 ? nil : "\(model.changedCount)" }

    private var profileRow: some View {
        Label {
            Text(profileName)
        } icon: {
            ProfileChip(letter: model.profile.form?.letter ?? "D", color: model.profile.form?.color ?? "blue")
        }
        .tag("profile")
        .accessibilityLabel(Words.text("settings-section-profile", fallback: "Profile"))
        .accessibilityValue(profileName)
    }

    private var groupRows: some View {
        ForEach(groups, id: \.title) { group in
            Section(group.title) {
                ForEach(group.sections) { section in
                    row(id: section.id, title: section.title, badge: badge(section), dim: dims(section), attention: attention(section))
                }
            }
        }
    }

    private func open(_ id: String?) {
        guard let id else { return }
        model.show(id)
    }

    private func row(id: String, title: String, badge: String?, dim: Bool = false, attention: Bool = false) -> some View {
        Label {
            HStack(spacing: Space._2) {
                Text(title)
                if attention { Circle().fill(tokens.statusWarning).frame(width: SettingsMetric.markDot, height: SettingsMetric.markDot).accessibilityHidden(true) }
            }
        } icon: {
            Image(systemName: Icon(section: id).rawValue)
        }
            .symbolVariant(!model.showsResults && model.selection == id ? .fill : .none)
            .badge(badge.map { Text($0) })
            .opacity(dim ? 0.5 : 1)
            .tag(id)
    }

    /// The Preview tag, or while a search runs the page's result count.
    private func badge(_ section: SettingsSchema.Section) -> String? { Self.badge(section, model) }

    static func badge(_ section: SettingsSchema.Section, _ model: SettingsModel) -> String? {
        if model.isSearching { return model.hitCounts[section.id].map(String.init) }
        return section.preview ? SettingsText.previewTag : nil
    }

    /// Coding agents wears a mark while an installed agent's hooks (or a profile's login of it) are only partly there.
    private func attention(_ section: SettingsSchema.Section) -> Bool { Self.attention(section, model) }

    /// The same, for the control folder's `settings-page`.
    static func attention(_ section: SettingsSchema.Section, _ model: SettingsModel) -> Bool {
        guard !model.showsResults, section.id == "agents" else { return false }
        let records = model.host.panelRecords()
        return records.agents.contains { $0.state == .needsRepair } || records.logins.contains { $0.state == .needsRepair }
    }

    private func dims(_ section: SettingsSchema.Section) -> Bool { model.isSearching && model.hitCounts[section.id] == nil }

    /// The sections in nav order under their heading; the profile and changed pages have a place of their own.
    private var groups: [(title: String, sections: [SettingsSchema.Section])] {
        var out: [(title: String, sections: [SettingsSchema.Section])] = []
        for section in model.navSections {
            if let last = out.last, last.title == section.navTitle {
                out[out.count - 1].sections.append(section)
            } else {
                out.append((section.navTitle, [section]))
            }
        }
        return out
    }
}
