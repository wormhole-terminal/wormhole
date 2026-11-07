// The terminal's fonts from Settings → Fonts (`font_family`, `font_size`, `line_height`, `font_ligatures`). The bundled
// family is the four files of the repository's folder when it can be found, and an installed family of the same name
// otherwise.

import Foundation
import SettingsModel
import TerminalMetal

enum FontSettingsMaker {
    static let bundledFamily = "JetBrains Mono"

    static func settings(_ values: [String: JSONValue], bundledDirectory: URL?) -> TerminalFontSettings {
        let family = values["font_family"]?.string.flatMap { $0.isEmpty ? nil : $0 } ?? bundledFamily
        let spec: FontSpec =
            if family == bundledFamily, let bundledDirectory { .jetBrainsMono(directory: bundledDirectory) } else { .system(family: family) }
        return TerminalFontSettings(
            font: spec, pointSize: values["font_size"]?.number ?? 14, lineHeight: values["line_height"]?.number ?? 1.3,
            ligatures: values["font_ligatures"]?.bool ?? true)
    }
}
