//! Motion: how long things take, how they ease and how they spring. Short and quiet: nothing
//! for what follows the pointer (hover, selection), 120–160 ms for what changes in place,
//! 250 ms for panels; exits run at about 0.8× their entrance. GPUI's animations already
//! hold still under Reduce Motion (a loop shows its first frame, a one-shot its last);
//! a component that draws something else then (the spinner's still dots) checks
//! `cx.reduce_motion()` itself.

mod easing;
mod spring;
mod timing;

pub use easing::*;
pub use spring::*;
pub use timing::*;
