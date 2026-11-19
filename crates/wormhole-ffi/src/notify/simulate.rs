//! What the notification rules decide for one event, without the event: the control folder's `notify-sim` (the GPUI
//! app's `notifications::simulate`). Core's own functions (`delivery`, `interruption_level`, `sound`, `bounces`,
//! `text`) answer; nothing is posted from here, the caller posts through the notifier it has.

use wormhole_core::notify::{self, About, Delivery, Kind, Seen};

use super::{NoticeKindRecord, NoticeLevelRecord, NotifySettingsRecord};

/// The rules' answer for one event.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotifySimRecord {
    /// A system banner is delivered (else nothing is).
    pub banner: bool,
    pub level: NoticeLevelRecord,
    pub sound: bool,
    pub bounce: bool,
    pub title: String,
    pub subtitle: String,
    pub body: String,
}

/// The rules' decision for `kind` in a terminal called `name`, seen (`looking`: on screen in the key window) or not, with
/// Wormhole in front or not, under `settings`.
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn notify_simulate(
    kind: NoticeKindRecord,
    settings: NotifySettingsRecord,
    looking: bool,
    app_active: bool,
    time_sensitive: bool,
    name: String,
    who: Option<String>,
    folder: Option<String>,
    branch: Option<String>,
    message: Option<String>,
) -> NotifySimRecord {
    let kind: Kind = kind.into();
    let core = settings.to_core();
    let delivery = notify::delivery(kind, Seen { looking, app_active }, &core);
    let level = notify::interruption_level(kind, time_sensitive);
    let about = About {
        name: &name,
        who: who.as_deref(),
        folder: folder.as_deref(),
        branch: branch.as_deref(),
        message: message.as_deref(),
        ..About::default()
    };
    let text = notify::text(kind, &about);
    NotifySimRecord {
        banner: delivery == Delivery::Banner,
        level: level.into(),
        sound: notify::sound(kind, level, core.notify_sound),
        bounce: notify::bounces(kind, app_active, settings.dock_bounce),
        title: text.title,
        subtitle: text.subtitle,
        body: text.body,
    }
}
