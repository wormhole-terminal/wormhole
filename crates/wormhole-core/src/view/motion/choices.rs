//! What the person picks for each thing that moves (Settings → Motion): one enum per motion, all in
//! `config.json`'s `motion` object under the names they serialize to. `Classic` is always how the app
//! moved before the choice existed (the name does not age as the other styles ship), so a person who
//! picks it loses nothing. The app draws the variants; one it has not built yet draws `Classic`.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

/// A motion's options, as the settings page lists them and `config.json` spells them.
pub trait MotionChoice: Copy + Eq + 'static {
    /// Every option, in the order the page lists them.
    const ALL: &'static [Self];

    /// The name `config.json` spells it by (kebab-case).
    fn name(self) -> &'static str;

    /// The option called `name`, if there is one.
    #[must_use]
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|choice| choice.name() == name)
    }
}

/// Implements [`MotionChoice`] for an enum, naming each variant once. A test per enum checks that
/// the names are the ones serde writes, so the two cannot drift apart.
macro_rules! choices {
    ($ty:ident { $($variant:ident => $name:literal),+ $(,)? }) => {
        impl MotionChoice for $ty {
            const ALL: &'static [Self] = &[$($ty::$variant),+];

            fn name(self) -> &'static str {
                match self {
                    $($ty::$variant => $name),+
                }
            }
        }
    };
}

/// How a terminal appears in the sidebar's list, and goes from it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InsertMotion {
    /// Classic: the row fades in where it will stand while the rows below slide aside.
    Classic,
    /// The row opens its height from nothing and fades in, pushing the rows below down.
    Reveal,
    /// The row slides in from the sidebar's edge as the rows below make room.
    #[default]
    FromSide,
    /// The row drops into its place on the soft spring.
    SoftSpring,
}

choices!(InsertMotion {
    Classic => "classic",
    Reveal => "reveal",
    FromSide => "from-side",
    SoftSpring => "soft-spring",
});

/// How a group folds and opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FoldMotion {
    /// Classic: the rows leave at once and the lines below slide up to close the gap.
    Classic,
    /// The rows slide shut like a drawer under the group's head, as the F-72 board draws it.
    Drawer,
    /// The rows tuck in one after another, each sliding up under the head.
    #[default]
    Tuck,
}

choices!(FoldMotion {
    Classic => "classic",
    Drawer => "drawer",
    Tuck => "tuck",
});

/// How a terminal's status tile shows that its status changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusMotion {
    /// Classic: the tile's light, edge and badge change at once.
    #[default]
    Classic,
    /// The old state fades out while the new one fades in.
    Crossfade,
    /// The corner badge pops in with a small overshoot.
    BadgePop,
    /// The row's card is washed with the new status colour, which fades away.
    Wash,
    /// The tile's edge flares once in the new status colour and settles.
    Flare,
    /// The tile nudges a point or two and settles.
    Nudge,
    /// The status line under the name rolls up to the new one, like a counter.
    LineRoll,
}

choices!(StatusMotion {
    Classic => "classic",
    Crossfade => "crossfade",
    BadgePop => "badge-pop",
    Wash => "wash",
    Flare => "flare",
    Nudge => "nudge",
    LineRoll => "line-roll",
});

/// How the sidebar, and the right panel, are hidden and shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColumnMotion {
    /// Classic: the column slides on a timed curve and the terminal resizes once, at the end.
    #[default]
    Classic,
    /// The column slides on the soft spring instead of a timed curve.
    SoftSpring,
    /// The column slides first and the terminal's text follows once it is in.
    ContentAfter,
}

choices!(ColumnMotion {
    Classic => "classic",
    SoftSpring => "soft-spring",
    ContentAfter => "content-after",
});

/// The motion settings: `config.json`'s `motion` object, one key per choice.
///
/// Like `feature_flags` it holds only what the person chose: a choice at its default is not written, so
/// a default that changes later reaches everyone who has not picked another (and picking the default
/// again is the same as never having picked). A key left out is the default. A value that is no option
/// reads as that choice's default and the other choices are kept; [`MotionChoices::problems`] says what
/// was wrong, for the config's notice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionChoices {
    /// A terminal opening or closing.
    #[serde(deserialize_with = "or_default", skip_serializing_if = "is_default")]
    pub insert: InsertMotion,
    /// A group folding.
    #[serde(deserialize_with = "or_default", skip_serializing_if = "is_default")]
    pub fold: FoldMotion,
    /// A terminal's status changing.
    #[serde(deserialize_with = "or_default", skip_serializing_if = "is_default")]
    pub status: StatusMotion,
    /// The sidebar or the panel hiding and showing.
    #[serde(deserialize_with = "or_default", skip_serializing_if = "is_default")]
    pub column: ColumnMotion,
}

fn is_default<T: Default + PartialEq>(choice: &T) -> bool {
    *choice == T::default()
}

/// A choice that reads as its default when its value is no option, instead of failing the whole file.
fn or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(T::deserialize(Value::deserialize(deserializer)?).unwrap_or_default())
}

impl MotionChoices {
    /// Every motion as the app made it before there was a choice.
    pub const CLASSIC: Self = Self {
        insert: InsertMotion::Classic,
        fold: FoldMotion::Classic,
        status: StatusMotion::Classic,
        column: ColumnMotion::Classic,
    };

