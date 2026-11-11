// The font pickers' rules from the Rust core (`wormhole-ffi` `fonts.rs`): which families a list shows and in what
// order (`grouped_font_choices`), what the user's fonts folder holds (`custom::scan`) and the bundled family. The
// installed families themselves come from CoreText, as they do in the GPUI app.

import FontPickerModel
import Foundation
import WormholeBindings

struct RustFontRules: FontRules {
    let fontsDir: String
    let bundledFiles: [URL]

    var bundledFamily: String { fontBundledFamily() }

    func scanCustomFonts() -> CustomFonts {
        let scan = fontCustomScan(dir: fontsDir)
        return CustomFonts(
            files: scan.files.map { URL(fileURLWithPath: $0.path) },
            families: scan.families.map { CustomFamily(name: $0.name, monospace: $0.monospace) },
            skipped: scan.skipped.count)
    }

    func choices(in catalog: FontCatalog, query: String, monospaceOnly: Bool) -> [FontChoice] {
        fontChoices(
            installed: catalog.installed, bundled: catalog.bundled, custom: catalog.customNames,
            monospace: catalog.monospace.map(Array.init), query: query, monospaceOnly: monospaceOnly
        ).map { FontChoice(source: Self.source($0.origin), name: $0.name) }
    }

    private static func source(_ origin: FontOrigin) -> FontSource {
        switch origin {
        case .bundled: .bundled
        case .custom: .custom
        case .installed: .installed
        }
    }
}
