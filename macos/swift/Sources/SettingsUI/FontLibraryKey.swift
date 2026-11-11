// The fonts of this Settings window, shared by its three pickers: one library, so the installed families are listed
// and classified once however many pickers open. `SettingsView` makes it and puts it in the environment.

import FontPickerModel
import SwiftUI

private struct FontLibraryKey: EnvironmentKey {
    static let defaultValue: FontLibrary? = nil
}

extension EnvironmentValues {
    var fontLibrary: FontLibrary? {
        get { self[FontLibraryKey.self] }
        set { self[FontLibraryKey.self] = newValue }
    }
}
