// swift-tools-version:5.9
import PackageDescription

// The native Settings window, built by crates/wormhole/build.rs into a static library the app links.
let package = Package(
    name: "WormholeSettings",
    platforms: [.macOS(.v14)],
    products: [.library(name: "SettingsUI", type: .static, targets: ["SettingsUI"])],
    targets: [
        .target(name: "SettingsUI", path: "Sources/SettingsUI"),
        .testTarget(name: "SettingsUITests", dependencies: ["SettingsUI"])
    ]
)
