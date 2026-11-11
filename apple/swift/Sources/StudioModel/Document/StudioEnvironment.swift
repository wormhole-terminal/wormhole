// What a Studio is given: Rust's resolver and token knowledge, the themes folder, the store whose windows
// preview the draft, and the contrast and look the person has. All seams, so a test brings a mock of each.

import DesignSystem
import Foundation

public struct StudioEnvironment: Sendable {
    public var resolver: any ThemeResolving
    public var folder: any ThemeFolder
    public var knowledge: any TokenKnowledge
    public var library: ThemeLibrary
    /// The core's theme file rules (formats, copies, pairs, updates); without them a draft stays in the older format.
    public var files: (any StudioFiles)?
    /// The project of the terminal in front; none where there is no project folder feature (a test).
    public var project: (any StudioProject)?
    /// Where the last export went, kept across launches so the next dialog starts there.
    public var exportFolder: ExportFolder?

    public init(
        resolver: any ThemeResolving, folder: any ThemeFolder, knowledge: any TokenKnowledge = CoreKnowledge(),
        library: ThemeLibrary = .bundled, files: (any StudioFiles)? = nil, project: (any StudioProject)? = nil,
        exportFolder: ExportFolder? = nil
    ) {
        (self.resolver, self.folder, self.knowledge, self.library) = (resolver, folder, knowledge, library)
        (self.files, self.project, self.exportFolder) = (files, project, exportFolder)
    }
}

/// The folder the last export went to: asked when the dialog opens, told when an export is done.
public struct ExportFolder: Sendable {
    public var recall: @Sendable () -> URL?
    public var remember: @Sendable (URL) -> Void

    public init(recall: @escaping @Sendable () -> URL?, remember: @escaping @Sendable (URL) -> Void) {
        (self.recall, self.remember) = (recall, remember)
    }
}
