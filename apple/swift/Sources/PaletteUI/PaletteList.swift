// The scrolling list of results: headings while nothing is typed, the sliding selection, hover and scroll-to.

import DesignSystem
import PaletteModel
import SwiftUI

struct PaletteList: View {
    let results: [PaletteResult]
    let selection: String?
    @Binding var hovered: String?
    let grouped: Bool
    let motion: PaletteMotion
    let run: (PaletteResult) -> Void

    @Namespace private var selectionNamespace

    var body: some View {
        let items = PaletteItem.items(for: results, grouped: grouped)
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(items) { item in
                        switch item {
                        case .header(let title): header(title)
                        case .result(let result): row(result)
                        }
                    }
                }
                .padding(Space._2)
                .animation(motion.results, value: items.map(\.id))
                .animation(motion.selection, value: selection)
            }
            .frame(height: PaletteItem.height(of: items))
            .scrollBounceBehavior(.basedOnSize)
            .animation(motion.results, value: items.count)
            // Keeps the row under the keyboard in view without recentring the list.
            .onChange(of: selection) { _, id in
                if let id { proxy.scrollTo(id) }
            }
        }
    }

    private func header(_ title: String) -> some View {
        Text(title.uppercased())
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(Palette.textSecondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, Space._2)
            .frame(height: Space._6 + Space._1, alignment: .bottom)
            .accessibilityAddTraits(.isHeader)
            .transition(.opacity)
    }

    private func row(_ result: PaletteResult) -> some View {
        PaletteRow(
            result: result, isSelected: result.id == selection, isHovered: result.id == hovered,
            selectionNamespace: selectionNamespace, motion: motion
        )
        .id(result.id)
        .onHover { inside in
            if inside { hovered = result.id } else if hovered == result.id { hovered = nil }
        }
        .onTapGesture { run(result) }
        .transition(.opacity)
    }
}
