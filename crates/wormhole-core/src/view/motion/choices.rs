//! What the person picks for each thing that moves (Settings → Motion): one enum per motion, all in
//! `config.json`'s `motion` object under the names they serialize to. `Today` is always what the app did
//! before the choice existed, so a person who picks it loses nothing. The app draws the variants; a
//! variant it has not built yet draws `Today`.

use serde::{Deserialize, Serialize};

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
    /// The app as it is: the row fades in where it will stand while the rows below slide aside.
    Today,
    /// The row opens its height from nothing and fades in, pushing the rows below down.
    Reveal,
    /// The row slides in from the sidebar's edge as the rows below make room.
    #[default]
    FromSide,
    /// The row drops into its place on the soft spring.
    SoftSpring,
}

choices!(InsertMotion {
    Today => "today",
    Reveal => "reveal",
    FromSide => "from-side",
    SoftSpring => "soft-spring",
});

/// How a group folds and opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FoldMotion {
    /// The app as it is: the rows leave at once and the lines below slide up to close the gap.
    Today,
    /// The rows slide shut like a drawer under the group's head, as the F-72 board draws it.
    Drawer,
    /// The rows tuck in one after another, each sliding up under the head.
    #[default]
    Tuck,
}

choices!(FoldMotion {
    Today => "today",
    Drawer => "drawer",
    Tuck => "tuck",
});

/// How a terminal's status tile shows that its status changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusMotion {
    /// The app as it is: the tile's light, edge and badge change at once.
    #[default]
    Today,
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
    Today => "today",
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
    /// The app as it is: the column slides on a timed curve and the terminal resizes once, at the end.
    #[default]
    Today,
    /// The column slides on the soft spring instead of a timed curve.
    SoftSpring,
    /// The column slides first and the terminal's text follows once it is in.
    ContentAfter,
}

choices!(ColumnMotion {
    Today => "today",
    SoftSpring => "soft-spring",
    ContentAfter => "content-after",
});

/// The motion settings: `config.json`'s `motion` object, one key per choice. A key left out is the
/// default; a value that is no option makes the whole object read as the defaults and the file is
/// reported, as for every other setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionChoices {
    /// A terminal opening or closing.
    pub insert: InsertMotion,
    /// A group folding.
    pub fold: FoldMotion,
    /// A terminal's status changing.
    pub status: StatusMotion,
    /// The sidebar or the panel hiding and showing.
    pub column: ColumnMotion,
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
            ["today", "reveal", "from-side", "soft-spring"]
        );
        assert_eq!(
            StatusMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["today", "crossfade", "badge-pop", "wash", "flare", "nudge", "line-roll"]
        );
        assert_eq!(FoldMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(), ["today", "drawer", "tuck"]);
        assert_eq!(
            ColumnMotion::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
            ["today", "soft-spring", "content-after"]
        );
    }

    #[test]
    fn the_defaults_are_the_picks_of_the_design() {
        let choices = MotionChoices::default();
        assert_eq!(choices.insert, InsertMotion::FromSide, "a terminal comes in from the side");
        assert_eq!(choices.fold, FoldMotion::Tuck, "rows tuck in when a group folds");
        assert_eq!(choices.status, StatusMotion::Today, "the status tile stays as it is");
        assert_eq!(choices.column, ColumnMotion::Today, "the columns slide as they do");
    }

    #[test]
    fn a_key_left_out_is_its_default_and_the_others_are_kept() {
        let read: MotionChoices =
            serde_json::from_str(r#"{ "fold": "today", "status": "line-roll" }"#).expect("reads");
        assert_eq!(
            read,
            MotionChoices {
                fold: FoldMotion::Today,
                status: StatusMotion::LineRoll,
                ..MotionChoices::default()
            }
        );
        assert_eq!(serde_json::from_str::<MotionChoices>("{}").expect("reads"), MotionChoices::default());
    }

    #[test]
    fn a_name_that_is_no_option_does_not_read() {
        for text in [
            r#"{ "insert": "bounce" }"#,
            r#"{ "fold": "Tuck" }"#,
            r#"{ "status": "badge_pop" }"#,
            r#"{ "column": 1 }"#,
        ] {
            assert!(serde_json::from_str::<MotionChoices>(text).is_err(), "{text}");
        }
    }

    #[test]
    fn the_object_writes_one_key_per_choice() {
        let written = serde_json::to_value(MotionChoices::default()).expect("serializes");
        assert_eq!(
            written,
            serde_json::json!({ "insert": "from-side", "fold": "tuck", "status": "today", "column": "today" })
        );
    }
}
