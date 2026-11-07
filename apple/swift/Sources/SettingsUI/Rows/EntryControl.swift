// The control of one row, drawn from its `ControlSpec` and the model's values: stateless pieces wired to
// `model.set`. This is the one place that knows which key a control reads and writes.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct EntryControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    private var key: String { entry.keys.first ?? "" }

    var body: some View {
        switch entry.control {
        case .toggle:
            FlatSwitch(isOn: model.value(key).bool ?? false, label: entry.label) { model.set(key, .bool($0)) }
        case .segmented(let options, let fallback, _):
            FlatSegmented(options: options, selected: current(fallback), label: entry.label) { model.set(key, $0) }
        case .select(let options, let fallback):
            FlatSelect(options: options, selected: current(fallback), label: entry.label) { model.set(key, $0) }
        case .dynamicSelect(.openIn): OpenInControl(model: model, key: key, label: entry.label)
        case .dynamicSelect(let source):
            FlatSelect(options: model.host.options(for: source), selected: model.value(key), label: entry.label) {
                model.set(key, $0)
            }
        case .stepper(let spec):
            NumberStepper(model: model, key: key, label: entry.label, spec: spec)
        case .slider(let spec):
            FlatSlider(spec: spec, value: model.value(key).number ?? spec.min, label: entry.label) {
                model.set(key, .number($0))
            }
        case .checkboxes(let items):
            HStack(spacing: Space._4) {
                ForEach(items, id: \.key) { item in
                    FlatCheckbox(title: item.label, isOn: model.value(item.key).bool ?? false) { model.set(item.key, .bool($0)) }
                }
            }
        case .font(let slot): FontControl(model: model, entry: entry, slot: slot)
        case .partsList(let parts): PartsList(model: model, key: key, parts: parts)
        case .recorder:
            ShortcutRecorder(spec: model.value(key).string ?? "", label: entry.label, check: { model.shortcuts?.check(key: key, spec: $0) }) {
                model.set(key, .string($0))
            }
        case .colorToggle: PrimaryColorControl(model: model, entry: entry)
        case .link(let section, let label): LinkButton(title: label, goes: true) { model.show(section) }
        case .action(.makeDefaultTerminal, let label, let icon): defaultTerminal(label, icon)
        case .action(.resetAll, let label, let icon): ResetAllButton(model: model, label: label, icon: icon)
        case .action(let action, let label, let icon):
            ActionButton(title: label, symbol: icon) { model.host.perform(action) }
        case .info(let name): InfoValue(name: name)
        case .toggleStepper(let stepKey, let steps): CommandDoneControl(model: model, entry: entry, stepKey: stepKey, steps: steps)
        case .panel(let name): PanelControl(model: model, entry: entry, name: name)
        case .themeCards, .preview: EmptyView()
        }
    }

    /// Make default, or that Wormhole already is (disabled, as GPUI's); disabled too where this build cannot be.
    private func defaultTerminal(_ label: String, _ icon: String) -> some View {
        let state = model.host.isDefaultTerminal()
        return ActionButton(
            title: state == true ? Words.text("settings-terminal-wormhole-is-the-default", fallback: "Wormhole is the default") : label,
            symbol: state == true ? "checkmark" : icon
        ) { model.host.perform(.makeDefaultTerminal) }
        .disabled(state != false)
    }

    /// The stored value, or what the row shows while the file says nothing.
    private func current(_ fallback: JSONValue?) -> JSONValue {
        let value = model.value(key)
        return value == .null ? (fallback ?? .null) : value
    }
}