    /// What is wrong with a `motion` object as the file has it, one sentence each: a value that is no
    /// option (it reads as the default), and a key that is no choice. Empty when it is all right.
    #[must_use]
    pub fn problems(object: &Map<String, Value>) -> Vec<String> {
        fn wrong<T: MotionChoice>(key: &str, value: &Value) -> Option<String> {
            if value.as_str().and_then(T::from_name).is_some() {
                return None;
            }
            let options: Vec<&str> = T::ALL.iter().map(|choice| choice.name()).collect();
            Some(format!("`motion.{key}` is {value}, which is not one of {}", options.join(", ")))
        }
        let mut problems = Vec::new();
        for (key, value) in object {
            let problem = match key.as_str() {
                "insert" => wrong::<InsertMotion>(key, value),
                "fold" => wrong::<FoldMotion>(key, value),
                "status" => wrong::<StatusMotion>(key, value),
                "column" => wrong::<ColumnMotion>(key, value),
                _ => Some(format!("unknown setting `motion.{key}`")),
            };
            problems.extend(problem);
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, MotionChoices, StatusMotion};

    /// Every option of `T` writes as its name, reads back from it and is listed once.
    fn names_are_the_serde_names<T>()
    where
        T: MotionChoice + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        for choice in T::ALL {
            let written = serde_json::to_string(choice).expect("serializes");
            assert_eq!(written, format!("\"{}\"", choice.name()), "{choice:?}");
            let read: T = serde_json::from_str(&written).expect("reads back");
            assert_eq!(read, *choice);
            assert_eq!(T::from_name(choice.name()), Some(*choice));
            assert_eq!(T::ALL.iter().filter(|other| other.name() == choice.name()).count(), 1, "{choice:?}");
        }
        assert_eq!(T::from_name("nope"), None);
    }

    #[test]
    fn every_option_is_spelt_the_way_config_json_spells_it() {
        names_are_the_serde_names::<InsertMotion>();
        names_are_the_serde_names::<FoldMotion>();
        names_are_the_serde_names::<StatusMotion>();
        names_are_the_serde_names::<ColumnMotion>();
        assert_eq!(
            InsertMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["classic", "reveal", "from-side", "soft-spring"]
        );
        assert_eq!(
            StatusMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["classic", "crossfade", "badge-pop", "wash", "flare", "nudge", "line-roll"]
        );
        assert_eq!(
            FoldMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["classic", "drawer", "tuck"]
        );
        assert_eq!(
            ColumnMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["classic", "soft-spring", "content-after"]
        );
    }

    #[test]
    fn the_defaults_are_the_picks_of_the_design() {
        let choices = MotionChoices::default();
        assert_eq!(choices.insert, InsertMotion::FromSide, "a terminal comes in from the side");
        assert_eq!(choices.fold, FoldMotion::Tuck, "rows tuck in when a group folds");
        assert_eq!(choices.status, StatusMotion::Classic, "the status tile stays as it is");
        assert_eq!(choices.column, ColumnMotion::Classic, "the columns slide as they do");
    }

    #[test]
    fn a_key_left_out_is_its_default_and_the_others_are_kept() {
        let read: MotionChoices =
            serde_json::from_str(r#"{ "fold": "classic", "status": "line-roll" }"#).expect("reads");
        assert_eq!(
            read,
            MotionChoices {
                fold: FoldMotion::Classic,
                status: StatusMotion::LineRoll,
                ..MotionChoices::default()
            }
        );
        assert_eq!(serde_json::from_str::<MotionChoices>("{}").expect("reads"), MotionChoices::default());
    }

    #[test]
    fn a_value_that_is_no_option_is_that_choices_default_and_the_others_are_kept() {
        let read: MotionChoices = serde_json::from_str(
            r#"{ "insert": "bounce", "fold": "classic", "status": "badge_pop", "column": 1 }"#,
        )
        .expect("reads");
        assert_eq!(
            read,
            MotionChoices { fold: FoldMotion::Classic, ..MotionChoices::default() },
            "insert, status and column are their defaults, fold is what it said"
        );
    }

    #[test]
    fn what_is_wrong_in_the_object_is_said_choice_by_choice() {
        let object = |text: &str| match serde_json::from_str(text).expect("json") {
            serde_json::Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        };
        assert!(
            MotionChoices::problems(&object(r#"{ "fold": "tuck", "column": "content-after" }"#)).is_empty()
        );
        let problems = MotionChoices::problems(&object(r#"{ "insert": "bounce", "fold": 3, "toast": "x" }"#));
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(
            problems[0].contains("`motion.insert`") && problems[0].contains("\"bounce\""),
            "{problems:?}"
        );
        assert!(problems[0].contains("from-side"), "it lists the options: {problems:?}");
        assert!(problems[1].contains("`motion.fold`") && problems[1].contains('3'), "{problems:?}");
        assert!(problems[2].contains("`motion.toast`"), "{problems:?}");
    }

    #[test]
    fn classic_is_the_first_option_of_every_motion() {
        assert_eq!(
            MotionChoices::CLASSIC,
            MotionChoices {
                insert: InsertMotion::ALL[0],
                fold: FoldMotion::ALL[0],
                status: StatusMotion::ALL[0],
                column: ColumnMotion::ALL[0],
            }
        );
    }

    #[test]
    fn only_what_the_person_chose_is_written() {
        let nothing = serde_json::to_value(MotionChoices::default()).expect("serializes");
        assert_eq!(nothing, serde_json::json!({}), "the defaults stay out of the file");
        let chosen = MotionChoices {
            fold: FoldMotion::Drawer,
            status: StatusMotion::Wash,
            ..MotionChoices::default()
        };
        assert_eq!(
            serde_json::to_value(chosen).expect("serializes"),
            serde_json::json!({ "fold": "drawer", "status": "wash" })
        );
        // And what was written reads back as it was.
        let text = serde_json::to_string(&chosen).expect("serializes");
        assert_eq!(serde_json::from_str::<MotionChoices>(&text).expect("reads"), chosen);
    }
}
