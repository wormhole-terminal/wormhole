// What a Studio is given: Rust's resolver and token knowledge, the themes folder, the store whose windows
// preview the draft, and the contrast and look the person has. All seams, so a test brings a mock of each.

import DesignSystem

public struct StudioEnvironment: Sendable {
    public var resolver: any ThemeResolving
    public var folder: any ThemeFolder
    public var knowledge: any TokenKnowledge
    public var library: ThemeLibrary

    public init(
        resolver: any ThemeResolving, folder: any ThemeFolder, knowledge: any TokenKnowledge = CoreKnowledge(),
        library: ThemeLibrary = .bundled
    ) {
        (self.resolver, self.folder, self.knowledge, self.library) = (resolver, folder, knowledge, library)
    }
}
