//! One language's catalog, loaded: the Fluent bundle and the table of every message without
//! variables, formatted once.

use std::collections::HashSet;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use fluent_syntax::ast::Entry;
use unic_langid::LanguageIdentifier;

use crate::generated::ids::{IDS, TAKES_ARGS};
use crate::pseudo;

type Bundle = FluentBundle<FluentResource>;

/// What a set language holds: the bundle (for messages with variables, formatted on demand) and the
/// table (for the rest, formatted now). Built once per language, never changed.
pub(crate) struct Loaded {
    bundle: Bundle,
    /// By message index. A message without variables is its text, in this language or, where this
    /// language has none, English's. A message with variables is empty: it has no `Key`.
    table: Box<[Box<str>]>,
    /// Whether this language itself has the message; where not, the caller formats English's.
    translated: Box<[bool]>,
    pseudo: bool,
}

impl Loaded {
    /// English: every message in `IDS` is there, or the generated files are stale.
    pub(crate) fn english(tag: &str, files: &[(&str, &str)]) -> Self {
        let resources = parse(files);
        let translated = defined(&resources);
        let loaded = Self::build(tag, resources, translated, None, false);
        debug_assert!(
            loaded.translated.iter().all(|&t| t),
            "the generated keys list a message the English files lack: run TEXT_BLESS=1 cargo test -p wormhole-text --test generated"
        );
        loaded
    }

    /// A translation. English's terms (`-brand`) are added behind its own files, so a message can
    /// use a term it does not define; a message it lacks falls back to English's text.
    pub(crate) fn translation(
        tag: &str,
        files: &[(&str, &str)],
        english_files: &[(&str, &str)],
        english: &Loaded,
    ) -> Self {
        let mut resources = parse(files);
        // Which messages the translation has itself, read before English's files are added.
        let translated = defined(&resources);
        resources.extend(parse(english_files));
        Self::build(tag, resources, translated, Some(english), false)
    }

    /// `en-XA`: English's files through the accent transform, every result padded and bracketed.
    pub(crate) fn pseudo(tag: &str, files: &[(&str, &str)], english: &Loaded) -> Self {
        let resources = parse(files);
        let translated = defined(&resources);
        Self::build(tag, resources, translated, Some(english), true)
    }

    /// The bundle of `resources` and the table: a message the language has is rendered, one it lacks
    /// takes English's text without being rendered here.
    fn build(
        tag: &str,
        resources: Vec<FluentResource>,
        translated: Box<[bool]>,
        english: Option<&Loaded>,
        pseudo: bool,
    ) -> Self {
        let bundle = bundle(tag, resources, pseudo);
        let table = IDS
            .iter()
            .enumerate()
            .map(|(i, id)| {
                if TAKES_ARGS[i] {
                    return Box::from("");
                }
                let own = if translated[i] { render(&bundle, id, None) } else { None };
                match own {
                    Some(text) if pseudo => pseudo::decorate(&text).into(),
                    Some(text) => text.into(),
                    None => english.map_or_else(|| Box::from(*id), |e| e.table[i].clone()),
                }
            })
            .collect();
        Self { bundle, table, translated, pseudo }
    }

    /// The text of the message at `index` that has no variables.
    pub(crate) fn plain(&self, index: usize) -> &str {
        self.table.get(index).map_or("", |text| text)
    }

    /// The message at `index` with `args`, or `None` where this language lacks it.
    pub(crate) fn format(&self, index: usize, args: &FluentArgs) -> Option<String> {
        if !self.translated.get(index).copied()? {
            return None;
        }
        let text = render(&self.bundle, IDS.get(index)?, Some(args))?;
        Some(if self.pseudo { pseudo::decorate(&text) } else { text })
    }
}

/// Each file parsed once. A file with a syntax error still gives its good entries; the tests
/// reject the error.
fn parse(files: &[(&str, &str)]) -> Vec<FluentResource> {
    files
        .iter()
        .map(|(_, source)| {
            FluentResource::try_new((*source).to_owned()).unwrap_or_else(|(resource, _)| resource)
        })
        .collect()
}

/// By index in `IDS`: whether the resources define the message or attribute.
fn defined(resources: &[FluentResource]) -> Box<[bool]> {
    let mut ids = HashSet::new();
    for entry in resources.iter().flat_map(FluentResource::entries) {
        if let Entry::Message(message) = entry {
            if message.value.is_some() {
                ids.insert(message.id.name.to_owned());
            }
            for attribute in &message.attributes {
                ids.insert(format!("{}.{}", message.id.name, attribute.id.name));
            }
        }
    }
    IDS.iter().map(|id| ids.contains(*id)).collect()
}

