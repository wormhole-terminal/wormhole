// swift-tools-version: 6.2
import PackageDescription

// A dummy consumer of the Rust seam, for measuring it without a UI (`swift build -c release` here,
// after `../scripts/build-ffi.sh`, which writes `../dist/`). The real app is `../swift/`.
let package = Package(
    name: "FFIBench",
    platforms: [.macOS(.v26)],
    products: [.executable(name: "ffi-bench", targets: ["FFIBench"])],
    targets: [
        .binaryTarget(name: "WormholeFFI", path: "WormholeFFI.xcframework"),
        // The generated bindings, Swift 5 mode (the generated code predates Swift 6's strictness).
        .target(
            name: "WormholeBindings",
            dependencies: ["WormholeFFI"],
            swiftSettings: [.swiftLanguageMode(.v5)],
            linkerSettings: [
                // What the Rust crates behind the library link: Security (the Keychain), the run loop
                // and FSEvents (folder watching), the system configuration.
                .linkedFramework("Security"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("CoreServices"),
                .linkedFramework("SystemConfiguration"),
                .linkedLibrary("iconv"),
            ]
        ),
        // Swift 5 mode: a one-file measuring tool with top-level state, not the app's code.
        .executableTarget(
            name: "FFIBench", dependencies: ["WormholeBindings"],
            swiftSettings: [.swiftLanguageMode(.v5)]),
    ]
)
