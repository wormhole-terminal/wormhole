// text-file: the control folder's `settings-page` (dev tooling), not interface text
// The page Settings shows, as text (`settings-page`): what `SettingsPage`, `EntryRowView`, `RowExtras` and the nav draw,
// read from the same functions they call. It decides nothing: a condition written here is the view's own one-line guard,
// named beside it. The shape is `DesignSystem.Readout`'s:
//
//     settings-page appearance "Appearance" results=false
//     nav agents attention badge="2" dim
//     banner warning "config.json can't be read, so …"
//     row opacity "Opacity" value="80 %" changed off chip="Beta" project="api"
//       caption "The theme's · #7aa2f7. …"
//       note warning "Without blur the desktop shows sharp …" button="Turn on blur"
//       swatches #… chosen="#…"
//       problem "macOS uses this shortcut"
//       option "Login shell" "/bin/zsh"

import DesignSystem
import Foundation
import SettingsModel
import SwiftUI
import WordsModel

@MainActor
public enum SettingsPageReadout {
    public static func text(_ model: SettingsModel) -> String {
        let section = model.schema.sections.first { $0.id == model.selection }
        var lines = [
            Readout.line(
                "settings-page", [model.showsResults ? "results" : model.selection], quoted: [section?.title ?? ""],
                pairs: [("query", model.query)])
        ]
        for nav in model.navSections {
            lines.append(
                Readout.line(
                    "nav", [nav.id], pairs: SettingsNav.badge(nav, model).map { [("badge", $0)] } ?? [],
                    flags: [Readout.flag("attention", SettingsNav.attention(nav, model)), Readout.flag("dim", model.isSearching && model.hitCounts[nav.id] == nil)]))
        }
        if model.showsResults {
            for hit in model.hits { lines += row(hit.entry, model, found: true, context: hit.group) }
            return lines.joined(separator: "\n")
        }
        if model.selection == SettingsModel.changedID {
            for located in model.changedEntries { lines += row(located.entry, model, found: false, context: located.group.title) }
            return lines.joined(separator: "\n")
        }
        guard let section else { return lines.joined(separator: "\n") }
        lines += banners(section, model)
        if section.id == "fonts" { lines += specimen(model) }
        let found = Set(model.hits.map(\.entry.id))
        for group in section.groups {
            // `SettingsPage`: a row whose `visibleWhen` does not hold is not there, nor is the strip's scene.
            let rows = group.entries.filter { $0.isShown(values: model.values) && $0.control.layout != .strip }
            guard !rows.isEmpty else { continue }
            lines.append(Readout.line("group", quoted: [group.title]))
            for entry in rows { lines += row(entry, model, found: found.contains(entry.id), context: nil) }
        }
        return lines.joined(separator: "\n")
    }

    /// The banners over the groups, in `SettingsPage`'s order and on its conditions.
    private static func banners(_ section: SettingsSchema.Section, _ model: SettingsModel) -> [String] {
        var said: [(String, String)] = []
        if let note = model.host.previewNote(section: section.id) { said.append(("info", note)) }
        if model.zenOn, ZenNote.affects(section) {
            said.append(("info", Words.text("zen-settings-note", fallback: "Zen mode is on, so the window shows less than these settings say. They keep your own values and apply again when you leave it.")))
        }
        if section.id == "advanced", let broken = model.backend.banner() {
            said.append(("warning", broken))
        } else if model.backend.unreadable {
            said.append(("warning", SettingsText.configUnreadable))
        }
        if section.id == "profile", let problem = model.host.profilesProblem() { said.append(("warning", problem)) }
        if let problem = model.host.panelRecords().problem { said.append(("danger", problem)) }
        return said.map { Readout.line("banner", [$0.0], quoted: [$0.1]) }
    }

