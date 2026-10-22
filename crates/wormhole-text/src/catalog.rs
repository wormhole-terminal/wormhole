//! One language's catalog, loaded: the Fluent bundle and the table of every message without
//! variables, formatted once.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
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
        let bundle = bundle(tag, files, None, false);
        let loaded = Self::from_bundle(bundle, None, false);
        debug_assert!(
            loaded.translated.iter().all(|&t| t),
            "keys.rs lists a message the English files lack: run TEXT_BLESS=1 cargo test -p wormhole-text --test generated"
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
        // Which messages the translation has itself: asked of a bundle without English's added.
        let own = bundle(tag, files, None, false);
        let translated: Box<[bool]> = IDS.iter().map(|id| has(&own, id)).collect();
        let full = bundle(tag, files, Some(english_files), false);
        let mut loaded = Self::from_bundle(full, Some(english), false);
        loaded.translated = translated;
        loaded.fill_gaps(english);
        loaded
    }

    /// `en-XA`: English's files through the accent transform, every result padded and bracketed.
    pub(crate) fn pseudo(tag: &str, files: &[(&str, &str)], english: &Loaded) -> Self {
        Self::from_bundle(bundle(tag, files, None, true), Some(english), true)
    }

    fn from_bundle(bundle: Bundle, english: Option<&Loaded>, pseudo: bool) -> Self {
        let table = IDS
            .iter()
            .enumerate()
            .map(|(i, id)| {
                if TAKES_ARGS[i] {
                    return Box::from("");
                }
                match render(&bundle, id, None) {
                    Some(text) if pseudo => pseudo::decorate(&text).into(),
                    Some(text) => text.into(),
                    None => english.map_or_else(|| Box::from(*id), |e| e.table[i].clone()),
                }
            })
            .collect();
        let translated = IDS.iter().map(|id| has(&bundle, id)).collect();
        Self { bundle, table, translated, pseudo }
    }

    /// A translation's table holds English where the language has no message.
    fn fill_gaps(&mut self, english: &Loaded) {
        for (i, own) in self.translated.iter().enumerate() {
            if !own {
                self.table[i] = english.table[i].clone();
            }
        }
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

fn bundle(tag: &str, files: &[(&str, &str)], behind: Option<&[(&str, &str)]>, pseudo: bool) -> Bundle {
    let language: LanguageIdentifier = tag.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![language]);
    // Only left-to-right languages ship: the isolation marks would land in every string with
    // variables, and how GPUI draws them is unverified. They go back on with the first RTL language.
    bundle.set_use_isolating(false);
    if pseudo {
        bundle.set_transform(Some(pseudo::accent));
    }
    for (_, source) in files.iter().chain(behind.into_iter().flatten()) {
        // A file with a syntax error still gives its good entries; the tests reject the error.
        let resource = FluentResource::try_new((*source).to_owned()).unwrap_or_else(|(resource, _)| resource);
        // An id defined twice keeps the first: the translation's, ahead of English's.
        let _ = bundle.add_resource(resource);
    }
    bundle
}

/// Whether the bundle has the message or attribute `id` (`name` or `name.attribute`).
fn has(bundle: &Bundle, id: &str) -> bool {
    match id.split_once('.') {
        None => bundle.get_message(id).is_some_and(|m| m.value().is_some()),
        Some((name, attribute)) => {
            bundle.get_message(name).is_some_and(|m| m.get_attribute(attribute).is_some())
        }
    }
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

    /// A small German: one plain message, one with a variable, one through English's term, and the
    /// plural of a message English has two forms of.
    const GERMAN: &[(&str, &str)] = &[(
        "settings.ftl",
        "settings-section-fonts = Schriftarten\n\
         settings-open-section = { $section } öffnen\n\
         settings-about-defaults = Standardwerte von { -brand }\n",
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
        assert_eq!(german.plain(SETTINGS_ABOUT_DEFAULTS.index()), "Standardwerte von Wormhole");
    }

    #[test]
    fn a_message_with_variables_formats_in_its_language_or_is_left_to_english() {
        let english = english();
        let german = Loaded::translation("de", GERMAN, LOCALES[0].files, &english);
        let mut args = FluentArgs::new();
        args.set("section", "Fonts");
        assert_eq!(german.format(14, &args).as_deref(), Some("Fonts öffnen"));
        assert_eq!(english.format(14, &args).as_deref(), Some("Open Fonts"));
        let mut count = FluentArgs::new();
        count.set("count", 5);
        // German has no font count of its own: the caller formats English's.
        assert_eq!(german.format(15, &count), None);
        assert_eq!(english.format(15, &count).as_deref(), Some("5 fonts"));
    }

    #[test]
    fn the_pseudo_language_wraps_accents_and_keeps_the_variables() {
        let english = english();
        let pseudo = Loaded::pseudo("en-XA", LOCALES[0].files, &english);
        assert_eq!(pseudo.plain(SETTINGS_SECTION_FONTS.index()), "⟦Föñtš··⟧");
        let mut args = FluentArgs::new();
        args.set("section", "Fonts");
        assert_eq!(pseudo.format(14, &args).as_deref(), Some("⟦Öpéñ Fonts····⟧"));
    }
}
