// The top of the token editor: the token's name in the code font, its band, for a pair the chip that says which
// versions the value is for, and what the token is for, with how many tokens follow it.

import DesignSystem
import StudioModel
import SwiftUI

struct EditorHeader: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        EditorSection(spacing: Space._1) {
            HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                Text(token).font(TextRole.body.font.weight(.semibold).monospaced()).lineLimit(1).truncationMode(.middle)
                    .textSelection(.enabled)
                Spacer(minLength: 0)
                Text(document.groupName(of: token) ?? "").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                if document.file.isPair { ReachChip(document: document, token: token) }
            }
            Text(description).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
        }
    }

    private var description: String {
        let base = document.description(of: token) ?? StudioDescriptions.describe(token)
        let count = document.followers(of: token).count
        return count > 0 ? StudioWords.followers(base, count) : base
    }
}

/// "both" or the shown version: which appearances a value is for; a click moves it (`reach_chip`).
struct ReachChip: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let version = document.shown ?? ""
        let both = document.place(of: token) == .top
        Button { document.move(token, to: both ? .version : .top) } label: {
            Text(both ? StudioWords.reachBoth : version.lowercased()).font(TextRole.caption.font)
                .padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
        }
        .buttonStyle(.plain).foregroundStyle(tokens.textMuted)
        .help(both ? StudioWords.reachTipBoth(version) : StudioWords.reachTipThis(version))
        .accessibilityLabel(StudioWords.appearance)
        .accessibilityValue(both ? StudioWords.reachBoth : version)
    }
}
