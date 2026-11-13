// The marker row of a fold (`terminal_view/render/folds.rs`): one line over the blank row the grid leaves for it, drawn by
// the app and never from output text. It says how many lines are hidden (and a Probe's fixed label), opens them on a click,
// Return or Space, and is a real accessibility element: a button named by Rust's spoken words, whose action is Show.

import DesignSystem
import SwiftUI
import TerminalCore

public struct FoldMarkers: View {
    let markers: [FoldMarkerRow]
    let geometry: BlockGeometry
    /// Width at the grid's right: the minimap's strip stays clear.
    let trailingInset: CGFloat
    let toggle: (FoldMarker) -> Void

    public init(markers: [FoldMarkerRow], geometry: BlockGeometry, trailingInset: CGFloat, toggle: @escaping (FoldMarker) -> Void) {
        (self.markers, self.geometry, self.trailingInset, self.toggle) = (markers, geometry, trailingInset, toggle)
    }

    public var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(markers, id: \.marker.id) { shown in
                MarkerRow(marker: shown.marker) { toggle(shown.marker) }
                    .frame(height: geometry.cellHeight)
                    .padding(.leading, geometry.gutter)
                    .padding(.trailing, trailingInset + geometry.gutter)
                    .offset(y: geometry.top + geometry.cellHeight * CGFloat(shown.row))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

private struct MarkerRow: View {
    let marker: FoldMarker
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._2) {
                Image(systemName: marker.isOpen ? "chevron.down" : "chevron.right")
                    .font(.system(size: tokens.size(.caption), weight: .semibold))
                Text(marker.label).font(.system(size: tokens.size(.caption))).lineLimit(1)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, Space._2)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            .foregroundStyle(marker.failed ? tokens.statusDanger : tokens.textMuted)
            .background(tokens.stateHover)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .pointerStyle(.link)
        .help(marker.tooltip ?? "")
        .accessibilityLabel(marker.spoken)
        .accessibilityAddTraits(.isButton)
        .onKeyPress(.return) {
            action()
            return .handled
        }
    }
}
