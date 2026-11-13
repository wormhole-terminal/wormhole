// A pane's header, which only shows once a tab is split: its status tile, what runs in it and where, then
// Zoom, the pane menu and Close. The focused pane's header wears the primary line (without gaps) and full
// text; the others are muted. A click gives the pane the keyboard; a right-click opens the pane menu; the
// whole header is what is dragged to rearrange (`style::pane_header`, `window/panes/render.rs`).

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

struct PaneHeader<Menu: View>: View {
    let info: PaneInfo
    let focused: Bool
    let gaps: Bool
    let zoomed: Bool
    let dragged: DraggedTerminal
    let position: Int
    let of: Int
    let focus: () -> Void
    let zoom: () -> Void
    let close: () -> Void
    @ViewBuilder let menu: () -> Menu
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            TerminalTile(activity: info.activity)
            AtMostWidth(width: PaneMetrics.nameMax) { Text(info.title).lineLimit(1).truncationMode(.tail) }
                .layoutPriority(1)
            Text(info.folder.map { Elide.front($0, PaneMetrics.folderMax) } ?? "")
                .font(tokens.font(.caption).weight(.regular)).foregroundStyle(tokens.textFaint)
                .lineLimit(1).truncationMode(.head).frame(maxWidth: .infinity, alignment: .leading)
            buttons
        }
        .font(tokens.font(.caption).weight(.medium))
        .foregroundStyle(focused ? tokens.textPrimary : tokens.textMuted)
        .padding(.leading, gaps ? PaneMetrics.inset : Space._3)
        .padding(.trailing, Space._1)
        .frame(height: gaps ? PaneMetrics.gapHeader : PaneMetrics.rowHeader)
        .overlay(alignment: .bottom) { rules }
        .contentShape(Rectangle())
        .onTapGesture(perform: focus)
        .contextMenu(menuItems: menu)
        // Dragged by its header: to the sidebar the terminal leaves the split, onto another pane's edge it
        // moves there.
        .draggable(dragged) { DragChip(info: info) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(PanesText.paneLabel(info.title, folder: info.folder, position: position, of: of, focused: focused))
    }

    /// From the right: Close never gives way, the menu next, Zoom first.
    private var buttons: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 0) { zoomButton; moreButton; closeButton }
            HStack(spacing: 0) { moreButton; closeButton }
            closeButton
        }
        .fixedSize()
    }

    private var zoomButton: some View {
        PaneButton(symbol: zoomed ? "arrow.down.right.and.arrow.up.left" : "arrow.up.left.and.arrow.down.right",
                   label: zoomed ? PanesText.unzoom : PanesText.zoom, key: "⇧⌘↩", action: zoom)
    }

    private var moreButton: some View {
        SwiftUI.Menu(content: menu) {
            Image(systemName: "ellipsis").font(.system(size: PaneMetrics.glyph))
                .frame(width: PaneMetrics.button, height: PaneMetrics.button)
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .foregroundStyle(tokens.textMuted)
        .help(PanesText.paneMenu).accessibilityLabel(PanesText.paneMenu)
    }

    private var closeButton: some View { PaneButton(symbol: "xmark", label: PanesText.closePane, key: "⌘W", action: close) }

    /// Without gaps a hairline under every header and the primary line under the focused one; with gaps the
    /// pane's own outline says which has the keyboard (`PaneFrame`).
    @ViewBuilder private var rules: some View {
        if !gaps {
            ZStack(alignment: .bottom) {
                Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                if focused { Rectangle().fill(tokens.primary).frame(height: PaneMetrics.focusLine) }
            }
        }
    }
}
