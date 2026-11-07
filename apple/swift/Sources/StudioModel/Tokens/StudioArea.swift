// The gallery's filter: which part of the app it shows, which narrows the token list to the tokens that part's
// components draw with (`studio/gallery/mod.rs` `Area`). "All" is every component.

public enum StudioArea: String, Sendable, CaseIterable, Identifiable {
    case all, sidebar, panel, chrome, settings, ambient
    public var id: String { rawValue }
}
