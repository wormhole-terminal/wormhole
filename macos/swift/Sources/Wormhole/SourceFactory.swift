// Which `FrameSource` the app runs: the mock unless a program was asked for, which needs the FFI.

import Foundation
import TerminalCore

#if WITH_UNIFFI
    import UniFFISource
#endif

enum SourceFactory {
    static func make(_ options: Options, size: GridSize, manual: Bool = false) throws -> any FrameSource {
        if let program = options.program {
            #if WITH_UNIFFI
                return UniFFISource.UniFFIFrameSource(program: program)
            #else
                throw Options.UsageError(
                    description: "--shell needs the Rust library: run macos/scripts/build-ffi.sh macos/swift and build again") // text: a launch flag's error on stderr
            #endif
        }
        return MockFrameSource(
            .init(scenario: options.scenario, seed: options.seed, hertz: options.hertz, manual: manual), size: size)
    }

    static var hasRealSource: Bool {
        #if WITH_UNIFFI
            true
        #else
            false
        #endif
    }
}
