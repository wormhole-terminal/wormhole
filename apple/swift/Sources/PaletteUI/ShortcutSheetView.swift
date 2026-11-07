// The keyboard-shortcuts sheet (`shortcut_sheet.rs`): every key by section in up to three columns, with a filter
// that finds a row by its label or its keys ("⌘W" finds Close terminal). Esc clears the filter, then closes.

import DesignSystem
import PaletteModel
import SwiftUI

public struct ShortcutSheetView: View {
    public let dismiss: @MainActor () -> Void
    let startsPresented: Bool

    @State private var filter: String
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens

    public init(dismiss: @escaping @MainActor () -> Void) {
        self.init(dismiss: dismiss, startsPresented: false)
    }

    init(dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, filter: String = "") {
        self.dismiss = dismiss
        self.startsPresented = startsPresented
        _filter = State(initialValue: filter)
    }

    public var body: some View {
        let found = ShortcutSheet.filter(filter)
        OverlayChrome(style: .sheet, top: OverlayMetrics.sheetTop, startsPresented: startsPresented, dismiss: dismiss) { context in
            let layout = SheetLayout(
                window: context.size, sectionRows: found.sections.map(\.rows.count),
                filtering: !filter.trimmingCharacters(in: .whitespaces).isEmpty)
            VStack(spacing: 0) {
                SheetHeader(filter: $filter) { key in
                    guard key == .escape else { return false }
                    if filter.isEmpty { context.close(nil) } else { filter = "" }
                    return true
                }
                SheetBody(found: found, layout: layout)
                SheetFooter(found: found, filtering: !filter.trimmingCharacters(in: .whitespaces).isEmpty)
            }
            .frame(width: layout.width)
            .frame(maxHeight: layout.maxHeight, alignment: .top)
            .fixedSize(horizontal: false, vertical: true)
            .overlayCard()
            .animation(OverlayMotion(reduceMotion: reduceMotion).resize, value: layout)
        }
        .accessibilityLabel(Text("Keyboard shortcuts"))
    }
}
