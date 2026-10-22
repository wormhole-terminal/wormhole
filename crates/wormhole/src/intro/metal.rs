//! The intro's drawing on macOS: a borderless, click-through window over the whole screen the app
//! window is on, above the menu bar and the Dock, whose content is a `CAMetalLayer` that
//! `shader/` draws into every frame. GPUI draws only inside a window and has no custom shaders,
//! so space around the window needs a window of its own; the shader leaves the app window's own
//! rectangle clear, so it shows through. While Emerge and Collapse play, the app window's layers
//! are scaled about its centre and the window faded in; dropping the [`Overlay`] puts both back.

use std::cell::Cell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use anyhow::{Context as _, Result, anyhow};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSStatusWindowLevel, NSView, NSWindow, NSWindowCollectionBehavior,
    NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSString};
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice,
    MTLDevice, MTLDrawable, MTLLibrary, MTLLoadAction, MTLPixelFormat, MTLPrimitiveType,
    MTLRenderCommandEncoder, MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
    MTLStoreAction, MTLTexture,
};
use objc2_quartz_core::{CALayer, CAMetalDrawable, CAMetalLayer, CATransform3D};
use wormhole_core::intro::page;
use wormhole_core::intro::voyage::{self, Landing, TRAIL_POINTS};
use wormhole_core::intro::{Start, Style, WindowMotion, window_motion_from};

/// The shader: one file per style on top of the shared pieces, in the order each needs the one before
/// it (Metal has no `#include` for a library built from source), behind the moments core owns.
fn source() -> String {
    const FILES: [&str; 12] = [
        include_str!("shader/common.metal"),
        include_str!("shader/emerge.metal"),
        include_str!("shader/vortex.metal"),
        include_str!("shader/collapse.metal"),
        include_str!("shader/warp.metal"),
        include_str!("shader/ftl.metal"),
        include_str!("shader/wormhole.metal"),
        include_str!("shader/aurora.metal"),
        include_str!("shader/tentacles.metal"),
        include_str!("shader/voyage.metal"),
        include_str!("shader/glow.metal"),
        include_str!("shader/entry.metal"),
    ];
    FILES.iter().fold(wormhole_core::intro::shader_constants(), |source, file| source + "\n" + file)
}

const PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;
/// macOS's window corners, near enough: the glow hugs them.
const CORNER_RADIUS: f32 = 12.0;
/// The logo's side on the intro window's page, before it has been measured.
const LOGO_FALLBACK: f32 = 96.0;
/// Where a trail that is not to be seen is: far beyond any screen.
const OFF_SCREEN: f32 = -1.0e5;
/// Drawable pixels per point: the effect is soft, and a whole Retina screen of noise at 2x every
/// frame costs four times as much for nothing anyone would see.
const RENDER_SCALE: f32 = 1.0;

/// `Uniforms` in `shader/common.metal`, field for field (float2 fields 8-aligned, float4s 16-aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Uniforms {
    pub size: [f32; 2],
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub scale: f32,
    pub t: f32,
    pub time: f32,
    pub win_scale: f32,
    pub radius: f32,
    pub win_alpha: f32,
    pub style: u32,
    pub speed: f32,
    pub travel: f32,
    /// Voyage's mist: 1 for the smooth tunnel and blurred arms, 0 for the others.
    pub mist: f32,
    /// The wormhole styles' run: its seconds, when the light lifts, the bloom's delay.
    pub run: [f32; 4],
    /// Voyage's ship: x, y, its icon box's side, heading; then thrust, opacity, flare, bank.
    pub ship: [f32; 4],
    pub ship_fx: [f32; 4],
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub c3: [f32; 4],
}

