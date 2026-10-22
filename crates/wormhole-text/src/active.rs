//! The language in use, the loaded catalogs, and the two ways to ask for a message.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use fluent_bundle::FluentArgs;

use crate::args::Arg;
use crate::catalog::Loaded;
use crate::generated::files::LOCALES;
use crate::key::Key;
use crate::language::Language;

/// One slot per language, filled when the language is first set (or, for English, asked for).
static LOADED: [OnceLock<Loaded>; Language::COUNT] = [const { OnceLock::new() }; Language::COUNT];
/// The language every thread reads without an override.
static ACTIVE: AtomicU8 = AtomicU8::new(0);

#[cfg(feature = "test-support")]
thread_local! {
    /// A test's own language, so parallel tests cannot flip each other. A `const` cell without a
    /// destructor: reading it allocates nothing.
    static OVERRIDE: std::cell::Cell<Option<Language>> = const { std::cell::Cell::new(None) };
}

/// The catalog of `language`, loaded now if it is not yet. Loading formats every message once
/// (about 2 ms for 700), which is why a language is loaded when it is *set* and never from a frame.
fn load(language: Language) -> &'static Loaded {
    LOADED[language.index()].get_or_init(|| {
        let english = || load(Language::ENGLISH);
        match language.locale() {
            // English itself: the source, whose files the other languages are checked against.
            Some(locale) if language == Language::ENGLISH => Loaded::english(locale.tag, locale.files),
            Some(locale) => {
                let source = &LOCALES[Language::ENGLISH.index()];
                Loaded::translation(locale.tag, locale.files, source.files, english())
            }
            None => {
                let source = &LOCALES[Language::ENGLISH.index()];
                Loaded::pseudo(language.tag(), source.files, english())
            }
        }
    })
}

fn current_language() -> Language {
    #[cfg(feature = "test-support")]
    if let Some(language) = OVERRIDE.with(std::cell::Cell::get) {
        return language;
    }
    Language::from_index(ACTIVE.load(Ordering::Relaxed))
}

/// What the language in use holds. `get()` only: a language that is not loaded (it always is, once
/// set) shows English, and English loads here the first time it is needed, which is at startup.
fn current() -> &'static Loaded {
    LOADED[current_language().index()].get().unwrap_or_else(|| load(Language::ENGLISH))
}

/// Makes `language` the one the interface speaks, loading it first so the next frame finds it
/// ready. Call it from where the setting changes (startup, the Settings window), then refresh the
/// windows.
pub fn set_language(language: Language) {
    load(language);
    ACTIVE.store(language.index() as u8, Ordering::Relaxed);
}

/// The language in use on this thread: the global one, or a test's `with_language`.
pub fn language() -> Language {
    current_language()
}

/// Whether `language`'s catalog is loaded, which [`set_language`] guarantees.
pub fn is_loaded(language: Language) -> bool {
    LOADED[language.index()].get().is_some()
}

/// The text of a message without variables in the language in use: an array index after one
/// relaxed atomic read, no allocation and no lock. A language that lacks the message gets
/// English's, filled in when the language was loaded.
pub fn text(key: Key) -> &'static str {
    current().plain(key.index())
}

/// Runs `body` with `language` as this thread's language, whatever the global one is. For tests,
/// which share a process: nobody else sees the change.
#[cfg(feature = "test-support")]
pub fn with_language<R>(language: Language, body: impl FnOnce() -> R) -> R {
    struct Restore(Option<Language>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|cell| cell.set(self.0));
        }
    }
    load(language);
    let _restore = Restore(OVERRIDE.with(|cell| cell.replace(Some(language))));
    body()
}

/// The message at `index` with `args`, in the language in use or, where it lacks the message,
/// English. What the generated functions call; one `String` and a few small ones for the arguments.
pub(crate) fn message(index: usize, args: &[(&'static str, Arg<'_>)]) -> String {
    let mut named = FluentArgs::with_capacity(args.len());
    for (name, value) in args {
        match value {
            Arg::Str(text) => named.set(*name, *text),
            Arg::Int(number) => named.set(*name, *number),
        }
    }
    let language = current();
    language
        .format(index, &named)
        .or_else(|| load(Language::ENGLISH).format(index, &named))
        .unwrap_or_default()
}
