// The Feature flags page (`sections/feature_flags.rs`): a switch for each in-progress feature of this build. A flag is
// changed when it differs from what the build does with no entry, and its reset arrow takes the entry out. The page and
// its nav group show only in a development build, on the Unstable channel, or after ⌃⌥⌘F.

import DesignSystem
import SettingsModel
import SwiftUI

struct FlagList: View {
    let model: SettingsModel

    var body: some View {
        // `values` is read so a flag changed from outside (the file) redraws.
        let _ = model.values
        ForEach(model.host.flags()) { flag in
            let changed = flag.on != flag.defaultOn
            SettingRow(
                label: flag.title, description: flag.description, visibleHint: false, state: RowState(isChanged: changed),
                frameId: "flag.\(flag.key)", onReset: { model.host.setFlag(flag.key, on: flag.defaultOn) }
            ) {
                SettingToggle(label: flag.title, isOn: flag.on) { model.host.setFlag(flag.key, on: $0) }
            }
            .id(flag.key)
        }
    }
}
