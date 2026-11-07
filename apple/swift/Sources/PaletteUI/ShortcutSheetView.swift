// The keyboard-shortcut sheet: every key by section, with a filter that finds a row by its label or its keys.

import DesignSystem
import PaletteModel
import SwiftUI

public struct ShortcutSheetView: View {
    public let dismiss: @MainActor () -> Void
    let startsPresented: Bool

    @State private var filter = ""
    @FocusState private var focused: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(dismiss: @escaping @MainActor () -> Void) {
        self.init(dismiss: dismiss, startsPresented: false)
    }

    init(dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, filter: String = "") {
        self.dismiss = dismiss
        self.startsPresented = startsPresented
        _filter = State(initialValue: filter)
    }

    private var motion: PaletteMotion { PaletteMotion(reduceMotion: reduceMotion) }

    public var body: some View {
        let sections = ShortcutSheet.sections(matching: filter)
        OverlayChrome(width: 520, startsPresented: startsPresented, dismiss: dismiss) { close in
            VStack(spacing: 0) {
                SearchField(prompt: "Filter shortcuts…", text: $filter, focus: $focused)
                Divider().overlay(Palette.divider)
                if sections.isEmpty {
                    Text("No shortcuts match “\(filter)”")
                        .font(.system(size: 14))
                        .foregroundStyle(Palette.textSecondary)
                        .frame(maxWidth: .infinity, minHeight: 200)
                } else {
                    list(sections)
                }
            }
            .onKeyPress(.escape) { close(); return .handled }
            .task { focused = true }
        }
        .accessibilityLabel(Text("Keyboard shortcuts"))
    }

    private func list(_ sections: [ShortcutSection]) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Space._4) {
                ForEach(sections) { section in
                    VStack(alignment: .leading, spacing: Space._1) {
                        Text(section.title.uppercased())
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(Palette.textSecondary)
                            .padding(.horizontal, Space._2)
                            .accessibilityAddTraits(.isHeader)
                        ForEach(section.rows) { row in
                            shortcutRow(row).transition(.opacity)
                        }
                    }
                }
            }
            .padding(Space._2)
            .animation(motion.results, value: sections)
        }
        .frame(height: 440)
        .scrollBounceBehavior(.basedOnSize)
    }

    private func shortcutRow(_ row: ShortcutRow) -> some View {
        HStack(spacing: Space._2) {
            Text(row.label).font(.system(size: 14)).lineLimit(1)
            Spacer(minLength: Space._4)
            ForEach(Array(row.keys.enumerated()), id: \.offset) { index, shortcut in
                if index > 0 && row.isRange {
                    Text("…").foregroundStyle(Palette.textSecondary)
                }
                KeyCaps(shortcut: shortcut)
            }
        }
        .padding(.horizontal, Space._2)
        .frame(height: Space._6 + Space._4)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text("\(row.label), \(row.keysText)"))
    }
}