impl Uniforms {
    /// For a screen of `screen` points with a window at `origin` (from the screen's top left) of
    /// `size` points.
    pub(super) fn new(
        screen: [f32; 2],
        origin: [f32; 2],
        size: [f32; 2],
        scale: f32,
        colors: [[f32; 4]; 3],
    ) -> Self {
        Self {
            size: screen,
            center: [origin[0] + size[0] / 2.0, origin[1] + size[1] / 2.0],
            half_size: [size[0] / 2.0, size[1] / 2.0],
            scale,
            radius: CORNER_RADIUS,
            win_scale: 1.0,
            win_alpha: 1.0,
            c1: colors[0],
            c2: colors[1],
            c3: colors[2],
            ..Default::default()
        }
    }

    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn at(self, style: Style, t: f32, time: f32) -> Self {
        self.at_from(style, t, time, Start::Shown)
    }

    /// [`Self::at`] for a window that is `start`ing shown or hidden.
    pub(super) fn at_from(mut self, style: Style, t: f32, time: f32, start: Start) -> Self {
        self.style = style.shader_id();
        self.t = t;
        self.time = time;
        let motion = window_motion_from(style, t, start);
        self.win_scale = motion.scale;
        self.win_alpha = motion.alpha;
        self.speed = wormhole_core::intro::speed(style, t);
        self.travel = wormhole_core::intro::travel(style, t);
        let run = style.run();
        self.run = [run.seconds, run.lift, run.bloom, run.arms];
        self.mist = run.mist;
        self
    }

    /// Voyage's ship at `time`, and the comet trail behind it: the uniforms to draw them, over a screen
    /// of `screen` points with the logo where `landing` says. Any other style has no ship, and neither
    /// has the Portal hand-over (`ship` false): its trail is moved off the screen, as the shader draws
    /// the trail whatever the ship's opacity.
    pub(super) fn with_ship(
        mut self,
        style: Style,
        ship: bool,
        time: f32,
        screen: [f32; 2],
        landing: Landing,
    ) -> (Self, [[f32; 2]; TRAIL_POINTS]) {
        if style != Style::Voyage {
            return (self, [[0.0; 2]; TRAIL_POINTS]);
        }
        if !ship {
            return (self, [[OFF_SCREEN; 2]; TRAIL_POINTS]);
        }
        let ship = voyage::ship(time, screen, landing);
        self.ship = [ship.pos[0], ship.pos[1], ship.size, ship.heading];
        self.ship_fx = [ship.thrust, ship.opacity, ship.flare, ship.bank];
        (self, voyage::trail(time, screen, landing))
    }
}

