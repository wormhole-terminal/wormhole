//! The system's Developer Tools list: an app on it starts programs that skip the first-run malware
//! check, which every new build, test binary and agent-made tool otherwise waits on. There is no
//! public way to read the list, so all the app can do is open the place where the person adds it.

use gpui_kit::*;

use super::os::dev_tools as os;

/// Open the system's settings where an app is added to the Developer Tools list; nothing where
/// there is no such list.
pub fn open_settings(cx: &App) {
    os::open_settings(cx);
}
