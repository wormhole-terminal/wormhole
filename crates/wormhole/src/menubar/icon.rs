//! The status item's glyph, drawn rather than taken from SF Symbols: the terminal
//! outline (filled while something works), with the needs-you dot, the failed dot or the finished
//! check as a badge on its top-right corner behind a cut-out ring, and the four frames of the optional working
//! animation, in which the prompt's line types itself out.
//!
//! Every state uses the same canvas with the box centred in it, so the glyph never shifts, resizes
//! or jumps as the status changes — only the badge comes and goes in the reserved top-right. The
//! canvas is no taller than the glyph, so the menu bar never scales it down, and the glyph sits at
//! the same size and height as its neighbours (a system terminal, the Control Center extras). The
//! weight — a 1.9 stroke, a 2.6 corner — matches those stroked SF Symbols.
//!
//! Without a mark the image is a template, so AppKit tints it for a light or dark menu bar and
//! the highlighted state. With one it can't be (the mark keeps its colour); the outline is then
//! drawn in `labelColor`, which resolves against the menu bar's appearance as it draws.

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_app_kit::{
    NSBezierPath, NSColor, NSCompositingOperation, NSGraphicsContext, NSImage, NSLineCapStyle,
    NSLineJoinStyle,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::model::Mark;
use super::{Rgba, StatusColors};

/// The canvas, the same in every state: the box is centred in it and the badge lives in the
/// reserved room on the right, so nothing moves as the status changes. Its height is the glyph's,
/// so the bar never scales it down.
const CANVAS: (f64, f64) = (21.0, 16.0);
/// The terminal box: a rounded rect, centred in the canvas (identical in every state).
const BOX: NSRect = NSRect::new(NSPoint::new(3.75, 2.5), NSSize::new(13.5, 11.0));
const RADIUS: f64 = 2.6;
const STROKE: f64 = 1.9;
/// The cut-out ring that sets a badge off from whatever is under it.
const RING: f64 = 1.3;
/// The needs-you and the failed dot.
const DOT: f64 = 6.0;
/// The finished check, drawn in its own stroke.
const CHECK: f64 = 2.6;

/// The image for one state of the status item. `filled`: something works; `frame`: the working
/// animation's frame (`None`: the full prompt, still).
pub(super) fn image(
    filled: bool,
    frame: Option<usize>,
    mark: Mark,
    colors: StatusColors,
    description: &str,
) -> Retained<NSImage> {
    let marked = mark != Mark::None;
    let size = NSSize::new(CANVAS.0, CANVAS.1);
    let handler = RcBlock::new(move |_: NSRect| -> Bool {
        let ink = if marked { NSColor::labelColor() } else { NSColor::blackColor() };
        glyph(filled, frame, &ink);
        match mark {
            Mark::NeedsYou => dot(colors.attention),
            Mark::Failed => dot(colors.danger),
            Mark::Finished => check(colors.done),
            Mark::None => {}
        }
        Bool::YES
    });
    // Flipped, so the coordinates read like an SVG's (y down).
    let image = NSImage::imageWithSize_flipped_drawingHandler(size, true, &handler);
    image.setTemplate(!marked);
    image.setAccessibilityDescription(Some(&NSString::from_str(description)));
    image
}

fn color(rgba: Rgba) -> Retained<NSColor> {
    let [r, g, b, a] = rgba.map(f64::from);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a)
}

fn compositing(operation: NSCompositingOperation) {
    if let Some(context) = NSGraphicsContext::currentContext() {
        context.setCompositingOperation(operation);
    }
}

fn line(points: &[(f64, f64)], width: f64) -> Retained<NSBezierPath> {
    let path = NSBezierPath::bezierPath();
    for (ix, &(x, y)) in points.iter().enumerate() {
        let point = NSPoint::new(x, y);
        if ix == 0 { path.moveToPoint(point) } else { path.lineToPoint(point) }
    }
    path.setLineWidth(width);
    path.setLineCapStyle(NSLineCapStyle::Round);
    path.setLineJoinStyle(NSLineJoinStyle::Round);
    path
}