/// The compiled shader and a queue to draw with: built once per intro (a few milliseconds).
pub(super) struct Renderer {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

impl Renderer {
    #[allow(unsafe_code)] // Metal interop; see the SAFETY note below.
    pub(super) fn new() -> Result<Self> {
        let device = MTLCreateSystemDefaultDevice().context("no Metal device")?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(&source()), None)
            .map_err(|err| anyhow!("compiling the intro shader: {}", err.localizedDescription()))?;
        let function = |name: &str| {
            library.newFunctionWithName(&NSString::from_str(name)).with_context(|| format!("no {name}"))
        };
        let descriptor = MTLRenderPipelineDescriptor::new();
        let (vertex, fragment) = (function("intro_vertex")?, function("intro_fragment")?);
        descriptor.setVertexFunction(Some(&vertex));
        descriptor.setFragmentFunction(Some(&fragment));
        // SAFETY: index 0 is always within a pipeline's colour attachments.
        unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) }.setPixelFormat(PIXEL_FORMAT);
        let pipeline = device
            .newRenderPipelineStateWithDescriptor_error(&descriptor)
            .map_err(|err| anyhow!("intro pipeline: {}", err.localizedDescription()))?;
        let queue = device.newCommandQueue().context("no Metal command queue")?;
        Ok(Self { device, queue, pipeline })
    }

    /// Encode one frame of `uniforms` into `texture`; the caller presents or reads it back.
    #[allow(unsafe_code)] // Metal interop; see the SAFETY notes below.
    fn encode(
        &self,
        texture: &ProtocolObject<dyn MTLTexture>,
        uniforms: &Uniforms,
        trail: &[[f32; 2]; TRAIL_POINTS],
    ) -> Result<Retained<ProtocolObject<dyn MTLCommandBuffer>>> {
        let pass = MTLRenderPassDescriptor::new();
        // SAFETY: index 0 is always within a pass's colour attachments.
        let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setTexture(Some(texture));
        attachment.setLoadAction(MTLLoadAction::Clear);
        attachment.setClearColor(MTLClearColor { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 });
        attachment.setStoreAction(MTLStoreAction::Store);
        let buffer = self.queue.commandBuffer().context("no command buffer")?;
        let encoder = buffer.renderCommandEncoderWithDescriptor(&pass).context("no render encoder")?;
        encoder.setRenderPipelineState(&self.pipeline);
        // SAFETY: `uniforms` is a live `repr(C)` value of the size passed, laid out as the
        // shader's `Uniforms`; Metal copies the bytes before this returns.
        unsafe {
            encoder.setFragmentBytes_length_atIndex(
                NonNull::from(uniforms).cast::<c_void>(),
                size_of::<Uniforms>(),
                0,
            );
            // The same: `trail` is a live array of the length passed, read by the shader as `float2`s.
            encoder.setFragmentBytes_length_atIndex(
                NonNull::from(trail).cast::<c_void>(),
                size_of::<[[f32; 2]; TRAIL_POINTS]>(),
                1,
            );
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
        }
        encoder.endEncoding();
        Ok(buffer)
    }

    #[cfg(any(test, feature = "devctl"))]
    /// One frame drawn off screen: `width` × `height` pixels, premultiplied RGBA, rows top first.
    #[allow(unsafe_code)] // Metal interop; see the SAFETY notes below.
    pub(super) fn render_pixels(
        &self,
        width: usize,
        height: usize,
        uniforms: &Uniforms,
        trail: &[[f32; 2]; TRAIL_POINTS],
    ) -> Result<Vec<u8>> {
        use objc2_metal::{
            MTLBlitCommandEncoder, MTLOrigin, MTLRegion, MTLSize, MTLStorageMode, MTLTextureDescriptor,
            MTLTextureUsage,
        };

        // SAFETY: a plain 2D descriptor with a valid format and non-zero size.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                PIXEL_FORMAT,
                width,
                height,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::RenderTarget);
        descriptor.setStorageMode(MTLStorageMode::Managed);
        let texture = self.device.newTextureWithDescriptor(&descriptor).context("no texture")?;
        let buffer = self.encode(&texture, uniforms, trail)?;
        let blit = buffer.blitCommandEncoder().context("no blit encoder")?;
        blit.synchronizeResource(ProtocolObject::from_ref(&*texture));
        blit.endEncoding();
        buffer.commit();
        buffer.waitUntilCompleted();

        let mut bgra = vec![0u8; width * height * 4];
        let region =
            MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width, height, depth: 1 } };
        // SAFETY: `bgra` holds exactly `height` rows of `width * 4` bytes, the region's size.
        unsafe {
            texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                NonNull::new(bgra.as_mut_ptr().cast::<c_void>()).context("null buffer")?,
                width * 4,
                region,
                0,
            );
        }
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        Ok(bgra)
    }
}

/// What the app window looked like before the intro touched it.
struct Saved {
    alpha: f64,
    shadow: bool,
}

/// The intro on screen around one window. Dropping it closes the overlay and puts the window back.
pub(super) struct Overlay {
    renderer: Renderer,
    window: Retained<NSWindow>,
    parent: Retained<NSWindow>,
    layer: Retained<CAMetalLayer>,
    /// The app window's layers that grow with it (its content and the blur behind).
    scaled: Vec<Retained<CALayer>>,
    saved: Saved,
    style: Style,
    /// Whether the window was showing as the intro began.
    start: Start,
    /// Whether the intro fades the traffic lights away as it begins (a replay over a showing window).
    /// It never brings them back: they belong to the intro window, which does when it is closed.
    lights: bool,
    /// Whether Voyage's ship flies: not in the terminal welcome's Portal hand-over.
    ship: bool,
    uniforms: Uniforms,
    /// The window's top left on the screen, and the screen's size: where Voyage's ship flies.
    origin: [f32; 2],
    screen: [f32; 2],
    window_size: [f32; 2],
    /// Where the page's logo is in the window (its middle and its side), once it has measured it.
    landing: Rc<Cell<Option<[f32; 3]>>>,
}

