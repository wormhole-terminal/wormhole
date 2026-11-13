// text-file: a developer tool's panel (inspect mode), English like the GPUI inspector's; never interface text
// The inspector's panel (`inspector/mod.rs` `render_panel`): the hint, Pick and Copy, then the element: its name, where
// it is built, the line of code, its box and style, and the tokens it is drawn with.

import DesignSystem
import SwiftUI

struct InspectorView: View {
    /// The panel as it opens.
    static let size = CGSize(width: 320, height: 420)
    /// The labels' column (`inspector/edit.rs` `prop`).
    private static let labelWidth: CGFloat = 96

    let inspector: Inspector
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Space._4) {
                header
                if let element = inspector.shown { details(element) } else {
                    Text("Point at anything in the window.").textRole(.caption).foregroundStyle(tokens.textMuted)
                }
            }
            .padding(Space._3)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(tokens.surfacePanel)
        .foregroundStyle(tokens.textPrimary)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Text("Inspector").textRole(.heading)
                Spacer(minLength: 0)
                if !inspector.picking { Button("Pick") { inspector.startPicking() }.controlSize(.small) }
                if inspector.shown != nil {
                    Button("Copy") { inspector.copy() }.controlSize(.small)
                        .help("Copy a note with the location and what it is drawn with")
                }
            }
            Text(inspector.picking ? "Hover to highlight, click to pin, scroll for the parent." : "Pinned. Copy the note, or pick again.")
                .textRole(.caption).foregroundStyle(tokens.textMuted)
        }
    }

    @ViewBuilder private func details(_ element: InspectedElement) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            SectionLabel("Element")
            Text(element.site.id).textRole(.body)
            Text(element.site.location).textRole(.caption).foregroundStyle(tokens.primary).textSelection(.enabled)
            if let code = InspectNote.sourceLine(of: element.site) {
                Text(code).font(codeFont.font(TextRole.caption.size))
                    .padding(.horizontal, Space._2).padding(.vertical, Space._1)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: Radius.control))
            }
        }
        VStack(alignment: .leading, spacing: Space._1) {
            SectionLabel("Box")
            row("size", String(format: "%.0f × %.0f", element.frame.width, element.frame.height))
            row("style", element.site.style.isEmpty ? "–" : element.site.style)
            row("parents", "\(inspector.chain.count - inspector.level - 1)")
        }
        let drawn = InspectNote.tokens(of: element.site.id)
        if !drawn.isEmpty {
            VStack(alignment: .leading, spacing: Space._1) {
                SectionLabel("Tokens")
                ForEach(drawn, id: \.self) { Text($0).textRole(.caption) }
            }
        }
    }

    /// One labelled value.
    private func row(_ label: String, _ value: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: Space._2) {
            Text(label).textRole(.caption).foregroundStyle(tokens.textMuted).frame(width: Self.labelWidth, alignment: .leading)
            Text(value).textRole(.caption).frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(minHeight: Metric.controlSmall)
    }
}
