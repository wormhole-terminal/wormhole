// The content of whichever tab is chosen. The window owns the strip and the frame; this swaps what is inside with a short
// cross-fade so changing tabs never cuts. (The worktree sheet and alerts are the window's, so they show with the panel shut.)

import Foundation
import PanelModel
import SwiftUI

public struct PanelContentView: View {
    let controller: PanelController
    @Environment(\.panelMotion) private var motion

    public init(controller: PanelController) { self.controller = controller }

    public var body: some View {
        ZStack {
            switch controller.tab {
            case .changes: ChangesView(store: controller.changes, presentation: controller.presentation).transition(motion.content)
            case .history: HistoryView(store: controller.history, presentation: controller.presentation, now: controller.now ?? Int(Date().timeIntervalSince1970)).transition(motion.content)
            case .notes: NotesView(store: controller.notes).transition(motion.content)
            case .usage: UsageView(store: controller.usage, now: controller.now).transition(motion.content)
            case .project: ProjectView(store: controller.project, notes: controller.notes).transition(motion.content)
            case .files: FilesView(store: controller.files, presentation: controller.presentation).transition(motion.content)
            }
        }
        .animation(motion.tab, value: controller.tab)
        .environment(controller.keyboard)
        .environment(\.panelHost, controller.host)
        .environment(\.panelRoot, controller.folder)
        .environment(\.showFileHistory, controller.showFileHistory)
        .environment(\.panelCopy, { [host = controller.host] in host.copy($0) })
    }
}
