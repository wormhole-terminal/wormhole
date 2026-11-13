// What the intro window is told about the app around it, and what it asks back: values and a protocol, so the
// window runs on samples now and on the Rust core (agents, trust, folder picks, settings) later.

import Foundation

/// An agent on PATH, as the third step lists it.
public struct FoundAgent: Equatable, Sendable, Identifiable {
    public enum Hooks: Equatable, Sendable { case notInstalled, partial, installed, unsupported }

    public var id: String
    public var name: String
    public var hooks: Hooks
    /// The file the hooks go in, for the row (`~/.claude/settings.json`).
    public var path: String?

    public init(id: String, name: String, hooks: Hooks, path: String? = nil) {
        (self.id, self.name, self.hooks, self.path) = (id, name, hooks, path)
    }
}

/// The project folder's trust card (`TrustCard`): what trusting it would let run.
public struct TrustCard: Equatable, Sendable {
    public var name: String
    public var directory: String
    public var commands: Int
    public var approving: Bool

    public init(name: String, directory: String, commands: Int, approving: Bool = false) {
        (self.name, self.directory, self.commands, self.approving) = (name, directory, commands, approving)
    }
}

/// What the intro window needs from the app. Every method is a request: nothing is installed, trusted or written
/// until the person presses the button that asks for it.
@MainActor
public protocol OnboardingBackend: AnyObject {
    /// The settings the Look and Agents steps change as the person goes.
    var preferences: OnboardingPreferences { get }
    /// The trust card of the project the intro is about, if it has one to show.
    var trustCard: TrustCard? { get }
    /// The trust card for the project the intro is about now: the folder picked in the first step, else the front terminal's
    /// project (`onboarding/mod.rs` `trust_root`). A backend with one fixed card answers it.
    func trustCard(for folder: URL?) -> TrustCard?
    /// Whether Notes is on in the window's profile: the primer's words and its "Jot a note" line follow it.
    var notesOn: Bool { get }
    /// What the person's keys are, for the words that name them ("⌘,"): Settings, the palette.
    var settingsKeys: String { get }
    /// Agents on PATH and the ones recognised but not found (their count), scanned off the main thread.
    func scanAgents() async -> (found: [FoundAgent], others: Int)
    func installHooks(agent: String) async throws -> FoundAgent
    func trust(_ folder: URL)
    /// The folders the first step offers.
    func places() -> [Place]
    /// Shows the open panel and answers the folder chosen.
    func chooseFolder() async -> URL?
    /// Finishing: the first run is remembered and the first terminal moves to `folder`; a replay changes nothing.
    func finish(folder: URL?, firstRun: Bool)
    /// "How much to show": the walk over what the window displays, from the settings and the window's panel as they are.
    func tailor() -> (any TailorWalking)?
    /// Whether the right panel of the window behind is to be open (an answer of the walk), and whether it is now.
    func panelOpen(_ open: Bool)
    /// The newest terminal on this Mac with settings to bring over, found off the main thread.
    func importOffer() async -> ImportChoice?
    /// Brings over what that terminal's settings tick by default; the sentence that says what came over.
    func useImport(_ choice: ImportChoice) async throws -> String
    /// Shows the open panel for a theme file and imports it as the theme; whether one was.
    func importThemeFile() async -> Bool
}

extension OnboardingBackend {
    public var notesOn: Bool { true }
    public func trustCard(for folder: URL?) -> TrustCard? { trustCard }
    /// A backend without the core has no walk.
    public func tailor() -> (any TailorWalking)? { nil }
    /// Nothing behind to follow the walk.
    public func panelOpen(_ open: Bool) {}
    /// No other terminal is looked for.
    public func importOffer() async -> ImportChoice? { nil }
    /// Nothing to bring over.
    public func useImport(_ choice: ImportChoice) async throws -> String { "" }
    /// No file is imported.
    public func importThemeFile() async -> Bool { false }
}
