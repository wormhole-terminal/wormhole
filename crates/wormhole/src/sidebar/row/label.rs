//! What VoiceOver says of a row: its name, what it is doing, its split and its place in the list.

use wormhole_core::view::sidebar::RowState;
use wormhole_core::view::sidebar::spoken::{self, RowSpeech, SplitSpeech};

use super::super::facts::RowInfo;
use super::Place;

/// What VoiceOver says of a row: its name, what it is doing, its split and its place in the list.
pub(super) fn row_label(info: &RowInfo, selected: bool, place: &Place) -> String {
    spoken::row_label_in_split(
        &info.name,
        if info.told() {
            RowSpeech::Agent {
                state: info.state,
                detail: info.message.as_deref().filter(|_| info.state != RowState::Idle),
            }
        } else {
            RowSpeech::Plain {
                command: info.command.as_deref(),
                running: info.running(),
                bell: info.bell,
                unread: info.unread,
                failed: info.failure.as_ref().map(|failure| failure.code),
            }
        },
        spoken::Place { selected, position: place.position, of: place.of, number: place.number },
        SplitSpeech::of(place.marks),
    )
}