    /// A row as `EntryRowView` draws it: label, value, its marks, then what hangs under it.
    private static func row(_ entry: SettingsSchema.Entry, _ model: SettingsModel, found: Bool, context: String?) -> [String] {
        let caption = EntryRowView.caption(entry, model)
        let projectNote = EntryRowView.projectNote(entry, model)
        let project = projectNote == nil ? nil : model.host.panelRecords().project?.name
        var pairs: [(String, String)] = [("value", value(entry, model))]
        if let chip = RowExtras.chip(entry, model) { pairs.append(("chip", chip)) }
        if let project { pairs.append(("project", project)) }
        if let context { pairs.append(("group", context)) }
        var lines = [
            Readout.line(
                "row", [entry.id], quoted: [caption?.label ?? entry.label], pairs: pairs,
                flags: [
                    Readout.flag("changed", model.isChanged(entry)), Readout.flag("off", isOff(entry, model)), Readout.flag("found", found),
                    Readout.flag("revealed", model.revealed == entry.id),
                ])
        ]
        // What shows under the label: the project's note, else the caption, else the table's description when it is shown.
        if let shown = projectNote ?? caption?.text ?? (entry.visibleHint ? entry.description : nil), !shown.isEmpty {
            lines.append(Readout.under("caption", quoted: [shown]))
        }
        lines += below(entry, model)
        if case .recorder = entry.control, let problem = EntryControl.shortcutProblem(key: entry.keys.first ?? "", model: model) {
            lines.append(Readout.under("problem", quoted: [problem]))
        }
        if case .partsList(let parts) = entry.control { lines += self.parts(parts, key: entry.keys.first ?? "", model) }
        for option in options(entry, model) where option.detail != nil {
            lines.append(Readout.under("option", quoted: [option.label, option.detail ?? ""]))
        }
        return lines
    }

