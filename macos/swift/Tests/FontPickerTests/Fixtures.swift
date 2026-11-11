// A catalog and a picker over it, the families being the ones the core's own tests use (`theme/fonts/mod.rs`).

@testable import FontPickerModel

@MainActor
enum Fixtures {
    static func catalog(classified: Bool = true) -> FontCatalog {
        var catalog = FontCatalog()
        catalog.installed = ["Menlo", "JetBrains Mono", "Arial", "Fira Code", "Monaspace Neon", ".SF NS", "Zapfino", "Monaco"]
        catalog.bundled = ["JetBrains Mono"]
        catalog.custom = [CustomFamily(name: "Fira Code", monospace: true), CustomFamily(name: "Iosevka", monospace: true)]
        catalog.monospace = classified ? ["JetBrains Mono", "Iosevka", "Fira Code", "Menlo", "Monaco"] : nil
        catalog.symbolic = ["Zapfino"]
        return catalog
    }

    /// A library already loaded with `catalog` (no system to ask).
    static func library(_ catalog: FontCatalog = catalog(), recents: any FontRecents = MemoryFontRecents()) -> FontLibrary {
        FontLibrary(
            rules: LocalFontRules(),
            system: StubSystemFonts(
                installed: Dictionary(uniqueKeysWithValues: catalog.installed.map { ($0, FontTraits(monospace: catalog.isMonospace($0), symbolic: catalog.symbolic.contains($0))) }),
                covered: ["JetBrains Mono"], joins: ["JetBrains Mono", "Fira Code"]),
            recents: recents, known: catalog)
    }

    static func picker(
        slot: String = "terminal", family: String = "Menlo", library: FontLibrary = library(),
        picked: PickLog = PickLog()
    ) -> (FontPickerModel, PickLog) {
        let slotSpec: FontPickerSlot =
            switch slot {
            case "interface": FontPickerSlot(key: slot, inheritLabel: "System font", sizes: 10...20)
            case "code": FontPickerSlot(key: slot, inheritLabel: "Same as terminal", sizes: 8...36)
            default: FontPickerSlot(key: slot, inheritLabel: nil, sizes: 8...36)
            }
        picked.family = family
        let model = FontPickerModel(
            slot: slotSpec, library: library, current: { picked.family }, inherited: { "Menlo" }, pick: { picked.family = $0 })
        return (model, picked)
    }
}

@MainActor
final class PickLog {
    var family = ""
}
