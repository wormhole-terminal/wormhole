// The nav (`render_nav`): the search field and the profile select as a fixed header under the traffic lights,
// then the pages, Changed first without a heading and the others under their group's label. While a search runs
// each page shows how many results it holds and dims when it holds none. Arrow keys move through the pages.

import DesignSystem
import SettingsModel
import SwiftUI

struct SettingsNav: View {
    let model: SettingsModel
    var focus: FocusState<SettingsFocus?>.Binding
    @Environment(\.tokens) private var tokens
    @Namespace private var slide

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: Space._3) {
                NavSearchField(model: model, focus: focus)
                ProfileSelect(
                    name: model.profile.form?.name ?? SettingsText.profileDefault, letter: model.profile.form?.letter ?? "D",
                    color: model.profile.form?.color ?? "blue", isSelected: model.selection == "profile" && !model.isSearching) {
                    model.show("profile")
                }
            }
            .padding(.horizontal, SettingsMetric.navInset).padding(.top, Space._2).padding(.bottom, Space._3)
            ScrollView {
                VStack(spacing: Space._1) {
                    row(id: SettingsModel.changedID, title: SettingsText.changedTitle, mark: changedMark)
                    ForEach(groups, id: \.title) { group in
                        if !group.title.isEmpty { GroupLabel(title: group.title) }
                        ForEach(group.sections) { section in row(id: section.id, title: section.title, mark: mark(section)) }
                    }
                }
                .padding(.horizontal, SettingsMetric.navInset).padding(.bottom, Space._4)
            }
            .focusable().focused(focus, equals: .nav)
            .onKeyPress(.downArrow) { step(1) }
            .onKeyPress(.upArrow) { step(-1) }
            .accessibilityLabel(SettingsText.settings)
        }
        .padding(.top, Metric.titleRow)
        .frame(width: SettingsMetric.navWidth)
        .background(tokens.sidebarBackground)
    }

    private func row(id: String, title: String, mark: NavMark) -> some View {
        NavRow(id: id, title: title, mark: mark, isSelected: model.selection == id && !model.isSearching, slide: slide) {
            model.query = ""
            model.show(id)
        }
        .motion(Motion.selection, value: model.selection)
    }

    private var changedMark: NavMark { model.isSearching ? .none : .count(model.changedCount) }

    private func mark(_ section: SettingsSchema.Section) -> NavMark {
        if model.isSearching { return model.hitCounts[section.id].map(NavMark.hits) ?? .noHits }
        return section.preview ? .preview : .none
    }

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

    private func step(_ delta: Int) -> KeyPress.Result {
        let ids = [SettingsModel.changedID] + model.navSections.map(\.id)
        guard let index = ids.firstIndex(of: model.selection) else { return .ignored }
        model.show(ids[min(max(index + delta, 0), ids.count - 1)])
        return .handled
    }
}

private struct GroupLabel: View {
    let title: String
    @Environment(\.tokens) private var tokens
    var body: some View {
        Text(title.uppercased()).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
            .frame(maxWidth: .infinity, alignment: .leading).padding(.leading, Space._2).padding(.top, Space._3).padding(.bottom, Space._1)
            .accessibilityAddTraits(.isHeader)
    }
}
