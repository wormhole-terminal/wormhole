//! The rules every catalog must keep. Each returns the problems it found, as sentences naming the
//! file, the id and what to change, so a failing test reads as a to-do list.

use std::collections::BTreeSet;

use intl_pluralrules::{PluralCategory, PluralRuleType, PluralRules};
use unic_langid::LanguageIdentifier;

use super::catalog::{CATEGORIES, Catalog, Item, VariantKeyText};

/// The categories of a language's cardinal plural rule, found by selecting over sample numbers
/// (`intl_pluralrules` does not list them): whole numbers up to 1,000 and some decimals.
pub fn plural_categories(tag: &str) -> Result<BTreeSet<&'static str>, String> {
    let language: LanguageIdentifier = tag.parse().map_err(|e| format!("{tag}: not a language tag ({e})"))?;
    let rules = PluralRules::create(language, PluralRuleType::CARDINAL)
        .map_err(|e| format!("{tag}: intl_pluralrules has no rules for it ({e})"))?;
    let name = |category: PluralCategory| match category {
        PluralCategory::ZERO => "zero",
        PluralCategory::ONE => "one",
        PluralCategory::TWO => "two",
        PluralCategory::FEW => "few",
        PluralCategory::MANY => "many",
        PluralCategory::OTHER => "other",
    };
    let mut found = BTreeSet::new();
    for n in 0..=1000i64 {
        if let Ok(category) = rules.select(n) {
            found.insert(name(category));
        }
    }
    for decimal in ["0.5", "1.5", "2.5", "10.5", "1.0", "0.0", "1000000", "1000000.5"] {
        if let Ok(category) = rules.select(decimal) {
            found.insert(name(category));
        }
    }
    Ok(found)
}

/// What every catalog must be, English or a translation: it parses, defines nothing twice, calls no
/// function (`NUMBER()` does not localise here and `DATETIME()` does not exist: numbers arrive
/// formatted), has a default in every select and names only the plural categories its language has.
fn well_formed(catalog: &Catalog, language: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for junk in &catalog.junk {
        problems.push(format!("{language}: does not parse: {junk}"));
    }
    for id in &catalog.duplicates {
        problems.push(format!("{language}: {id} is defined more than once"));
    }
    let categories = plural_categories(language);
    for item in catalog.messages.iter().chain(&catalog.terms) {
        for function in &item.functions {
            problems.push(format!(
                "{language}: {}: calls {function}(): pass numbers and dates already formatted",
                item.id
            ));
        }
        for select in &item.selects {
            if !select.has_default {
                problems
                    .push(format!("{language}: {}: a select has no default variant (`*[other]`)", item.id));
            }
            if !select.is_plural() {
                continue;
            }
            let Ok(categories) = &categories else {
                if let Err(why) = &categories {
                    problems.push(format!("{language}: {}: {why}", item.id));
                }
                continue;
            };
            let named: BTreeSet<&str> = select
                .keys
                .iter()
                .filter_map(|key| match key {
                    VariantKeyText::Name(name) => Some(name.as_str()),
                    VariantKeyText::Number(_) => None,
                })
                .collect();
            for name in &named {
                if !CATEGORIES.contains(name) {
                    problems.push(format!("{language}: {}: [{name}] is not a plural category", item.id));
                } else if !categories.contains(name) {
                    problems.push(format!(
                        "{language}: {}: [{name}] is not a plural category of {language}",
                        item.id
                    ));
                }
            }
            for needed in categories {
                if !named.contains(needed) {
                    problems.push(format!(
                        "{language}: {}: the select lacks [{needed}], which {language} uses",
                        item.id
                    ));
                }
            }
        }
    }
    problems
}

/// A reference to a message or term that nobody defines.
fn dangling(catalog: &Catalog, also: Option<&Catalog>, language: &str) -> Vec<String> {
    let has_message =
        |id: &str| catalog.message(id).is_some() || also.is_some_and(|c| c.message(id).is_some());
    let has_term = |id: &str| catalog.has_term(id) || also.is_some_and(|c| c.has_term(id));
    let mut problems = Vec::new();
    for item in catalog.messages.iter().chain(&catalog.terms) {
        for id in item.messages.iter().filter(|id| !has_message(id)) {
            problems
                .push(format!("{language}: {}: refers to the message {id}, which does not exist", item.id));
        }
        for id in item.terms.iter().filter(|id| !has_term(id)) {
            problems.push(format!("{language}: {}: refers to the term {id}, which does not exist", item.id));
        }
    }
    problems
}

/// The rules of the source language, on top of [`well_formed`]: references resolve, and a message
/// with variables carries a comment saying what they are, for the translator.
pub fn english_problems(english: &Catalog) -> Vec<String> {
    let mut problems = well_formed(english, "en");
    problems.extend(dangling(english, None, "en"));
    for item in &english.messages {
        if !item.vars.is_empty() && !item.commented {
            problems.push(format!(
                "en: {}: has variables ({}) and no comment above it saying what they are",
                item.id,
                item.vars.join(", ")
            ));
        }
    }
    problems
}

/// The rules of a translation against English: nothing English lacks, the same variables, terms and
/// references in each message, no term of English's redefined, and the rules of [`well_formed`].
/// A message it lacks is not a problem: English's text shows.
pub fn translation_problems(english: &Catalog, language: &str, translation: &Catalog) -> Vec<String> {
    let mut problems = well_formed(translation, language);
    problems.extend(dangling(translation, Some(english), language));
    let source = english.by_id();
    for item in &translation.messages {
        let Some(original) = source.get(item.id.as_str()) else {
            problems
                .push(format!("{language}: {}: English has no such message (an orphan: delete it)", item.id));
            continue;
        };
        problems.extend(same_sets(language, original, item));
    }
    for term in &translation.terms {
        if english.has_term(&term.id) {
            problems.push(format!(
                "{language}: {}: English defines this term, and it is not translated",
                term.id
            ));
        }
    }
    problems
}

fn same_sets(language: &str, original: &Item, item: &Item) -> Vec<String> {
    let mut problems = Vec::new();
    let vars = |item: &Item| item.vars.iter().cloned().collect::<BTreeSet<_>>();
    for (what, want, got) in [
        ("variables", vars(original), vars(item)),
        ("terms", original.terms.clone(), item.terms.clone()),
        ("message references", original.messages.clone(), item.messages.clone()),
    ] {
        if want != got {
            problems.push(format!("{language}: {}: has {what} {got:?}, English has {want:?}", item.id));
        }
    }
    problems
}

/// How much of English a language has translated, as one line for `-- --nocapture`.
pub fn coverage(english: &Catalog, language: &str, translation: &Catalog) -> String {
    let have = english.messages.iter().filter(|m| translation.message(&m.id).is_some()).count();
    let total = english.messages.len();
    format!("{language}: {have}/{total} messages ({}%)", (100 * have).checked_div(total).unwrap_or(100))
}
