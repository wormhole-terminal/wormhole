//! Motion as data: which animation the person picked for each thing that moves (`choices`), the
//! spring numbers the animations are named by (`spring`) and the arithmetic of a looping preview
//! (`timeline`). No GPUI: a test pins every default, every name `config.json` spells and every preset
//! here, and the app's style layer (`style::motion`) only turns them into GPUI values.
//!
//! The split with the app: what the person *chooses*, the springs' stiffness and damping and the curves
//! (`ease`), which the design fixes (the picks of the motion plan), are here, and so is what a sidebar row
//! looks like at each moment of each motion (`pose`), so the list and the Settings previews draw the same
//! numbers. How long the other things take (hover, the panels, the spinner) stays in the app's
//! `style::motion::timing`: nothing but the drawing reads them. The motions' own durations are with
//! their poses; `pose`'s `//!` says why. Dragging a terminal has two motions the design fixed (`drag`),
//! so no choice and no pose, only their numbers.
//!
//! Rolling counts (`roll`) have no choice: the design fixed them. Like the rest of `view` this uses only
//! `theme` below it, and the sidebar's [`Urgency`](crate::view::sidebar::Urgency), which a status change
//! is between (`pose::status`): never settings, which hold a [`MotionChoices`], not the other way round.

mod choices;
pub mod drag;
mod ease;
pub mod pose;
pub mod roll;
mod spring;
mod timeline;

pub use choices::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, MotionChoices, StatusMotion};
pub use ease::{CubicBezier, EASE_IN, EASE_IN_OUT, EASE_OUT, LINEAR};
pub use spring::SpringSpec;
pub use timeline::{Span, presence};