    /// The control's value in the words it shows: an option's label, a number in its format, on or off.
    private static func value(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> String {
        let key = entry.keys.first ?? ""
        let raw = model.value(key)
        switch entry.control {
        case .toggle: return raw.bool == true ? "on" : "off"
        case .segmented(let options, let fallback, _), .select(let options, let fallback):
            let current = raw == .null ? (fallback ?? .null) : raw
            return options.first { $0.value == current }?.label ?? RowCaptions.handWritten(key, current) ?? "\(current)"
        case .dynamicSelect(let source): return model.host.options(for: source).first { $0.value == raw }?.label ?? "\(raw)"
        case .stepper(let spec): return raw.number.map(spec.format) ?? "\(raw)"
        case .slider(let spec): return spec.format(raw.number ?? spec.min)
        case .checkboxes(let items): return items.map { "\($0.label)=\(model.value($0.key).bool == true ? "on" : "off")" }.joined(separator: ",")
        case .recorder: return raw.string ?? ""
        default: return entry.keys.map { "\(model.value($0))" }.joined(separator: ",")
        }
    }

    private static func options(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> [SettingOption] {
        switch entry.control {
        case .segmented(let options, _, _), .select(let options, _): options
        case .dynamicSelect(let source): model.host.options(for: source)
        default: []
        }
    }

    /// The system makes every window opaque (what `ThemeEnvironment` hands the views as their environment).
    private static var opaque: Bool { ThemeStore.shared.reduceTransparency || ThemeStore.shared.increaseContrast }

    /// A control drawn off: the opacity slider while the system makes windows opaque (`SettingSlider.off`), Reset tips with
    /// none closed, Make default while it cannot be (`EntryControl`).
    private static func isOff(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> Bool {
        switch entry.control {
        case .slider: entry.id == "opacity" && opaque
        case .action(.resetTips, _, _): model.host.dismissedTips == 0
        case .action(.makeDefaultTerminal, _, _): model.host.isDefaultTerminal() != false
        default: false
        }
    }

    /// What `RowExtras.below` puts under the row, on each view's own guard.
    private static func below(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> [String] {
        let held = RowExtras.projectSets(entry, model)
        switch entry.id {
        case "opacity" where opaque:
            let words = ThemeStore.shared.reduceTransparency
                ? Words.text("settings-appearance-opaque-reduce-transparency", fallback: "Reduce Transparency is on in System Settings, so every window is opaque. Your opacity applies again when it's off.")
                : Words.text("settings-appearance-opaque-increase-contrast", fallback: "Increase Contrast is on in System Settings, so every window is opaque. Your opacity applies again when it's off.")
            return [Readout.under("note", ["info"], quoted: [words])]
        case "blur" where model.host.blurIsUnsafe(opacity: model.value("opacity").number ?? 1, blur: model.value("blur").bool ?? true):
            let words = Words.text("settings-appearance-blur-off-warning", fallback: "Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read.")
            let button = held ? [] : [("button", Words.text("settings-appearance-turn-on-blur", fallback: "Turn on blur"))]
            return [Readout.under("note", ["warning"], quoted: [words], pairs: button)]
        case "liquid-glass" where (model.value("liquid_glass").bool ?? false) && opaque:
            return [Readout.under("note", ["info"], quoted: [Words.text("settings-appearance-glass-held-by-accessibility", fallback: "Reduce Transparency or Increase Contrast is on in System Settings, so every surface stays painted. The glass returns when it's off.")])]
        case "primary-color" where !(model.value("theme_primary").bool ?? true) && !held:
            let raw = (model.value("primary_color").string ?? "").trimmingCharacters(in: .whitespaces).lowercased()
            let chosen = Color(hex: raw) == nil ? nil : raw
            let skip = ThemeStore.shared.theme.tokens().rgba(named: "primary")?.hexOpaque
            return [swatches(offeredSwatches(ansi: [2, 5, 1, 3, 6, 4], skip: skip, chosen: chosen), chosen)]
        case "branch-color" where model.value("branch_color").string == "custom" && !held:
            let text = (model.value("branch_color_custom").string ?? "").trimmingCharacters(in: .whitespaces).lowercased()
            let chosen = Color(hex: text) == nil ? nil : text
            return [swatches(offeredSwatches(ansi: [5, 1, 3, 2, 6, 4], skip: nil, chosen: chosen), chosen, well: true)]
        default: return []
        }
    }

    /// The title row's parts as `PartsList` lists them: the shown ones in order, then the hidden ones, each with its note.
    private static func parts(_ parts: [ControlSpec.TitlePart], key: String, _ model: SettingsModel) -> [String] {
        let order = (model.value(key).array ?? []).compactMap(\.string)
        let shown = order.compactMap { value in parts.first { $0.value == value } }
        let hidden = parts.filter { part in !order.contains(part.value) }
        return (shown + hidden).map { part in
            Readout.under(
                "part", [part.value], quoted: [part.label], pairs: PartsList.note(part, model).map { [("note", $0)] } ?? [],
                flags: [order.contains(part.value) ? "shown" : "hidden", Readout.flag("hideable", part.hideable)])
        }
    }

    /// The Fonts page's specimen (`FontsScene`): each tab's family, size and facts line, and the font a picker is looking at.
    private static func specimen(_ model: SettingsModel) -> [String] {
        var lines = ["terminal", "code", "interface"].map { slot in
            let family = FontsScene.familyOf(slot, model), size = FontsScene.sizeOf(slot, model)
            return Readout.line(
                "specimen", [slot], quoted: [FontsScene.facts(family: family, size: size, theme: PreviewTheme.name(model), slot: slot, model: model)],
                pairs: [("family", family), ("size", String(format: "%.0f", size))])
        }
        if let preview = model.fontPreview {
            lines.append(Readout.line("preview", [preview.slot], pairs: [("family", preview.family), ("size", String(format: "%.0f", preview.size))]))
        }
        return lines
    }

    private static func swatches(_ colors: [String], _ chosen: String?, well: Bool = false) -> String {
        Readout.under("swatches", colors, pairs: [("chosen", chosen ?? "-")], flags: [Readout.flag("well", well)])
    }
}