fn bundle(tag: &str, resources: Vec<FluentResource>, pseudo: bool) -> Bundle {
    let language: LanguageIdentifier = tag.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![language]);
    // Only left-to-right languages ship: the isolation marks would land in every string with
    // variables, and how GPUI draws them is unverified. They go back on with the first RTL language.
    bundle.set_use_isolating(false);
    if pseudo {
        bundle.set_transform(Some(pseudo::accent));
    }
    for resource in resources {
        // An id defined twice keeps the first: the translation's, ahead of English's.
        let _ = bundle.add_resource(resource);
    }
    bundle
}

/// `id` (`name` or `name.attribute`) formatted, or `None` where the bundle lacks it.
fn render(bundle: &Bundle, id: &str, args: Option<&FluentArgs>) -> Option<String> {
    let message;
    let pattern = match id.split_once('.') {
        None => {
            message = bundle.get_message(id)?;
            message.value()?
        }
        Some((name, attribute)) => {
            message = bundle.get_message(name)?;
            message.get_attribute(attribute)?.value()
        }
    };
    // Errors are a missing variable or reference; the tests reject both in the catalogs, and the text
    // still comes out with the hole marked, which is the least bad thing to show.
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors).into_owned();
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::files::LOCALES;
    use crate::keys::*;

    fn english() -> Loaded {
        Loaded::english("en", LOCALES[0].files)
    }

    /// Where a message is in the generated id list: the table moves as messages are added, so a test
    /// asks for it by id.
    fn index(id: &str) -> usize {
        IDS.iter().position(|known| *known == id).expect("the message exists")
    }

    /// A small German: one plain message, one with a variable, one through English's term, and the
    /// plural of a message English has two forms of.
    const GERMAN: &[(&str, &str)] = &[(
        "settings.ftl",
        "settings-section-fonts = Schriftarten\n\
         settings-open-section = { $section } öffnen\n\
         settings-agents-other = Andere\n\
         \x20   .description = Agenten, die { -brand } erkennt\n",
    )];

    #[test]
    fn a_translation_has_its_own_text_where_it_has_one_and_english_where_not() {
        let english = english();
        let german = Loaded::translation("de", GERMAN, LOCALES[0].files, &english);
        assert_eq!(german.plain(SETTINGS_SECTION_FONTS.index()), "Schriftarten");
        assert_eq!(german.plain(SETTINGS_SECTION_PROFILE.index()), "Profile");
    }

    #[test]
    fn a_translation_uses_the_term_english_defines() {
        let english = english();
        let german = Loaded::translation("de", GERMAN, LOCALES[0].files, &english);
        assert_eq!(german.plain(SETTINGS_AGENTS_OTHER_DESCRIPTION.index()), "Agenten, die Wormhole erkennt");
    }

    #[test]
    fn a_message_with_variables_formats_in_its_language_or_is_left_to_english() {
        let english = english();
        let german = Loaded::translation("de", GERMAN, LOCALES[0].files, &english);
        let mut args = FluentArgs::new();
        args.set("section", "Fonts");
        assert_eq!(german.format(index("settings-open-section"), &args).as_deref(), Some("Fonts öffnen"));
        assert_eq!(english.format(index("settings-open-section"), &args).as_deref(), Some("Open Fonts"));
        let mut count = FluentArgs::new();
        count.set("count", 5);
        // German has no font count of its own: the caller formats English's.
        assert_eq!(german.format(index("settings-font-count"), &count), None);
        assert_eq!(english.format(index("settings-font-count"), &count).as_deref(), Some("5 fonts"));
    }

    #[test]
    fn the_pseudo_language_wraps_accents_and_keeps_the_variables() {
        let english = english();
        let pseudo = Loaded::pseudo("en-XA", LOCALES[0].files, &english);
        assert_eq!(pseudo.plain(SETTINGS_SECTION_FONTS.index()), "⟦Föñtš··⟧");
        let mut args = FluentArgs::new();
        args.set("section", "Fonts");
        assert_eq!(pseudo.format(index("settings-open-section"), &args).as_deref(), Some("⟦Öpéñ Fonts····⟧"));
    }
}
