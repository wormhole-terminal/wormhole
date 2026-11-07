// What a selected commit changed: a one-line summary, then its files.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitDetail: View {
    let commit: Commit
    @Namespace private var space

    var body: some View {
        VStack(spacing: 0) {
            SectionHeader(title: summary) {
                ChangeCounts(added: commit.files.map(\.added).reduce(0, +), removed: commit.files.map(\.removed).reduce(0, +))
            }
            if commit.files.isEmpty {
                EmptyState(symbol: "doc", title: "No files", detail: "This commit changed no files.")
            } else {
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(commit.files) { file in
                            FileRow(file: file, space: space).padding(.horizontal, Space._1)
                        }
                    }
                }
            }
        }
        .accessibilityElement(children: .contain)
    }

    private var summary: String {
        commit.files.count == 1 ? "1 file changed" : "\(commit.files.count) files changed"
    }
}
