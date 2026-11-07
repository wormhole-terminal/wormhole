// Whether the window runs on the core: the FFI is linked and a program was asked for (`--shell`, WORMHOLE_SHELL).
// Everything else (`--mock`, a build without the library) keeps the sample backends, over the same views.

extension Options {
    var isReal: Bool { SourceFactory.hasRealSource && program != nil }
}
