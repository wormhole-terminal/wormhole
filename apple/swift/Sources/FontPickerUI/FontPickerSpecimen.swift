// The open picker on sample data, for the Theme Studio's gallery: a library that already knows its families (the ones
// every Mac has, plus the bundled one and a "custom" one so the groups show), CoreText for what the preview says, and
// the keyboard left alone.
// text-file: the demo catalog: font families, which are names and not interface words

import DesignSystem
import FontPickerModel
import SwiftUI

public struct FontPickerSpecimen: View {
    @State private var model: FontPickerModel

    public init() {
        var catalog = FontCatalog()
        catalog.installed = ["Menlo", "Monaco", "Courier New", "SF Mono", "Helvetica Neue", "Times New Roman", "Zapfino"]
        catalog.bundled = ["JetBrains Mono"]
        catalog.custom = [CustomFamily(name: "Fira Code", monospace: true)]
        catalog.monospace = ["Menlo", "Monaco", "Courier New", "SF Mono", "JetBrains Mono", "Fira Code"]
        catalog.symbolic = ["Zapfino"]
        let library = FontLibrary(
            rules: LocalFontRules(), system: CoreTextFonts(), recents: MemoryFontRecents(), known: catalog)
        let slot = FontPickerSlot(key: "terminal", inheritLabel: nil, sizes: 8...36)
        _model = State(
            initialValue: FontPickerModel(
                slot: slot, library: library, current: { "JetBrains Mono" }, inherited: { "JetBrains Mono" }, pick: { _ in }))
    }

    public var body: some View {
        FontPickerContent(model: model, look: PreviewLook(size: 13, lineHeight: 1.3, ligatures: true), autofocus: false)
            .onAppear { model.isOpen = true }
    }
}
