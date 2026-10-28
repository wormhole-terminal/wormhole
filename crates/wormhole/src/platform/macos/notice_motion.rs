//! The notice chip's motion on Core Animation: the drop on a spring, the way back up, the fade of Reduce
//! Motion. Each plays on the holder's layer, so GPUI draws nothing while it runs. The numbers are core's
//! (`view::motion::notice`); `notice` decides when each one plays.

use objc2::runtime::AnyObject;
use objc2_core_foundation::CGAffineTransform;
use objc2_foundation::{NSNumber, NSString};
use objc2_quartz_core::{
    CAAnimation, CABasicAnimation, CALayer, CAMediaTiming as _, CAMediaTimingFunction, CASpringAnimation,
};
use wormhole_core::view::motion::CubicBezier;
use wormhole_core::view::motion::notice::DROP;

/// Where the holder's layer sits when the chip is at rest and whole.
pub(super) fn rest(layer: &CALayer) {
    layer.setAffineTransform(translation(0.0));
    layer.setOpacity(1.0);
}

/// Up by `y` points (a layer's y runs up in a view that isn't flipped).
pub(super) fn translation(y: f64) -> CGAffineTransform {
    CGAffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: y }
}

/// The drop: from `from` points above its place to it, on core's spring (mass 1).
pub(super) fn drop_in(layer: &CALayer, from: f64) {
    let spring =
        CASpringAnimation::animationWithKeyPath(Some(&NSString::from_str("transform.translation.y")));
    spring.setMass(1.0);
    spring.setStiffness(f64::from(DROP.stiffness));
    spring.setDamping(f64::from(DROP.damping));
    set_range(&spring, from, 0.0);
    spring.setDuration(spring.settlingDuration());
    layer.addAnimation_forKey(&spring, Some(&NSString::from_str("notice")));
}

/// A slide along y from `from` to `to`.
pub(super) fn slide(layer: &CALayer, from: f64, to: f64, seconds: f64, curve: CubicBezier) {
    let slide = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("transform.translation.y")));
    set_range(&slide, from, to);
    timed(&slide, seconds, curve);
    layer.addAnimation_forKey(&slide, Some(&NSString::from_str("notice")));
}

/// The opacity from `from` to `to`.
pub(super) fn fade(layer: &CALayer, from: f64, to: f64, time: std::time::Duration, curve: CubicBezier) {
    let fade = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("opacity")));
    set_range(&fade, from, to);
    timed(&fade, time.as_secs_f64(), curve);
    layer.addAnimation_forKey(&fade, Some(&NSString::from_str("notice")));
}

#[allow(unsafe_code)]
fn set_range(animation: &CABasicAnimation, from: f64, to: f64) {
    let (from, to) = (NSNumber::new_f64(from), NSNumber::new_f64(to));
    let (from, to): (&AnyObject, &AnyObject) = (&from, &to);
    // SAFETY: the animated key paths are numbers, which is what the values are.
    unsafe {
        animation.setFromValue(Some(from));
        animation.setToValue(Some(to));
    }
}

fn timed(animation: &CAAnimation, seconds: f64, CubicBezier(x1, y1, x2, y2): CubicBezier) {
    animation.setDuration(seconds);
    animation.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(x1, y1, x2, y2)));
}
