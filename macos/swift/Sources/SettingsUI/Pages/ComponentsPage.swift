// Settings → Components (`sections/components/`), behind the `components` flag: the controls the pages are built from, drawn in
// one place on sample state so how they look can be tuned without hunting through the pages. Each specimen is the real control,
// called the way the pages call it; its state lives here and is written nowhere (no key of config.json is touched).

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ComponentsPage: View {
    @State private var on = true
    @State private var checked = true
    @State private var choice: JSONValue = .string("b")
    @State private var segment: JSONValue = .string("one")
    @State private var level = 0.65
    @State private var count = 14.0
    @State private var text = ""
    @State private var picked: String? = "#7aa2f7"
    @Environment(\.tokens) private var tokens

    private static let options = ["a", "b", "c"].map { SettingOption(value: .string($0), label: $0.uppercased()) }
    private static let segments = ["one", "two", "three"].map { SettingOption(value: .string($0), label: $0.capitalized) }
    private static let slider = ControlSpec.Slider(min: 0.3, max: 1, step: 0.01, format: .percent, ticks: [0.3, 0.65, 1])
    private static let swatches = ["#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff"] // text: sample colours

    var body: some View {
        PageScaffold(title: Words.text("settings-section-components", fallback: "Components"), intro: Self.intro, action: { EmptyView() }) {
            rows
            controls
            buttons
            marks
            banners
        }
    }

    private static let intro = "Every control the pages are built from, on sample state. Nothing here is saved." // text: a developer page's caption

    /// The setting row in each of its states.
    private var rows: some View {
        SettingGroup(title: "Rows", blurb: "SettingRow: default, changed, found by a search, revealed, with a chip, a note under it.") { // text: a developer page's captions
            specimen("Default", "A description under the label.", RowState()) { SettingToggle(label: "Default", isOn: on) { on = $0 } } // text: sample row
            specimen("Changed", "The dot and the reset arrow.", RowState(isChanged: true)) { SettingToggle(label: "Changed", isOn: !on) { on = !$0 } } // text: sample row
            specimen("Found", "Marked where a search matched.", RowState(found: 0..<5)) { SettingToggle(label: "Found", isOn: on) { on = $0 } } // text: sample row
            specimen("Revealed", "Marked for a moment after a result was chosen.", RowState(isRevealed: true)) { SettingToggle(label: "Revealed", isOn: on) { on = $0 } } // text: sample row
            SettingRow(label: "With chip and note", description: "A tag after the label, a note below.", visibleHint: true, state: RowState(), chip: "Beta", below: AnyView(Banner(kind: .info, text: "A note under the row.")), onReset: {}) { // text: sample row
                SettingToggle(label: "With chip", isOn: on) { on = $0 } // text: sample control
            }
        }
    }

    private var controls: some View {
        SettingGroup(title: "Controls", blurb: "Switch, checkbox, select, segmented, slider, stepper, field, swatches.") { // text: a developer page's captions
            specimen("Switch and checkbox", "", RowState()) { // text: sample row
                HStack(spacing: Space._4) {
                    SettingToggle(label: "Switch", isOn: on) { on = $0 } // text: sample control
                    SettingToggle(label: "Checkbox", isOn: checked, isCheckbox: true) { checked = $0 } // text: sample control
                }
            }
            specimen("Select", "", RowState()) { OptionPicker(options: Self.options, selected: choice, label: "Select") { choice = $0 } } // text: sample row
            specimen("Segmented", "", RowState()) { OptionPicker(options: Self.segments, selected: segment, label: "Segmented", segmented: true) { segment = $0 } } // text: sample row
            specimen("Slider", "", RowState()) { SettingSlider(spec: Self.slider, value: level, label: "Slider") { level = $0 } } // text: sample row
            specimen("Stepper", "", RowState()) { // text: sample row
                ValueStepper(value: "\(Int(count)) pt", label: "Stepper", decrease: count > 8 ? { count -= 1 } : nil, increase: count < 32 ? { count += 1 } : nil) // text: sample control
            }
            specimen("Field", "", RowState()) { InputField(placeholder: "Placeholder", text: $text, onCommit: {}) } // text: sample row
            specimen("Swatches", "", RowState()) { SwatchRow(colors: Self.swatches, chosen: picked) { picked = $0 } } // text: sample row
        }
    }

    private var buttons: some View {
        SettingGroup(title: "Buttons", blurb: "ActionButton, destructive, disabled, LinkButton.") { // text: a developer page's captions
            specimen("Buttons", "", RowState()) { // text: sample row
                HStack(spacing: Space._2) {
                    ActionButton(title: "Action", symbol: Icon.refresh.rawValue) {} // text: sample control
                    ActionButton(title: "Destructive", symbol: Icon.trash.rawValue, isDestructive: true) {} // text: sample control
                    ActionButton(title: "Disabled", symbol: nil) {}.disabled(true) // text: sample control
                    LinkButton(title: "Link", goes: true) {} // text: sample control
                }
            }
        }
    }

    private var marks: some View {
        SettingGroup(title: "Marks", blurb: "RowChip, StatusBadge, AgentMark.") { // text: a developer page's captions
            specimen("Marks", "", RowState()) { // text: sample row
                HStack(spacing: Space._3) {
                    RowChip(text: "Chip") // text: sample control
                    StatusBadge(text: "On", color: tokens.statusSuccessText) // text: sample control
                    StatusBadge(text: "Incomplete", color: tokens.statusWarningText) // text: sample control
                    AgentMark(text: "Cl", size: Metric.tile) // text: sample control
                }
            }
        }
    }

    private var banners: some View {
        SettingGroup(title: "Banners", blurb: "One per kind.") { // text: a developer page's captions
            VStack(alignment: .leading, spacing: Space._2) {
                Banner(kind: .info, text: "Info") // text: sample control
                Banner(kind: .warning, text: "Warning") { ActionButton(title: "Action", symbol: nil) {} } // text: sample control
                Banner(kind: .danger, text: "Danger") // text: sample control
                Banner(kind: .success, text: "Success") // text: sample control
            }
            .padding(.vertical, Space._2)
        }
    }

    private func specimen<C: View>(_ label: String, _ description: String, _ state: RowState, @ViewBuilder control: () -> C) -> some View {
        SettingRow(label: label, description: description, visibleHint: !description.isEmpty, state: state, onReset: {}, control: control)
    }
}