/// One frame of the intro: where the run has got to, and what is asked of the window around it.
pub(super) struct Frame {
    /// Progress across the run, 0 to 1, and the seconds since it began.
    pub t: f32,
    pub time: f32,
    /// How opaque the traffic lights are while the overlay fades them away, or `None` to leave them be.
    pub lights: Option<f32>,
    /// After a skip: how the window was, and how far it has settled back to rest (0 to 1), the
    /// overlay fading away with it.
    pub settling: Option<(WindowMotion, f32)>,
}

impl Overlay {
    /// Put an overlay over the screen of the window whose GPUI view is `view`: above everything,
    /// or, when not `immersive` (test instances kept behind other apps), just above that window.
    #[allow(unsafe_code)] // AppKit interop; see the SAFETY notes below.
    pub(super) fn new(
        view: &NSView,
        style: Style,
        colors: [[f32; 4]; 3],
        immersive: bool,
        options: super::Options,
        landing: Rc<Cell<Option<[f32; 3]>>>,
    ) -> Result<Self> {
        let mtm = MainThreadMarker::new().context("not on the main thread")?;
        let parent = view.window().context("the view has no window")?;
        let renderer = Renderer::new()?;

        let frame = parent.frame();
        let rect = parent.screen().context("the window is on no screen")?.frame();
        // AppKit's screen coordinates grow upwards; the shader's from the top left.
        let origin = [
            (frame.origin.x - rect.origin.x) as f32,
            (rect.origin.y + rect.size.height - frame.origin.y - frame.size.height) as f32,
        ];
        let size = [frame.size.width as f32, frame.size.height as f32];
        let screen = [rect.size.width as f32, rect.size.height as f32];

        // SAFETY: on the main thread, with a valid frame; the window is kept by `Retained`, so
        // AppKit must not also release it when it closes.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                rect,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setHasShadow(false);
        window.setIgnoresMouseEvents(true);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );

