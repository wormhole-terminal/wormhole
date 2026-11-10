// The keyboard-shortcuts sheet (`shortcut_sheet.rs`): every key by section in up to three columns, with a filter
// that finds a row by its label or its keys ("⌘W" finds Close terminal). Esc clears the filter, then closes.

import DesignSystem
import PaletteModel
import SwiftUI

public struct ShortcutSheetView: View {
    public let dismiss: @MainActor () -> Void
    /// The actions whose feature is off: the core leaves their rows out.
    let hidden: [String]
    /// The global show/hide key as the footer writes it ("⌃⌥Space"), when one is set.
    let hotkey: String?
    let startsPresented: Bool

    @State private var filter: String
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens

    public init(hidden: [String] = [], hotkey: String? = nil, dismiss: @escaping @MainActor () -> Void) {
        self.init(hidden: hidden, hotkey: hotkey, dismiss: dismiss, startsPresented: false)
    }

    init(hidden: [String], hotkey: String?, dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, filter: String = "") {
        self.hidden = hidden
        self.hotkey = hotkey
        self.dismiss = dismiss
        self.startsPresented = startsPresented
        _filter = State(initialValue: filter)
    }

    public var body: some View {
        let found = ListedKeymap.sheet(hidden: hidden, query: filter)
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
                SheetFooter(found: found, filtering: !filter.trimmingCharacters(in: .whitespaces).isEmpty, hotkey: hotkey)
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
