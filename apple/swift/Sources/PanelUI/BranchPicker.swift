// The branch picker's closed trigger in a card's head (`select::SelectTrigger::compact`, `PickerChip`): 24 tall,
// caption text in 500, a `border.strong` outline, the muted prefix ("vs"), the branch mark in `git.branch`
// and the branch (HEAD when none), then the up/down chevrons. A menu of branches opens from it.

import DesignSystem
import SwiftUI

struct BranchPicker: View {
    var prefix = ""
    let branch: String?
    var branches: [String] = []
    let pick: (String?) -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Menu {
            Button("HEAD") { pick(nil) }
            ForEach(branches, id: \.self) { name in Button(name) { pick(name) } }
        } label: {
            HStack(spacing: Space._1) {
                if !prefix.isEmpty { Text(prefix).foregroundStyle(tokens.textMuted) }
                if branch != nil {
                    Image(systemName: "arrow.triangle.branch").font(.system(size: 12)).foregroundStyle(tokens.gitBranch)
                }
                Text(branch ?? "HEAD").lineLimit(1).truncationMode(.tail).foregroundStyle(tokens.textPrimary)
                Image(systemName: "chevron.up.chevron.down").font(.system(size: 10, weight: .medium))
                    .foregroundStyle(tokens.textMuted)
            }
            .font(TextRole.caption.font.weight(.medium))
            .padding(.leading, Space._2).padding(.trailing, Space._1)
            .frame(height: 24)
            .frame(maxWidth: 160)
            .background(hovering ? tokens.stateHover : .clear, in: shape)
            .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
            .contentShape(shape)
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .fixedSize()
        .onHover { hovering = $0 }
        .accessibilityLabel("\(prefix) \(branch ?? "HEAD")")
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius, style: .continuous) }
}
