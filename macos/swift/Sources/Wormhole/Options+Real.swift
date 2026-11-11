// Whether the window runs on the core: the FFI is linked and the sample was not asked for. Then every terminal runs
// what the core decides (the person's login shell, as the GPUI app does) unless `--shell` or WORMHOLE_SHELL names a
// program. `--sample`, `--mock` and a build without the library keep the sample backends, over the same views.

extension Options {
    var isReal: Bool { SourceFactory.hasRealSource && !sample }
}
