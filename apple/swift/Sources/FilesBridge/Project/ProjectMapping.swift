// The Project tab's records from Rust as `PanelModel`'s values. Every line of wording that depends on core's rules
// (the hint under a switch, what trusting applies) was written in Rust and is carried through as it is.

import PanelModel
import WordsModel
import WormholeBindings
import WorkspaceModel

extension ProjectState {
    init(_ record: ProjectStateRecord) {
        self.init(
            root: record.root, status: TrustStatus(record.status),
            commands: record.commands.map { TrustLine(what: $0.what, runs: $0.runs, note: $0.note) },
            applies: record.applies, asksForTrust: record.asksForTrust,
            // A switch this build has no row for (a part a later core adds) is not drawn.
            features: record.features.compactMap { feature in
                ProjectFeature(rawValue: feature.key).map { FeatureRow($0, on: feature.on, hint: feature.hint, warns: feature.warns) }
            },
            actions: record.actions.map { ProjectAction(id: $0.id, title: $0.title, detail: $0.detail, key: $0.key) },
            prompts: record.prompts.map { ProjectPrompt(id: $0.id, title: $0.title, detail: $0.detail) },
            layout: record.layout, errors: record.errors, warnings: record.warnings)
    }
}

extension TrustStatus {
    init(_ record: TrustStatusRecord) {
        switch record {
        case .untrusted: self = .untrusted
        case .trusted: self = .trusted
        case .changed: self = .changed
        }
    }
}

extension FolderPaint {
    init(_ record: FolderPaintRecord) {
        self.init(
            chosen: record.chosen.flatMap(FolderColor.init(rawValue:)), custom: record.custom, locked: record.locked,
            note: record.note)
    }
}

extension ProjectFailure {
    /// What the notice chip says of a refused call: Rust's own words.
    var spoken: String {
        switch self {
        case let .Refused(reason): Words.text("project-refused", ["reason": .text(reason)], fallback: "Not allowed: \(reason)")
        case let .NotFound(what): Words.text("project-not-found", ["what": .text(what)], fallback: "Not found: \(what)")
        case let .Failed(message): message
        }
    }
}
