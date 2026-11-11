//! `format` asks for a message by its id, as the Swift app does. Its own test binary: the language in use is
//! process-wide.

use wormhole_text::{Language, Value, format, set_language};

#[test]
fn a_message_with_variables_is_formatted_by_its_id_in_the_language_in_use() {
    set_language(Language::ENGLISH);
    let args = [("folder", Value::Text("src")), ("group", Value::Text("work"))];
    assert_eq!(format("sidebar-folder-ask-title", &args).as_deref(), Some("Use “src” for “work”?"));
    assert_eq!(format("settings-section-fonts", &[]).as_deref(), Some("Fonts"));
    assert_eq!(format("no-such-message", &[]), None);

    set_language(Language::PSEUDO);
    let pseudo = format("sidebar-folder-ask-title", &args).expect("a message");
    assert!(pseudo.contains('⟦'), "{pseudo}");
    set_language(Language::ENGLISH);
}
