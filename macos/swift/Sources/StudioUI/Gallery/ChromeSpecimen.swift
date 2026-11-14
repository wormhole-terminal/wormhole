// The primitives of `DesignSystem` as the chrome draws them: icon buttons, count badges, every status glyph, a
// section label folded and open, a row in each state, and the row's × in the room that takes its click (`gallery/chrome.rs`).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SidebarUI
import SwiftUI

struct ChromeSpecimen: View {
    @Namespace private var selection
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._1) {
                ForEach(["plus", "sidebar.left", "magnifyingglass", "gearshape", "xmark"], id: \.self) { symbol in
                    IconButton(symbol: symbol, label: symbol) {}
                }
                Spacer(minLength: 0)
                CountBadge(3)
                CountBadge(1_234)
            }
            // The icons that answer a click or a change with their own drawing (`IconMotion`), at rest: the + and the refresh
            // and copy glyphs by their icon, the splits, the zoom button's swap and the right panel's tab glyphs.
            HStack(spacing: Space._4) {
                IconButton(lucide: .plus, label: "Add") {}
                IconButton(lucide: .refreshCw, label: "Refresh") {}
                IconButton(lucide: .copy, label: "Copy") {}
                // The toggle that is on, and the one with nothing to do now (`IconButton::marked`, `::disabled`).
                IconButton(lucide: .plus, label: "On") {}.selected(true)
                IconButton(lucide: .plus, label: "Unavailable") {}.disabled(true)
                LucideView(icon: .columns2, size: Metric.titleGlyph).iconMotion(.newPane(.right), run: 0)
                LucideView(icon: .rows2, size: Metric.titleGlyph).iconMotion(.newPane(.below), run: 0)
                IconSwap(key: false) { _ in LucideView(icon: .chevronDown, size: Metric.glyph) }
                LucideView(icon: .chevronDown, size: Metric.glyph).menuChevron(open: true)
                ForEach([Lucide.gitCompare, .commit, .fileText, .gauge, .folder], id: \.self) { TabGlyph(icon: $0, size: Metric.glyph, run: 0) }
            }
            .foregroundStyle(tokens.textMuted)
            // The faces and parts the title rows, bands, rows and the find bar take from the one button (`IconButton` with its
            // `IconButtonGlyph`): a title row's 24 pt face, a row's 22 pt face, a find option toggled on, an active filter's dot,
            // the keyboard ring round the face, and a tooltip whose key VoiceOver does not read ("Split right", hint "Command D").
            HStack(spacing: Space._4) {
                IconButton(lucide: .columns2, label: "Split right (⌘D)") {}.inTitleRow()
                IconButton(lucide: .copy, label: "Copy") {}.inRow()
                IconButton(lucide: .caseSensitive, label: "Match case") {}.inRow().toggled(true)
                IconButton(symbol: "line.3.horizontal.decrease", label: "Filter") {}.selected(true).dot(true)
                IconButton(lucide: .plus, label: "Focused") {}.focusRing(true)
                // A button that opens a menu (`IconMenuButton`): a row's "…" and a narrow chevron beside a button, which
                // turns over while its menu is open (`narrow`, `menu_chevron`).
                IconMenuButton(symbol: "ellipsis", label: "Pane menu") { Button("Close pane") {} }.inRow()
                IconMenuButton(lucide: .chevronDown, label: "Open in another app") { Button("Finder") {} }.narrow(16).chevron()
            }
            HStack(spacing: Space._4) {
                ForEach([StatusKind.idle, .working, .attention, .done, .failed], id: \.self) { StatusDot($0) }
            }
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel("Open section", isCollapsed: false)
                SectionLabel("Folded section", isCollapsed: true)
            }
            VStack(spacing: Space._1) {
                Row(isSelected: false, selection: selection, action: {}) { _ in rowLabel("A row") }
                Row(isSelected: true, selection: selection, action: {}) { _ in rowLabel("The selected row") }
            }
            Noted("← a 22 pt × in its 28 pt hit area") { CloseHitSpecimen() }
            // A path's hover card (the title row's folder, the file a viewer shows), as it stands in its popover.
            PathCard(
                path: "~/Projects/wormhole/crates/wormhole-core",
                buttons: [
                    PathCardButton(id: "open", symbol: "arrow.up.forward.app", title: "Open in Finder") {},
                    PathCardButton(id: "copy", symbol: "doc.on.doc", title: "Copy path", copied: "Copied") {},
                    PathCardButton(id: "insert", symbol: "text.insert", title: "Insert into terminal", enabled: false) {},
                ], hover: PathCardHover())
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func rowLabel(_ text: String) -> some View {
        Text(text).font(TextRole.body.font).frame(height: Metric.row).frame(maxWidth: .infinity, alignment: .leading)
    }
}
