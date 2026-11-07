// A titled group of files (Staged, Unstaged). Rows report taps; the owner holds the selection.

import DesignSystem
import PanelModel
import SwiftUI

struct FileListSection: View {
    let title: String
    let files: [ChangedFile]
    let selection: ChangedFile.ID?
    let space: Namespace.ID
    let select: (ChangedFile.ID) -> Void

    var body: some View {
        if !files.isEmpty {
            VStack(spacing: 0) {
                SectionHeader(title: title, count: files.count)
                ForEach(files) { file in
                    FileRow(file: file, selected: file.id == selection, space: space)
                        .padding(.horizontal, Space._1)
                        .onTapGesture { select(file.id) }
                }
            }
        }
    }
}
