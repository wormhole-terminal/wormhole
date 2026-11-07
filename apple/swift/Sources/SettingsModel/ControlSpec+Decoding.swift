// Reads one control from the dump: `kind` picks the case, the other fields are that case's.

import Foundation

extension ControlSpec: Decodable {
    private enum Key: String, CodingKey {
        case kind, options, fallback = "default", customKey, min, max, step, template, format, ticks, items
        case slot, inherit, parts, source, section, label, action, icon, info, panel, scene, stepKey, steps, value
        case hideable
    }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: Key.self)
        let kind = try c.decode(String.self, forKey: .kind)
        switch kind {
        case "toggle": self = .toggle
        case "recorder": self = .recorder
        case "colorToggle": self = .colorToggle
        case "themeCards": self = .themeCards
        case "segmented":
            self = .segmented(
                options: try c.decode([SettingOption].self, forKey: .options),
                fallback: try c.decodeIfPresent(JSONValue.self, forKey: .fallback),
                customKey: try c.decodeIfPresent(String.self, forKey: .customKey))
        case "select":
            self = .select(
                options: try c.decode([SettingOption].self, forKey: .options),
                fallback: try c.decodeIfPresent(JSONValue.self, forKey: .fallback))
        case "dynamicSelect": self = .dynamicSelect(try c.decode(OptionSource.self, forKey: .source))
        case "stepper":
            self = .stepper(
                .init(
                    min: try c.decode(Double.self, forKey: .min), max: try c.decode(Double.self, forKey: .max),
                    step: try c.decode(Double.self, forKey: .step), template: try c.decode(String.self, forKey: .template)))
        case "slider":
            self = .slider(
                .init(
                    min: try c.decode(Double.self, forKey: .min), max: try c.decode(Double.self, forKey: .max),
                    step: try c.decode(Double.self, forKey: .step),
                    format: Slider.Format(rawValue: try c.decode(String.self, forKey: .format)) ?? .decimal2,
                    ticks: try c.decode([Double].self, forKey: .ticks)))
        case "checkboxes":
            self = .checkboxes(try c.decode([Item].self, forKey: .items).map { .init(key: $0.key, label: $0.label) })
        case "font":
            self = .font(
                .init(
                    slot: try c.decode(String.self, forKey: .slot), min: try c.decode(Double.self, forKey: .min),
                    max: try c.decode(Double.self, forKey: .max), inherit: try c.decodeIfPresent(String.self, forKey: .inherit)))
        case "partsList":
            self = .partsList(
                try c.decode([Part].self, forKey: .parts).map { .init(value: $0.value, label: $0.label, hideable: $0.hideable) })
        case "preview": self = .preview(try c.decode(String.self, forKey: .scene))
        case "link": self = .link(section: try c.decode(String.self, forKey: .section), label: try c.decode(String.self, forKey: .label))
        case "action":
            self = .action(
                try c.decode(SettingsAction.self, forKey: .action), label: try c.decode(String.self, forKey: .label),
                icon: try c.decode(String.self, forKey: .icon))
        case "info": self = .info(try c.decode(String.self, forKey: .info))
        case "panel": self = .panel(try c.decode(String.self, forKey: .panel))
        case "toggleStepper":
            self = .toggleStepper(stepKey: try c.decode(String.self, forKey: .stepKey), steps: try c.decode([Int].self, forKey: .steps))
        default:
            throw DecodingError.dataCorruptedError(forKey: .kind, in: c, debugDescription: "unknown control kind \(kind)")
        }
    }

    private struct Item: Decodable { let key: String, label: String }
    private struct Part: Decodable { let value: String, label: String, hideable: Bool }
}