        let layer = CAMetalLayer::new();
        layer.setDevice(Some(&renderer.device));
        layer.setPixelFormat(PIXEL_FORMAT);
        layer.setOpaque(false);
        layer.setContentsScale(f64::from(RENDER_SCALE));
        layer.setDrawableSize(objc2_core_foundation::CGSize::new(
            rect.size.width * f64::from(RENDER_SCALE),
            rect.size.height * f64::from(RENDER_SCALE),
        ));
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), rect.size));
        // A layer-hosting view: the layer is set before `wantsLayer`.
        content.setLayer(Some(&layer));
        content.setWantsLayer(true);
        window.setContentView(Some(&content));

        let scaled = match style {
            Style::Emerge
            | Style::Collapse
            | Style::Ftl
            | Style::Aurora
            | Style::Tentacles
            | Style::Voyage => {
                // SAFETY: the view is GPUI's live content view and we are on the main thread.
                let siblings = unsafe { view.superview() }.map(|parent| parent.subviews());
                siblings.into_iter().flatten().filter_map(|sibling| sibling.layer()).collect()
            }
            Style::Vortex | Style::Glow => Vec::new(),
        };
        let saved = Saved { alpha: parent.alphaValue(), shadow: parent.hasShadow() };
        if !scaled.is_empty() {
            // The window server's shadow would show the window's full size around a small one.
            parent.setHasShadow(false);
        }

        if immersive {
            // Over the menu bar and the Dock too: the whole screen goes to space.
            window.setLevel(NSStatusWindowLevel);
            window.orderFrontRegardless();
        } else {
            // SAFETY: both windows are live and on the main thread.
            unsafe { parent.addChildWindow_ordered(&window, NSWindowOrderingMode::Above) };
        }
        let uniforms = Uniforms::new(screen, origin, size, RENDER_SCALE, colors);
        let (start, lights) = (options.start, options.lights);
        let ship = options.handover.is_none_or(|handover| handover.ship());
        let overlay = Self {
            renderer,
            window,
            parent,
            layer,
            scaled,
            saved,
            style,
            start,
            lights,
            ship,
            uniforms,
            origin,
            screen,
            window_size: size,
            landing,
        };
        overlay.frame(&Frame { t: 0.0, time: 0.0, lights: Some(page::lights(0.0, start)), settling: None });
        Ok(overlay)
    }

    /// How the window moves at `t`: what a skip settles it from.
    pub(super) fn motion_at(&self, t: f32) -> WindowMotion {
        window_motion_from(self.style, t, self.start)
    }

    /// Draw `frame`.
    pub(super) fn frame(&self, frame: &Frame) {
        let mut motion = self.motion_at(frame.t);
        let mut opacity = 1.0;
        if let Some((from, settled)) = frame.settling {
            let lerp = |a: f32, b: f32| a + (b - a) * settled;
            motion = WindowMotion {
                scale: lerp(from.scale, WindowMotion::AT_REST.scale),
                alpha: lerp(from.alpha, WindowMotion::AT_REST.alpha),
            };
            opacity = 1.0 - settled;
        }
        self.move_window(motion);
        self.layer.setOpacity(opacity);
        if let (true, Some(lights)) = (self.lights, frame.lights) {
            crate::platform::window::set_opacity(&self.parent, lights);
        }
        let mut uniforms = self.uniforms.at_from(self.style, frame.t, frame.time, self.start);
        // The shader leaves the window's rectangle clear as far as the window shows.
        (uniforms.win_scale, uniforms.win_alpha) = (motion.scale, motion.alpha);
        let (mut uniforms, trail) =
            uniforms.with_ship(self.style, self.ship, frame.time, self.screen, self.landing());
        // After a skip the ship goes with the rest.
        if let Some((_, settled)) = frame.settling {
            uniforms.ship_fx[1] *= 1.0 - settled;
        }
        let Some(drawable) = self.layer.nextDrawable() else { return };
        match self.renderer.encode(&drawable.texture(), &uniforms, &trail) {
            Ok(buffer) => {
                let drawable: &ProtocolObject<dyn CAMetalDrawable> = &drawable;
                buffer.presentDrawable(ProtocolObject::<dyn MTLDrawable>::from_ref(drawable));
                buffer.commit();
            }
            Err(err) => tracing::warn!("intro frame: {err:#}"),
        }
    }

    /// The logo on screen: where the page measured it, or, before it has, a guess at where it sits.
    fn landing(&self) -> Landing {
        let [x, y, size] = self.landing.get().unwrap_or([
            self.window_size[0] / 2.0,
            self.window_size[1] * 0.2,
            LOGO_FALLBACK,
        ]);
        Landing { centre: [self.origin[0] + x, self.origin[1] + y], size }
    }

    fn move_window(&self, motion: WindowMotion) {
        if self.scaled.is_empty() {
            return;
        }
        self.parent.setAlphaValue(f64::from(motion.alpha) * self.saved.alpha);
        for layer in &self.scaled {
            layer.setTransform(scale_about_centre(layer.bounds().size, f64::from(motion.scale)));
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        for layer in &self.scaled {
            layer.setTransform(scale_about_centre(layer.bounds().size, 1.0));
        }
        if !self.scaled.is_empty() {
            self.parent.setAlphaValue(self.saved.alpha);
            self.parent.setHasShadow(self.saved.shadow);
            self.parent.invalidateShadow();
        }
        self.parent.removeChildWindow(&self.window);
        self.window.orderOut(None);
        self.window.close();
    }
}

