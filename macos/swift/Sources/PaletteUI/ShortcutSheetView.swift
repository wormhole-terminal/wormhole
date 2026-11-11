// The keyboard-shortcuts sheet (`shortcut_sheet.rs`): every key by section in up to three columns, with a filter
// that finds a row by its label or its keys ("⌘W" finds Close terminal). Esc clears the filter, then closes.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

public struct ShortcutSheetView: View {
    public let dismiss: @MainActor () -> Void
    /// The actions whose feature is off: the core leaves their rows out.
    let hidden: [String]
    /// The global show/hide key as the footer writes it ("⌃⌥Space"), when one is set.
    let hotkey: String?
    /// The front terminal's trusted project: its own keys go in a block under the sections.
    let project: PaletteProject?
    let startsPresented: Bool

    @State private var filter: String
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens

    public init(hidden: [String] = [], hotkey: String? = nil, project: PaletteProject? = nil, dismiss: @escaping @MainActor () -> Void) {
        self.init(hidden: hidden, hotkey: hotkey, project: project, dismiss: dismiss, startsPresented: false)
    }

    init(
        hidden: [String], hotkey: String?, project: PaletteProject? = nil, dismiss: @escaping @MainActor () -> Void, startsPresented: Bool,
        filter: String = ""
    ) {
        self.hidden = hidden
        self.hotkey = hotkey
        self.project = project
        self.dismiss = dismiss
        self.startsPresented = startsPresented
        _filter = State(initialValue: filter)
    }

    public var body: some View {
        let keys = ProjectKeys(project, query: filter)
        let found = ListedKeymap.sheet(hidden: hidden, query: filter)
        OverlayChrome(style: .sheet, top: OverlayMetrics.sheetTop, startsPresented: startsPresented, dismiss: dismiss) { context in
            let layout = SheetLayout(
                window: context.size, sectionRows: found.sections.map(\.rows.count), projectRows: keys?.rows.count ?? 0,
                filtering: !filter.trimmingCharacters(in: .whitespaces).isEmpty)
            VStack(spacing: 0) {
                SheetHeader(filter: $filter) { key in
                    guard key == .escape else { return false }
                    if filter.isEmpty { context.close(nil) } else { filter = "" }
                    return true
                }
                SheetBody(found: found.counting(keys), project: keys, layout: layout)
                SheetFooter(found: found.counting(keys), filtering: !filter.trimmingCharacters(in: .whitespaces).isEmpty, hotkey: hotkey)
            }
            .frame(width: layout.width)
            .frame(maxHeight: layout.maxHeight, alignment: .top)
            .fixedSize(horizontal: false, vertical: true)
            .overlayCard()
            .animation(OverlayMotion(reduceMotion: reduceMotion).resize, value: layout)
        }
        .accessibilityLabel(Text(Words.text("palette-sheet-title", fallback: "Keyboard shortcuts")))
    }
}
