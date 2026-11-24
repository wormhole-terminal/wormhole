// The "SF Symbols" page of the gallery: every icon the app draws itself, beside the SF Symbol that could take its place and the
// motions that symbol could answer a click with, so one can be picked by its row and letter before anything is changed
// in the app. Nothing here is used by a screen.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SidebarUI
import SwiftUI

/// `window`: the window's own two toggles drawn by the module the Studio cannot name (`WindowSpecimens`), the sidebar's and the panel's.
struct SymbolsSpecimen<Window: View>: View {
    let sidebar: Window
    let panel: Window
    /// Each motion held still at 0, a half and 1, instead of played: the geometry of a mask or a strip, seen.
    @State private var frames = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            VStack(alignment: .leading, spacing: Space._2) {
                Text("Click a symbol under A or B to play its motion; a motion that turns a state over (a strip, a chevron) plays back on the next click. Symbols are drawn in the weight that matches the drawing's lines: regular unless the row says.")
                    .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                Segmented([("Play", false), ("At 0, ½ and 1", true)], selection: $frames)
            }
            ForEach(SymbolProposals.groups) { group in
                VStack(alignment: .leading, spacing: Space._1) {
                    Text(group.title).textCase(.uppercase).font(TextRole.caption.font.weight(.semibold)).foregroundStyle(tokens.textMuted)
                        .padding(.top, Space._2)
                    ForEach(group.rows) { row in
                        SymbolRowView(proposal: row, window: row.id == "toggle-sidebar" ? sidebar : panel, frames: frames)
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

/// One row: its id, what it is, today's drawing, the proposed symbol (with its variant), then A to D.
struct SymbolRowView<Window: View>: View {
    let proposal: SymbolProposal
    let window: Window
    let frames: Bool
    @Environment(\.tokens) private var tokens

    private static var letters: [String] { ["A", "B", "C", "D"] }

    var body: some View {
        HStack(spacing: Space._3) {
            Text(proposal.id).font(TextRole.caption.font.monospaced()).foregroundStyle(tokens.textPrimary)
                .frame(width: Metric.row * 4, alignment: .leading).lineLimit(1)
            VStack(alignment: .leading, spacing: 0) {
                Text(proposal.meaning).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1)
                ForEach(detail, id: \.self) { Text($0).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(2) }
            }
            .frame(width: Metric.row * 11, alignment: .leading)
            today.frame(width: Metric.row * 2, alignment: .leading)
            proposed.frame(width: Metric.row * 2, alignment: .leading)
            HStack(spacing: Space._2) {
                ForEach(Array(proposal.motions.enumerated()), id: \.offset) { index, motion in
                    VStack(spacing: 0) {
                        if frames {
                            SymbolFrames(symbol: proposal.symbol ?? "", alternate: proposal.alternate, motion: motion, size: proposal.size.points, weight: proposal.weight)
                        } else {
                            SymbolPlayer(
                                symbol: proposal.symbol ?? "", alternate: proposal.alternate, motion: motion, size: proposal.size.points,
                                weight: proposal.weight, letter: Self.letters[index])
                        }
                        Text(Self.letters[index]).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                    }
                }
            }
        }
        .frame(minHeight: Metric.row)
        .foregroundStyle(tokens.textPrimary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(proposal.id)
    }

    /// The lines under the meaning: the symbol's name and weight, what A and B are, or why the drawing stays.
    private var detail: [String] {
        guard let symbol = proposal.symbol else { return ["keeps its drawing: \(proposal.reason ?? "")"] }
        let weight = proposal.weight == .regular ? "" : " · \(proposal.weightName)"
        let names = symbol + (proposal.alternate.map { " / \($0)" } ?? "") + weight
        return [names] + proposal.motions.enumerated().map { "\(Self.letters[$0]) \($1.title)" }
    }

    @ViewBuilder private var today: some View {
        let size = proposal.size.points
        switch proposal.source {
        case .lucide(let icon): LucideView(icon: icon, size: size)
        case .glyph(let glyph): GlyphView(glyph: glyph, size: size)
        case .needsYou: NeedsYouMark(size: size)
        case .window: window
        }
    }

    @ViewBuilder private var proposed: some View {
        if let symbol = proposal.symbol {
            HStack(spacing: Space._3) {
                Image(systemName: symbol)
                if let alternate = proposal.alternate { Image(systemName: alternate) }
            }
            .font(.system(size: proposal.size.points, weight: proposal.weight))
        } else {
            Text("—").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
        }
    }
}
