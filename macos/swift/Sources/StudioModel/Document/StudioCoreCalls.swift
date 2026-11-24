// How often a draft asked the core, counted where each call is made: the control folder's `studio-perf` prints what one
// step of a person's work asked, and a test can hold a step to its number. Plain counters; nothing reads them to decide.

/// How often a draft asked the core for each of its answers.
public struct StudioCoreCalls: Sendable, Equatable {
    /// `resolve_theme` for the draft (each one writes the draft file into the scratch folder first).
    public var resolves = 0
    /// The contrast report of the whole draft.
    public var contrasts = 0
    /// Whether the theme a copy came from has changed.
    public var updateChecks = 0
    /// The completion list for the text in the value field.
    public var completions = 0

    public init() {}

    /// What was asked between two readings.
    public static func - (after: StudioCoreCalls, before: StudioCoreCalls) -> StudioCoreCalls {
        var calls = StudioCoreCalls()
        calls.resolves = after.resolves - before.resolves
        calls.contrasts = after.contrasts - before.contrasts
        calls.updateChecks = after.updateChecks - before.updateChecks
        calls.completions = after.completions - before.completions
        return calls
    }
}
