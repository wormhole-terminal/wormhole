// What a source of rows has to be (`view::picker::Provider` and `Decorator`): the folder picker's logic (typed
// paths, the multi-level fuzzy walk, recent folders by frecency, git branch badges) is core's
// `folder_picker`, which reaches Swift over the FFI behind this protocol; the panel knows nothing else of it.

import Foundation

public protocol FolderProviding: Sendable {
    /// The rows for `query`. Asynchronous and cancellable: text that has moved on cancels the task.
    func answer(_ query: PickerQuery) async -> PickerAnswer
}

public protocol FolderDecorating: Sendable {
    /// The kinds of row it knows something about; empty means every kind.
    var kinds: [String] { get }
    /// What is known about the rows with these keys; a key it knows nothing about is left out.
    func decorate(_ keys: [String]) async -> [PickerDecoration]
}

extension FolderDecorating {
    public var kinds: [String] { [] }
}
