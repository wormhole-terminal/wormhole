// Which control a row has, as `settings_dump/controls.rs` writes it. A value type, so the UI switches on it
// and a new kind is a compile error everywhere it is not drawn.

public enum ControlSpec: Sendable, Equatable {
    case toggle
    /// `fallback` is the option shown while the file says nothing (the motion choices); `customKey` is a second
    /// key a choice reveals a colour for (branch colour: Custom).
    case segmented(options: [SettingOption], fallback: JSONValue?, customKey: String?)
    case select(options: [SettingOption], fallback: JSONValue?)
    case dynamicSelect(OptionSource)
    case stepper(Stepper)
    case slider(Slider)
    case checkboxes([Checkbox])
    case font(FontSlot)
    case partsList([TitlePart])
    case recorder
    /// A switch (the theme's own colour) with a colour picked while it is off.
    case colorToggle
    case themeCards
    /// A scene of the strip above the page: a row that is no row.
    case preview(String)
    case link(section: String, label: String)
    case action(SettingsAction, label: String, icon: String)
    case info(String)
    /// A block the page draws from records of its own (agents, imports, the profile).
    case panel(String)
    /// A switch and, beside it, the number of seconds it waits for, in `steps`.
    case toggleStepper(stepKey: String, steps: [Int])

    public struct Stepper: Sendable, Equatable {
        public let min: Double, max: Double, step: Double
        /// The value in words with a placeholder number (`1234567`), which `format(_:)` swaps for the value.
        public let template: String

        public func format(_ value: Double) -> String {
            template.replacingOccurrences(of: "1234567", with: String(Int(value)))
        }
    }

    public struct Slider: Sendable, Equatable {
        public enum Format: String, Sendable { case percent, decimal2 }
        public let min: Double, max: Double, step: Double
        public let format: Format
        public let ticks: [Double]

        public func format(_ value: Double) -> String {
            switch format {
            case .percent: "\(Int((value * 100).rounded())) %"
            case .decimal2: String(format: "%.2f", value)
            }
        }
    }

    public struct Checkbox: Sendable, Equatable {
        public let key: String
        public let label: String
    }

    public struct FontSlot: Sendable, Equatable {
        public let slot: String
        public let min: Double, max: Double
        /// What an empty family follows ("Same as terminal"); `nil` for the terminal's own.
        public let inherit: String?
    }

    public struct TitlePart: Sendable, Equatable {
        public let value: String
        public let label: String
        public let hideable: Bool
    }
}
