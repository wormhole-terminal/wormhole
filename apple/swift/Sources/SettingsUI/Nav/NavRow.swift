// A section in the nav (`style::nav_row`): 28 high, inset 8 from the nav's edges, rounded, the open one on the
// sidebar's selected tint (one shape that slides). At its right a mark: Changed's count, a Preview tag, or while
// a search runs the number of results (a section with none dims).

import DesignSystem
import SwiftUI

enum NavMark: Equatable {
    case none, count(Int), preview, hits(Int), noHits
}

struct NavRow: View {
    let id: String
    let title: String
    let mark: NavMark
    let isSelected: Bool
    let slide: Namespace.ID
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._2) {
                Image(systemName: NavIcons.symbol(id)).font(.system(size: Metric.titleGlyph)).frame(width: Metric.titleGlyph + 4)
                Text(title).font(.system(size: TextRole.body.size, weight: isSelected ? .semibold : .regular)).lineLimit(1)
                Spacer(minLength: 0)
                markView
            }
            .padding(.horizontal, Space._2)
            .frame(height: SettingsMetric.navRowHeight)
            .foregroundStyle(isSelected ? tokens.sidebarRowSelectedText : (mark == .noHits ? tokens.textFaint : tokens.textMuted))
            .background {
                if isSelected {
                    RoundedRectangle(cornerRadius: Radius.row).fill(tokens.sidebarRowSelected).matchedGeometryEffect(id: "nav-selection", in: slide)
                } else if hovering {
                    RoundedRectangle(cornerRadius: Radius.row).fill(tokens.sidebarRowHover)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(title)
        .accessibilityValue(spoken)
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    @ViewBuilder private var markView: some View {
        switch mark {
        case .count(let n) where n > 0: CountBadge(n)
        case .hits(let n): CountBadge(n)
        case .preview:
            Text(SettingsText.previewTag).font(.system(size: TextRole.caption.size, weight: .medium))
                .foregroundStyle(tokens.primary).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight + 4)
                .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(tokens.primary, lineWidth: tokens.metrics.borderWidth))
        default: EmptyView()
        }
    }

    private var spoken: String {
        switch mark {
        case .count(let n): String(localized: "settings-nav-changed-count", defaultValue: "\(n) changed")
        case .hits(let n): String(localized: "settings-nav-hits", defaultValue: "\(n) results")
        case .noHits: String(localized: "settings-nav-no-hits", defaultValue: "no results")
        case .preview: SettingsText.previewTag
        case .none: ""
        }
    }
}
