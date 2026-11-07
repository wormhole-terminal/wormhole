// swift-tools-version: 6.2
import PackageDescription

// The Metal terminal view of the Swift UI experiment, with a synthetic frame source so the renderer is
// developed and measured without Rust. It is its own package so that it builds without the FFI;
// `UniFFIFrameSource` (Sources/WormholeUniFFI) joins only when `WormholeFFI.xcframework` has been linked
// in by `apple/scripts/build-ffi.sh apple/swift`.

let package = Package(
    name: "WormholeMetalSpike",
    platforms: [.macOS(.v26)],
    products: [
        // The executable keeps the product's name (the updater looks for `Contents/MacOS/Wormhole`).
        .executable(name: "Wormhole", targets: ["Wormhole"])
    ],
    targets: [
        // The packed frame, the `FrameSource` seam and its synthetic implementation. Foundation only.
        .target(
            name: "TerminalCore",
            swiftSettings: hotLoopSettings
        ),
        // The Metal view: CoreText, the glyph atlas, instanced quads, the display link, keyboard and IME.
        .target(
            name: "TerminalMetal",
            dependencies: ["TerminalCore"],
            swiftSettings: hotLoopSettings
        ),
        .executableTarget(
            name: "Wormhole",
            dependencies: ["TerminalCore", "TerminalMetal"],
            exclude: ["Info.plist"],
            linkerSettings: [
                // The bundle identity of a bare executable: Bundle.main reads it from this section, so the
                // isolation check (`Isolation.swift`) sees the same identifier inside and outside an app bundle.
                .unsafeFlags([
                    "-Xlinker", "-sectcreate", "-Xlinker", "__TEXT", "-Xlinker", "__info_plist",
                    "-Xlinker", Context.packageDirectory + "/Sources/Wormhole/Info.plist",
                ])
            ]
        ),
        .testTarget(name: "TerminalCoreTests", dependencies: ["TerminalCore", "TerminalMetal"]),
    ]
)

/// The frame loop is a hot loop over unsafe buffers; exclusivity checks on class properties it touches
/// cost a measurable share there (plan 3.2), and nothing in these targets relies on them.
var hotLoopSettings: [SwiftSetting] {
    [.unsafeFlags(["-enforce-exclusivity=unchecked"])]
}
