// The settings window's parts on sample data, for the Theme Studio's gallery (`gallery/settings.rs`): the nav as it is idle
// and while a search runs, a group's rows in each state, the shortcut recorder and the title row's arranged list. They are
// the window's own views over an in-memory backend, so a theme edit shows on the real ones.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SettingsModel
import SwiftUI

/// The Settings window's nav, setting rows, recorder and arranged list on sample data, never acting on anything real.
public struct SettingsSpecimen: View {
    @State private var idle = SettingsSpecimen.model(query: "")
    @State private var searching = SettingsSpecimen.model(query: "terminal")
    @FocusState private var focus: SettingsFocus?
    @Environment(\.tokens) private var tokens

    public init() {}

    public var body: some View {
        HStack(alignment: .top, spacing: Space._6) {
            VStack(spacing: Space._3) {
                SettingsNav(model: idle, focus: $focus).frame(height: Self.navHeight)
                SettingsNav(model: searching, focus: $focus).frame(height: Self.navHeight)
            }
            .frame(width: SettingsMetric.navWidth)
            VStack(alignment: .leading, spacing: Space._4) {
                rows
                recorders
                if let parts = Self.titleParts(idle) { PartsList(model: idle, key: "title_row", parts: parts) }
            }
            .frame(maxWidth: SettingsMetric.introWidth, alignment: .leading)
        }
    }

    /// Tall enough for the nav's profile row, Changed, and the pages under their headings.
    private static let navHeight: CGFloat = 320

    /// The setting row in each of its states: default, changed, the reset arrow kept in view, set by a project, a slider,
    /// a link, found by a search, and disabled (`setting_rows`).
    private var rows: some View {
        SettingGroup(title: "Window", aside: "A note at the right of the head", blurb: "A paragraph under the head, in muted caption text.") {
            row("Blur what's behind the window", "Only visible below 100 % opacity") { SettingToggle(label: "Blur", isOn: true) { _ in } }
            row("Line height", "Terminal rows, as a multiple of the font size", RowState(isChanged: true)) {
                ValueStepper(value: "1.45", label: "Line height", decrease: {}, increase: {})
            }
            row("Tab width", "Columns a tab stop is wide", RowState(isChanged: true, resetAlways: true)) {
                ValueStepper(value: "8", label: "Tab width", decrease: {}, increase: {})
            }
            row("Theme", "Set by api-server/.wormhole/theme.json while this terminal is active", visibleHint: true) {
                LinkButton(title: "Edit in project") {}
            }
            row("Background opacity", "30–100 %. Below 100 % the desktop shows through") {
                if let slider = Self.slider(idle) { SettingSlider(spec: slider, value: 0.8, label: "Background opacity") { _ in } }
            }
            row("Font", "JetBrains Mono 14 pt · line height 1.30") { LinkButton(title: "Edit in Fonts", goes: true) {} }
            row("Wrap long lines", "Otherwise the diff scrolls sideways", RowState(found: 10..<14)) {
                SettingToggle(label: "Wrap", isOn: false) { _ in }
            }
            row("Terminal layout", "No layout.json in this project", visibleHint: true) {
                SettingToggle(label: "Layout", isOn: false) { _ in }.disabled(true)
            }
        }
    }

    private var recorders: some View {
        HStack(spacing: Space._3) {
            ForEach(["", "shift-cmd-space"], id: \.self) { spec in
                ShortcutRecorder(spec: spec, label: "Shortcut", check: { _ in nil }, onRecording: { _ in }, onChange: { _ in })
            }
        }
    }

    private func row<Control: View>(
        _ label: String, _ description: String, _ state: RowState = RowState(), visibleHint: Bool = false,
        @ViewBuilder control: () -> Control
    ) -> some View {
        SettingRow(label: label, description: description, visibleHint: visibleHint, state: state, onReset: {}, control: control)
    }

    /// Two real schema pages on the mock backend: some values changed so the nav carries its count, and one with a search
    /// running so it shows each page's results.
    private static func model(query: String) -> SettingsModel {
        let backend = MockSettingsBackend(overrides: ["line_height": .number(1.45), "tab_width": .number(8)])
        let model = SettingsModel(backend: backend, host: MockSettingsHost(), selection: "fonts")
        model.query = query
        return model
    }

    private static func slider(_ model: SettingsModel) -> ControlSpec.Slider? {
        if case .slider(let spec)? = model.schema.entry("opacity")?.entry.control { spec } else { nil }
    }

    private static func titleParts(_ model: SettingsModel) -> [ControlSpec.TitlePart]? {
        if case .partsList(let parts)? = model.schema.entry("title-row")?.entry.control { parts } else { nil }
    }
}