/// A scale by `s` about the centre of a layer of `size`: AppKit anchors a view's layer at its
/// corner, so the scale is moved back by what it shifts the centre.
fn scale_about_centre(size: objc2_core_foundation::CGSize, s: f64) -> CATransform3D {
    let mut m = CATransform3D {
        m11: s,
        m12: 0.0,
        m13: 0.0,
        m14: 0.0,
        m21: 0.0,
        m22: s,
        m23: 0.0,
        m24: 0.0,
        m31: 0.0,
        m32: 0.0,
        m33: 1.0,
        m34: 0.0,
        m41: 0.0,
        m42: 0.0,
        m43: 0.0,
        m44: 1.0,
    };
    m.m41 = size.width / 2.0 * (1.0 - s);
    m.m42 = size.height / 2.0 * (1.0 - s);
    m
}

#[cfg(test)]
mod tests {
    use super::{Landing, Renderer, Uniforms};
    use wormhole_core::intro::Style;

    /// The shader compiles at run time, so a mistake in it would only show as an intro that
    /// never plays: compile it, and draw a frame of each style.
    #[test]
    fn the_shader_compiles_and_draws() {
        let renderer = Renderer::new().expect("the intro shader compiles");
        let colors = [[0.4, 0.5, 1.0, 1.0], [0.7, 0.4, 1.0, 1.0], [0.7, 0.4, 0.9, 1.0]];
        let uniforms = Uniforms::new([256.0, 240.0], [96.0, 96.0], [64.0, 48.0], 1.0, colors);
        for style in [
            Style::Emerge,
            Style::Vortex,
            Style::Collapse,
            Style::Ftl,
            Style::Aurora,
            Style::Tentacles,
            Style::Voyage,
            Style::Glow,
        ] {
            let (uniforms, trail) = uniforms.at(style, 0.5, 0.5).with_ship(
                style,
                true,
                0.5 * style.seconds(),
                [256.0, 240.0],
                Landing { centre: [128.0, 80.0], size: 24.0 },
            );
            let pixels = renderer.render_pixels(256, 240, &uniforms, &trail).expect("a frame");
            assert!(pixels.as_chunks::<4>().0.iter().any(|px| px[3] > 0), "{style:?} drew nothing");
        }
    }

    /// The Portal hand-over is the Voyage run without its ship: where the ship docks the picture is
    /// empty (the window covers the whole screen, so nothing else is drawn there), and with the ship
    /// it is not. The comet trail goes with it.
    #[test]
    fn a_voyage_without_a_ship_draws_none_where_it_docks() {
        use wormhole_core::intro::voyage;

        let renderer = Renderer::new().expect("the intro shader compiles");
        let colors = [[0.4, 0.5, 1.0, 1.0], [0.7, 0.4, 1.0, 1.0], [0.7, 0.4, 0.9, 1.0]];
        let screen = [256.0, 240.0];
        let uniforms = Uniforms::new(screen, [0.0, 0.0], screen, 1.0, colors);
        let landing = Landing { centre: [128.0, 96.0], size: 48.0 };
        let [cx, cy] = landing.chevron();
        let busiest = |ship: bool, time: f32| -> u8 {
            let (uniforms, trail) = uniforms
                .at(Style::Voyage, time / Style::Voyage.seconds(), time)
                .with_ship(Style::Voyage, ship, time, screen, landing);
            let pixels = renderer.render_pixels(256, 240, &uniforms, &trail).expect("a frame");
            let mut most = 0;
            for y in (cy as usize).saturating_sub(24)..(cy as usize + 24) {
                for x in (cx as usize).saturating_sub(24)..(cx as usize + 24) {
                    most = most.max(pixels[(y * 256 + x) * 4 + 3]);
                }
            }
            most
        };
        for time in [voyage::DOCK, voyage::LAND] {
            assert!(busiest(true, time) > 200, "the ship is there at {time}s");
            assert!(busiest(false, time) < 8, "and with none it is not, at {time}s");
        }
    }
}
