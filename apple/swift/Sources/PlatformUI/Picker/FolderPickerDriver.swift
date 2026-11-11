// Runs the picker's model (`picker/mod.rs`): each provider is asked on its own, so the folders of the current
// folder never wait for a recent folder on a share that does not answer; the list is made of the answers that are
// in, merged. Text that has moved on cancels the tasks and drops a late answer. Decorations (a branch badge, a
// chevron) are asked only for the rows in view, after the rows are shown.

import Foundation
import Observation
import PlatformModel

@MainActor @Observable
public final class FolderPickerDriver {
    /// Rows the panel shows at once (`MAX_ROWS`).
    static let maxRows = 9

    public private(set) var model: PickerModel
    public var field = PickerField(text: "")
    /// What picking a row hands over, and closing the picker.
    public var onPick: (String) -> Void = { _ in }
    public var onForget: (String) -> Void = { _ in }
    public var onClose: () -> Void = {}
    /// The panel changed what it shows (the caret moved, rows came in): a hook for placing it again.
    public var onChange: () -> Void = {}

    @ObservationIgnored private let providers: [any FolderProviding]
    @ObservationIgnored private let decorators: [any FolderDecorating]
    @ObservationIgnored private var tasks: [Task<Void, Never>] = []
    @ObservationIgnored private var answers: [Int: PickerAnswer] = [:]

    public init(context: PickerContext, providers: [any FolderProviding], decorators: [any FolderDecorating] = []) {
        model = PickerModel(context: context)
        self.providers = providers
        self.decorators = decorators
    }

    /// Asks with nothing typed: the folders below the terminal's, and the recent ones.
    public func open() { ask(model.text) }

    /// The text in the field changed (typed, pasted, cleared).
    public func typed(_ text: String) {
        field = PickerField(text: text)
        ask(text)
    }

    /// A key the picker took; returns whether it was the picker's (else the field has it).
    public func key(_ key: PickerKey) -> Bool {
        let action = PickerKeys.decide(key, field: field, facts: model.facts)
        if action == .pass { return false }
        switch model.apply(action) {
        case .nothing: break
        case .close: onClose()
        case .pick(let path): onPick(path)
        case .setText(let text): typed(text)
        case .forget(let key):
            onForget(key)
            ask(model.text)
        }
        onChange()
        return true
    }

    /// A click on a row.
    public func click(_ key: String) {
        model.select(key)
        _ = self.key(.enter)
    }

    /// A click on a step of the breadcrumb.
    public func click(crumb index: Int) {
        guard model.answer.crumbs.indices.contains(index) else { return }
        typed(model.answer.crumbs[index].text)
    }

    public func close() {
        tasks.forEach { $0.cancel() }
        tasks = []
    }

    private func ask(_ text: String) {
        close()
        answers = [:]
        let (query, generation) = model.type(text)
        for (index, provider) in providers.enumerated() {
            tasks.append(Task { [weak self] in
                let answer = await provider.answer(query)
                guard !Task.isCancelled else { return }
                self?.received(answer, from: index, generation: generation)
            })
        }
    }

    private func received(_ answer: PickerAnswer, from index: Int, generation: Int) {
        guard generation == model.generation else { return }
        answers[index] = answer
        model.receive(PickerMerge.merge(providers.indices.compactMap { answers[$0] }), generation: generation)
        decorate(generation)
        onChange()
    }

    private func decorate(_ generation: Int) {
        let rows = model.answer.rows.prefix(Self.maxRows)
        for decorator in decorators {
            let keys = rows.filter { decorator.kinds.isEmpty || decorator.kinds.contains($0.kind) }.map(\.key)
            guard !keys.isEmpty else { continue }
            tasks.append(Task { [weak self] in
                let found = await decorator.decorate(keys)
                guard !Task.isCancelled, let self, generation == self.model.generation else { return }
                self.model.decorate(found)
            })
        }
    }
}
