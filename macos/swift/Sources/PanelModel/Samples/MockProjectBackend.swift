// A `ProjectBackend` over a sample project that keeps its switches, its trust and its colour as they are changed.

import WorkspaceModel

public actor MockProjectBackend: ProjectBackend {
    var state: ProjectState?
    var paint: FolderPaint
    public private(set) var log: [String] = []

    public init(state: ProjectState? = PanelSamples.project, paint: FolderPaint = .own) {
        self.state = state
        self.paint = paint
    }

    public func project(at folder: String) async -> ProjectState? { state }
    public func folderPaint(of folder: String) async -> FolderPaint { paint }

    public func setFolderColor(_ color: FolderColor?, of folder: String) async {
        paint.chosen = color
        paint.custom = nil
        log.append("color \(color?.rawValue ?? "none")")
    }

    public func setFolderColor(hex: String, of folder: String) async {
        paint.chosen = nil
        paint.custom = hex
        log.append("color \(hex)")
    }

    public func createProject(at folder: String) async -> String? {
        state = PanelSamples.project
        log.append("create")
        return nil
    }

    public func setFeature(_ feature: ProjectFeature, on: Bool, at root: String) async {
        guard let at = state?.features.firstIndex(where: { $0.feature == feature }) else { return }
        state?.features[at].on = on
        log.append("feature \(feature.rawValue) \(on)")
    }

    public func approve(at root: String) async -> Bool {
        state?.status = .trusted
        log.append("approve")
        return true
    }

    public func revoke(at root: String) async {
        state?.status = .untrusted
        log.append("revoke")
    }

    public func run(action id: String, at root: String) async { log.append("run \(id)") }
    public func send(prompt id: String, at root: String) async { log.append("prompt \(id)") }
    public func text(ofPrompt id: String, at root: String) async -> String? { "Review my diff for \(id): look for bugs first." }
    public func openLayout(at root: String) async { log.append("layout") }
    public nonisolated func changes(at folder: String) -> AsyncStream<Void> { AsyncStream { $0.finish() } }
}

extension PanelSamples {
    public static let project = ProjectState(
        root: "/Users/wanderer/Projects/wormhole", status: .untrusted,
        commands: [
            TrustLine(what: "action test (key \u{2303}\u{2318}T): run", runs: "cargo test -p wormhole-core", note: "(current)"),
            TrustLine(what: "action build: run", runs: "scripts/check.sh --all", note: "(current)"),
            TrustLine(what: "layout terminal dev: starts", runs: "scripts/dev.sh", note: "in ."),
        ],
        applies: "its settings in config.json and actions in actions.json", asksForTrust: true,
        features: [
            FeatureRow(.settings, on: true, hint: "Needs trust before it takes effect", warns: true),
            FeatureRow(.theme, on: true, hint: "No theme.json in this project"),
            FeatureRow(.actions, on: true, hint: "Needs trust before it takes effect", warns: true),
            FeatureRow(.prompts, on: true, hint: ProjectFeature.prompts.detail),
            FeatureRow(.layout, on: false, hint: "Needs trust before it takes effect", warns: true),
            FeatureRow(.notes, on: false, hint: "Syncs once the project is trusted", warns: true),
        ],
        actions: [
            ProjectAction(id: "test", title: "Run tests", detail: "cargo test -p wormhole-core", key: "\u{2303}\u{2318}T"),
            ProjectAction(id: "build", title: "Check everything", detail: "scripts/check.sh --all"),
        ],
        prompts: [
            ProjectPrompt(id: "review", title: "Review my diff", detail: "prompts/review.md"),
            ProjectPrompt(id: "explain", title: "Explain this module"),
        ],
        layout: ["dev", "tests", "agent"], warnings: ["Action \u{201c}deploy\u{201d} wants \u{2318}T, which is taken, so it has no key."])
}