/// The terminal: a rounded box with a `>` prompt and its line, the prompt centred
/// in the box. While working the box is filled and the prompt cut out of it; an animation frame
/// draws the line 0–3 long.
fn glyph(filled: bool, frame: Option<usize>, ink: &NSColor) {
    ink.set();
    let body = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(BOX, RADIUS, RADIUS);
    // The prompt sits on the box's vertical centre, inset from its left edge.
    let left = BOX.origin.x;
    let mid = BOX.origin.y + BOX.size.height / 2.0;
    let chevron = line(&[(left + 3.15, mid - 2.4), (left + 5.85, mid), (left + 3.15, mid + 2.4)], STROKE);
    let length = frame.map_or(3.0, |frame| frame as f64);
    let prompt =
        (length > 0.0).then(|| line(&[(left + 7.15, mid + 2.4), (left + 7.15 + length, mid + 2.4)], STROKE));
    if filled {
        body.fill();
        compositing(NSCompositingOperation::DestinationOut);
    } else {
        body.setLineWidth(STROKE);
        body.stroke();
    }
    chevron.stroke();
    if let Some(prompt) = prompt {
        prompt.stroke();
    }
    compositing(NSCompositingOperation::SourceOver);
}

/// The amber dot (needs you) or the danger one (failed), a badge tucked onto the box's top-right corner inside a ring cut out of what is
/// under it.
fn dot(rgba: Rgba) {
    let center = (BOX.origin.x + BOX.size.width - 0.1, DOT / 2.0 + RING);
    let circle = |r: f64| {
        NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
            NSPoint::new(center.0 - r, center.1 - r),
            NSSize::new(2.0 * r, 2.0 * r),
        ))
    };
    compositing(NSCompositingOperation::DestinationOut);
    circle(DOT / 2.0 + RING).fill();
    compositing(NSCompositingOperation::SourceOver);
    color(rgba).set();
    circle(DOT / 2.0).fill();
}

/// The green check, a badge on the box's top-right corner, outlined by a cut-out ring.
fn check(rgba: Rgba) {
    let center = (BOX.origin.x + BOX.size.width - 0.3, 4.4);
    let points: Vec<(f64, f64)> =
        [(-2.8, 0.2), (-0.9, 2.1), (2.7, -2.3)].iter().map(|(x, y)| (center.0 + x, center.1 + y)).collect();
    compositing(NSCompositingOperation::DestinationOut);
    line(&points, CHECK + 2.0 * RING).stroke();
    compositing(NSCompositingOperation::SourceOver);
    color(rgba).set();
    line(&points, CHECK).stroke();
}

/// devctl: every state of the icon as a TIFF in `dir` (`<prefix>-<state>.tiff`), drawn in the
/// app's appearance (status items can't be captured from GPUI's render).
#[cfg(feature = "devctl")]
pub(super) fn save_all(
    dir: &std::path::Path,
    prefix: &str,
    colors: StatusColors,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let states: [(&str, bool, Option<usize>, Mark); 9] = [
        ("idle", false, None, Mark::None),
        ("working", true, None, Mark::None),
        ("working-0", true, Some(0), Mark::None),
        ("working-1", true, Some(1), Mark::None),
        ("working-2", true, Some(2), Mark::None),
        ("working-3", true, Some(3), Mark::None),
        ("needs-you", false, None, Mark::NeedsYou),
        ("failed", false, None, Mark::Failed),
        ("finished", false, None, Mark::Finished),
    ];
    let mut saved = Vec::new();
    for (name, filled, frame, mark) in states {
        let data = image(filled, frame, mark, colors, name)
            .TIFFRepresentation()
            .ok_or_else(|| anyhow::anyhow!("no TIFF for {name}"))?;
        let path = dir.join(format!("{prefix}-{name}.tiff"));
        std::fs::write(&path, data.to_vec())?;
        saved.push(path);
    }
    Ok(saved)
}
