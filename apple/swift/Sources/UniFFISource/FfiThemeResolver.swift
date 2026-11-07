// `ThemeResolving` on `wormhole-core`'s own loader and token rules, through `wormhole-ffi`'s `resolve_theme`.
// Set it as `ThemeStore.resolver` when the FFI is linked; the answers are the dump's JSON, read by DesignSystem.

import DesignSystem
import Foundation
import WormholeBindings

public struct FfiThemeResolver: ThemeResolving {
    public init() {}

    public func resolve(_ request: ThemeRequest) throws -> Data {
        let json = try resolveTheme(
            theme: request.theme, themesDir: request.themesDirectory?.path, primary: request.primary,
            branch: request.branch, increaseContrast: request.increaseContrast,
            opacity: Float(request.look.opacity), blur: request.look.blur)
        return Data(json.utf8)
    }

    public func userThemes(in directory: URL) -> Data { Data(listUserThemes(themesDir: directory.path).utf8) }
}
