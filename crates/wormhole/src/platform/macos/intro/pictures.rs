//! devctl's pictures of the intro (`intro-frames`, `welcome-dock`): the shader drawn off screen,
//! over a stand-in or over the window's own picture.

use std::path::Path;

use anyhow::{Context as _, Result};
use gpui_kit::Window;
use wormhole_core::onboarding::intro::voyage::{self, Landing};
use wormhole_core::onboarding::intro::{Style, window_motion};

use super::{Renderer, Uniforms};

/// `count` frames of `style` over a stand-in screen and window, to `<dir>/intro-<style>-<n>.bmp`.
pub fn frames(
    style: Style,
    count: usize,
    colors: [[f32; 4]; 3],
    stand_in: [[f32; 3]; 3],
    dir: &Path,
) -> Result<String> {
    let (w, h) = (1440, 900);
    let (width, height) = (860.0_f32, 540.0_f32);
    let origin = [290.0_f32, 180.0_f32];
    let [desk, win, edge] = stand_in;
    let uniforms = Uniforms::new([w as f32, h as f32], origin, [width, height], 1.0, colors);
    let centre = [origin[0] + width / 2.0, origin[1] + height / 2.0];
    let renderer = Renderer::new()?;
    let mut out = String::new();
    for n in 0..count {
        let t = (n as f32 + 0.5) / count as f32;
        let time = t * style.seconds();
        // The logo's spot on the stand-in window: its top, as the page has it.
        let landing = Landing { centre: [centre[0], origin[1] + 88.0], size: 96.0 };
        let (scene, trail) =
            uniforms.at(style, t, time).with_ship(style, true, time, [w as f32, h as f32], landing);
        let pixels = renderer.render_pixels(w, h, &scene, &trail)?;
        let motion = window_motion(style, t);
        let mut rgb_out = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                let (px, py) = (x as f32 - centre[0], y as f32 - centre[1]);
                let (hw, hh) = (width / 2.0 * motion.scale, height / 2.0 * motion.scale);
                let mut base = desk;
                if px.abs() <= hw && py.abs() <= hh {
                    let on_edge = hw - px.abs() < 1.0 || hh - py.abs() < 1.0;
                    let fill = if on_edge { edge } else { win };
                    base = std::array::from_fn(|i| base[i] + (fill[i] - base[i]) * motion.alpha);
                }
                let i = (y * w + x) * 4;
                let a = f32::from(pixels[i + 3]) / 255.0;
                for c in 0..3 {
                    let lit = f32::from(pixels[i + c]) / 255.0 + base[c] * (1.0 - a);
                    rgb_out.push((lit.clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }
        let path = dir.join(format!("intro-{}-{n}.bmp", style.key()));
        std::fs::write(&path, bmp(w, h, &rgb_out)).with_context(|| path.display().to_string())?;
        out += &format!("saved {}\n", path.display());
    }
    Ok(out)
}

/// Voyage's ship at `landing` (where a terminal's dock says, in window points), drawn over the
/// window's own picture at four moments of the landing, to `<dir>/<name>-<n>.png`: the ship on its
/// way in, docked, at the touchdown, and dissolving into what is under it.
pub fn dock_frames(
    window: &Window,
    landing: Landing,
    colors: [[f32; 4]; 3],
    dir: &Path,
    name: &str,
) -> Result<String> {
    let base = window.render_to_image()?;
    let size = window.viewport_size();
    let screen = [f32::from(size.width), f32::from(size.height)];
    let scale = base.width() as f32 / screen[0];
    let (w, h) = (base.width() as usize, base.height() as usize);
    let renderer = Renderer::new()?;
    let style = Style::Voyage;
    let moments = [voyage::DOCK - 0.5, voyage::DOCK, voyage::LAND, voyage::LAND + 0.2];
    let mut out = String::new();
    for (n, time) in moments.into_iter().enumerate() {
        // The window fills the picture, so the shader leaves all of it clear but the ship.
        let uniforms = Uniforms::new(screen, [0.0, 0.0], screen, scale, colors);
        let (uniforms, trail) =
            uniforms.at(style, time / style.seconds(), time).with_ship(style, true, time, screen, landing);
        let pixels = renderer.render_pixels(w, h, &uniforms, &trail)?;
        let mut shot = base.clone();
        for (px, ship) in shot.pixels_mut().zip(pixels.as_chunks::<4>().0) {
            let a = f32::from(ship[3]) / 255.0;
            for (under, over) in px.0.iter_mut().zip(ship).take(3) {
                // The ship is premultiplied: it over the picture is the ship plus the rest.
                let lit = f32::from(*over) + f32::from(*under) * (1.0 - a);
                *under = lit.clamp(0.0, 255.0) as u8;
            }
        }
        let path = dir.join(format!("{name}-{n}.png"));
        shot.save(&path).with_context(|| path.display().to_string())?;
        out += &format!("saved {} (ship at {time:.2}s)\n", path.display());
    }
    let [x, y] = landing.chevron();
    out += &format!("chevron at ({x:.1}, {y:.1}) pt, canvas {:.1} pt\n", landing.canvas());
    Ok(out)
}

/// A 24-bit BMP of `rgb` rows, top first: no image crate in the app for a debug picture.
fn bmp(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
    let row = (width * 3).div_ceil(4) * 4;
    let size = 54 + row * height;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    // Negative: rows top first.
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 3;
            out.extend_from_slice(&[rgb[i + 2], rgb[i + 1], rgb[i]]);
        }
        out.resize(out.len() + row - width * 3, 0);
    }
    out
}
