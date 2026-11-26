// The shader's source: the GPUI app's own files (`Resources/shader` is a copy of
// `crates/wormhole/src/platform/macos/intro/shader`, made by `macos/scripts/build-ffi.sh`), joined in the order each needs the one
// before it (a library built from source has no `#include`), behind the moments the model owns (`ShaderConstants`).

import AuxModel
import Foundation

enum IntroShader {
    /// One file per style on top of the shared pieces.
    static let files = [
        "common", "emerge", "vortex", "collapse", "warp", "ftl", "wormhole", "aurora", "tentacles", "voyage", "glow",
        "entry",
    ]

    enum LoadError: Error { case missing(String) }

    /// Where `name`'s source is in this module's resource bundle (the app's own `Contents/Resources` in a shipped build), if it is.
    static func url(of name: String) -> URL? {
        Bundle.module.url(forResource: name, withExtension: "metal", subdirectory: "shader")
    }

    static func source() throws -> String {
        try files.reduce(ShaderConstants.source) { source, name in
            guard let url = url(of: name) else { throw LoadError.missing(name) }
            return source + "\n" + (try String(contentsOf: url, encoding: .utf8))
        }
    }
}
