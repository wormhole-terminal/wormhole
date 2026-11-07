// The panel's host in the environment, so a row's menu can copy, reveal and open without every view being handed it.

import PanelModel
import SwiftUI

extension EnvironmentValues {
    @Entry var panelHost: (any PanelHost)?
}
