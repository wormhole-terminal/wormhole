//! Motion as data: which animation the person picked for each thing that moves (`choices`), the
//! spring numbers the animations are named by (`spring`) and the arithmetic of a looping preview
//! (`timeline`). No GPUI: a test pins every default, every name `config.json` spells and every preset
//! here, and the app's style layer (`style::motion`) only turns them into GPUI values.
//!
//! The split with the app: what the person *chooses* and the springs' stiffness and damping, which
//! the design fixes (the picks of the motion plan), are here. How long a thing takes and the curve it
//! eases along stay in the app's `style::motion::{timing, easing}`: they are tuned by eye against
//! GPUI's frame clock, nothing but the drawing reads them, and a test there already pins the curves.
//!
//! Like the rest of `view` it uses only `theme` below it: never settings, which hold a
//! [`MotionChoices`], not the other way round.

mod choices;
mod spring;
mod timeline;

pub use choices::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, MotionChoices, StatusMotion};
pub use spring::SpringSpec;
pub use timeline::{Span, presence};
