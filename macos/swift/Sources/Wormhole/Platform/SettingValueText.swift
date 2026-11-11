// A setting's value as its row shows it at the palette's right (the GPUI palette's `value_text`): a choice's config word
// as a name, a switch as On or Off, the opacity as its slider's percentage, a count grouped, a fraction as the page writes it.

import Foundation
import SettingsModel
import WordsModel

enum SettingValueText {
    static func of(key: String, _ value: JSONValue) -> String? {
        switch value {
        case .string(let text) where !text.isEmpty: text.prefix(1).uppercased() + text.dropFirst()
        case .bool(let on): on ? Words.text("platform-value-on", fallback: "On") : Words.text("platform-value-off", fallback: "Off")
        case .number(let number):
            if key == "opacity" { "\(Int((number * 100).rounded())) %" }  // text: a number with its unit
            else if number == number.rounded(), number >= 0 { Int(number).formatted(.number) }
            else { "\((number * 100).rounded() / 100)" }  // text: a number
        default: nil
        }
    }
}
