// The terminal's breadcrumb, as the GPUI title row draws its default arrangement: the folder, the branch chip
// and the terminal's name with its state, each part a gap apart and joined by a faint chevron. Parents of the
// folder are muted and the last step is the text colour; the branch chip is a 20 pt wash with its own colour.
// The name is what gives way when the row is narrow: it is cut at its end, the rest keeps its width.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct Breadcrumb: View {
    let terminal: TerminalSummary
    /// The state in words ("working"), shown only while the status bar, which has them, is off.
    let showsState: Bool
    @Environment(\.chrome) private var colours

    var body: some View {
        HStack(spacing: 0) {
            if let folder = terminal.subtitle {
                FolderCrumb(path: folder)
                Chevron()
            }
            if let branch = terminal.branch {
                BranchChip(name: branch)
                Chevron()
            }
            HStack(spacing: Chrome.crumbGap) {
                StatusDot(terminal.activity.status)
                Text(terminal.title)
                    .font(ChromeText.label)
                    .foregroundStyle(colours.text)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .contentTransition(.interpolate)
            }
            .layoutPriority(-1)
            if showsState, let words = terminal.activity.stateWords {
                Text(words)
                    .font(TextRole.caption.font)
                    .foregroundStyle(terminal.activity.stateColour(colours))
                    .lineLimit(1)
                    .fixedSize()
                    .padding(.leading, Chrome.crumbGap)
                    .transition(.opacity)
            }
        }
        .accessibilityElement(children: .combine)
    }
}

private struct Chevron: View {
    @Environment(\.chrome) private var colours

    var body: some View {
        Image(systemName: "chevron.right")
            .font(.system(size: Metric.chevron, weight: .semibold))
            .foregroundStyle(colours.faint)
            .frame(width: Metric.glyph)
            .padding(.horizontal, Chrome.crumbGap)
            .accessibilityHidden(true)
    }
}

private struct FolderCrumb: View {
    let path: String
    @Environment(\.chrome) private var colours

    var body: some View {
        HStack(spacing: Chrome.crumbGap) {
            Image(systemName: "folder")
                .font(.system(size: Metric.glyph))
                .foregroundStyle(colours.muted)
            // The parents stand back; the folder itself is the text.
            (Text(parents).foregroundStyle(colours.muted) + Text(last).foregroundStyle(colours.text))
                .font(TextRole.caption.font)
                .lineLimit(1)
                .truncationMode(.head)
        }
        .layoutPriority(-2)
    }

    private var split: (String, String) {
        guard let slash = path.lastIndex(of: "/"), path.index(after: slash) < path.endIndex else { return ("", path) }
        return (String(path[...slash]), String(path[path.index(after: slash)...]))
    }
    private var parents: String { split.0 }
    private var last: String { split.1 }
}

struct BranchChip: View {
    let name: String
    @Environment(\.chrome) private var colours

    var body: some View {
        Label {
            Text(name).lineLimit(1)
        } icon: {
            Image(systemName: "arrow.triangle.branch")
        }
        .font(TextRole.caption.font.weight(.medium))
        .foregroundStyle(colours.branch)
        .padding(.leading, Space._1)
        .padding(.trailing, Space._2)
        .frame(height: Chrome.chipHeight)
        .background(colours.branchBackground, in: RoundedRectangle(cornerRadius: colours.radiusChip))
        .contentTransition(.interpolate)
        .accessibilityLabel("branch \(name)")
    }
}
