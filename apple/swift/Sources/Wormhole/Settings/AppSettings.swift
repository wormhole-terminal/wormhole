// The settings model the window and the Settings window share: the core's (`config.json`, watched) where the window is
// real, the sample's in memory everywhere else. Made once at launch, because the window follows it (`SettingsApplier`)
// whether or not the Settings window is open.

import Foundation
import SettingsModel

#if WITH_UNIFFI
    import SettingsBridge
#endif

@MainActor
enum AppSettings {
    static func makeModel(app: AppController) -> SettingsModel {
        let model = makeBare(app: app)
        model.onRecording = { [weak app] in app?.hotkey?.pause($0) }
        model.openStudio = { [weak app] in app?.studio.show() }
        model.hotkeyError = { [weak app] in app?.hotkey?.error }
        return model
    }

    private static func makeBare(app: AppController) -> SettingsModel {
        #if WITH_UNIFFI
            if app.options.isReal {
                do {
                    return try SettingsBridge.makeModel(platform: SettingsPlatform.make(app: app))
                } catch {
                    FileHandle.standardError.write(Data("settings: the core's are not available (\(error)); using the sample\n".utf8)) // text: a line on stderr for whoever launched the app
                }
            }
        #endif
        return SettingsModel(backend: MockSettingsBackend(), host: MockSettingsHost())
    }
}
