//! Inline images in the pane, drawn from the terminal's image store (core `terminal::images`):
//! the pictures the engine placed in the grid, and the tiles of kitty's Unicode placeholder cells.
//!
//! Every placeholder cell says which tile of which virtual placement it shows; the row builder
//! groups consecutive tiles into runs, and each run paints as one clip of the whole picture laid
//! over the placement's cells. The picture is converted to GPUI's BGRA frame once per image and
//! kept here until the store forgets the image.

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::{Bounds, Pixels, Point, RenderImage, Size, point, px, size};
use wormhole_term::images::{Image, ImageStore, Tiles};
use wormhole_term::types::{Extent, ImagePlacement};

use super::frame::Geometry;
use super::rows::ImageRun;

/// One image paint of a frame: the run's cells, and where the whole picture is.
pub(super) struct ImagePaint {
    pub bounds: Bounds<Pixels>,
    pub image_bounds: Bounds<Pixels>,
    pub image: Arc<RenderImage>,
}

struct Cached {
    /// The store's pixels this was made from (a re-sent image with the same id is another `Arc`).
    source: *const Image,
    image: Arc<RenderImage>,
}

/// GPUI's frames for the store's images, converted once each.
#[derive(Default)]
pub(super) struct ImageCache {
    frames: HashMap<u32, Cached>,
}

impl ImageCache {
    fn render_image(&mut self, id: u32, image: &Arc<Image>) -> Arc<RenderImage> {
        let source = Arc::as_ptr(image);
        if let Some(cached) = self.frames.get(&id)
            && std::ptr::eq(cached.source, source)
        {
            return cached.image.clone();
        }
        let converted = Arc::new(to_render_image(image));
        self.frames.insert(id, Cached { source, image: converted.clone() });
        converted
    }

    /// Drop what the store no longer has.
    fn retain(&mut self, store: &ImageStore) {
        self.frames.retain(|id, _| store.contains(*id));
    }
}

/// GPUI keeps its images as BGRA: the red and blue channels swapped, straight alpha as decoded.
fn to_render_image(image: &Image) -> RenderImage {
    let mut bgra = image.rgba.clone();
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let buffer = image::ImageBuffer::from_raw(image.width, image.height, bgra)
        .unwrap_or_else(|| image::ImageBuffer::from_pixel(1, 1, image::Rgba([0, 0, 0, 0])));
    RenderImage::new(vec![image::Frame::new(buffer)])
}

/// Where a `cols` × `rows` placement at `origin` draws a `width` × `height` picture, in pixels:
/// the picture's shape centred inside those cells, at the largest size that fits.
fn fitted_rect(
    origin: Point<Pixels>,
    cols: u16,
    rows: u16,
    (width, height): (f32, f32),
    geometry: Geometry,
) -> Bounds<Pixels> {
    let full = size(geometry.cell_width * f32::from(cols), geometry.line_height * f32::from(rows));
    if width <= 0.0 || height <= 0.0 {
        return Bounds::new(origin, full);
    }
    let scale = (f32::from(full.width) / width).min(f32::from(full.height) / height);
    let drawn = size(px(width * scale), px(height * scale));
    let inset = point((full.width - drawn.width) / 2.0, (full.height - drawn.height) / 2.0);
    Bounds::new(origin + inset, drawn)
}

/// The whole picture's rectangle, laid so that its part `src` (x, y, width, height in its pixels)
/// fills `shown`.
fn lay_over(
    shown: Bounds<Pixels>,
    (src_x, src_y, src_w, src_h): (f32, f32, f32, f32),
    (full_w, full_h): (f32, f32),
) -> Bounds<Pixels> {
    let (sx, sy) = (f32::from(shown.size.width) / src_w, f32::from(shown.size.height) / src_h);
    Bounds::new(
        point(shown.origin.x - px(src_x * sx), shown.origin.y - px(src_y * sy)),
        Size { width: px(full_w * sx), height: px(full_h * sy) },
    )
}

/// The paint for one run: its cells clip the whole picture, laid over the `tiles` placement so
/// that the run's tiles land on its cells.
fn run_paint(
    row: usize,
    run: &ImageRun,
    image: &Image,
    tiles: &Tiles,
    render_image: Arc<RenderImage>,
    geometry: Geometry,
) -> ImagePaint {
    let Geometry { origin, cell_width, line_height, .. } = geometry;
    let at = |col: usize, row: usize| {
        point(origin.x + cell_width * col as f32, origin.y + line_height * row as f32)
    };
    let bounds = Bounds::new(at(run.col, row), size(cell_width * run.cells as f32, line_height));
    // The placement's top left: this run's first tile is `tile_col` cells in and `tile_row` down.
    let placement_origin = point(
        bounds.origin.x - cell_width * f32::from(run.tile_col),
        bounds.origin.y - line_height * f32::from(run.tile_row),
    );
    let (full_w, full_h) = (image.width as f32, image.height as f32);
    let shown = Bounds::new(
        placement_origin,
        size(
            cell_width * tiles.cols.min(u32::from(u16::MAX)) as f32,
            line_height * tiles.rows.min(u32::from(u16::MAX)) as f32,
        ),
    );
    let image_bounds = lay_over(shown, (0.0, 0.0, full_w, full_h), (full_w, full_h));
    // Only the shown part may paint: the run's cells, within the placement's picture.
    let bounds = bounds.intersect(&shown);
    ImagePaint { bounds, image_bounds, image: render_image }
}

/// The paints for every image run in view (`runs`: viewport row, run), from the store. `tiles`
/// is what the engine says about the virtual placements the runs' cells name (asked for
/// beforehand, `Pty::virtual_placement`): the cells carry the program's ids, and the store holds
/// the pixels under the engine's handles.
pub(super) fn image_paints<'a>(
    runs: impl Iterator<Item = (usize, &'a ImageRun)>,
    tiles: &HashMap<(u32, u32), Tiles>,
    store: &mut ImageStore,
    cache: &mut ImageCache,
    geometry: Geometry,
) -> Vec<ImagePaint> {
    let mut paints = Vec::new();
    for (row, run) in runs {
        // No virtual placement for the cell: nothing says how big its picture is.
        let Some(tiles) = tiles.get(&(run.image, run.placement)) else { continue };
        let Some(image) = store.get(tiles.key) else { continue };
        let render_image = cache.render_image(tiles.key, &image);
        let paint = run_paint(row, run, &image, tiles, render_image, geometry);
        if paint.bounds.size.width > Pixels::ZERO && paint.bounds.size.height > Pixels::ZERO {
            paints.push(paint);
        }
    }
    cache.retain(store);
    paints
}

/// The paints for the pictures the engine placed in the grid, split into those drawn under the
/// text (negative `z`, as kitty has it) and those over it. `offset` is how far the view is
/// scrolled into the history, `clip` the grid's pane: a tall picture that scrolled partly out of
/// view must not draw past it. Pixel sizes and offsets are in the cells' own units, which is what
/// the engine was told a cell measures.
pub(super) fn placement_paints(
    placements: &[ImagePlacement],
    offset: usize,
    store: &mut ImageStore,
    cache: &mut ImageCache,
    clip: Bounds<Pixels>,
    geometry: Geometry,
) -> (Vec<ImagePaint>, Vec<ImagePaint>) {
    let (mut under, mut over) = (Vec::new(), Vec::new());
    for placement in placements {
        let id = placement_key(placement);
        let Some(image) = store.get(id) else { continue };
        let render_image = cache.render_image(id, &image);
        let Some(paint) = placement_paint(placement, offset, &image, render_image, clip, geometry) else {
            continue;
        };
        if placement.z < 0 { under.push(paint) } else { over.push(paint) }
    }
    cache.retain(store);
    (under, over)
}

/// The store's key of a placement's picture (its handle).
fn placement_key(placement: &ImagePlacement) -> u32 {
    wormhole_term::images::key_of(placement.image)
}

/// Where one placement draws: the part of the picture it shows, sized by its extent over the
/// cells it covers, clipped to the pane.
fn placement_paint(
    placement: &ImagePlacement,
    offset: usize,
    image: &Image,
    render_image: Arc<RenderImage>,
    clip: Bounds<Pixels>,
    geometry: Geometry,
) -> Option<ImagePaint> {
    let Geometry { origin, cell_width, line_height, .. } = geometry;
    let row = placement.line.0 + offset as i32;
    let corner = point(origin.x + cell_width * placement.column as f32, origin.y + line_height * row as f32);
    let cells = size(cell_width * placement.cols as f32, line_height * placement.rows as f32);
    let (full_w, full_h) = (image.width as f32, image.height as f32);
    let source = placement.source;
    let src = if source.width > 0 && source.height > 0 {
        (source.x as f32, source.y as f32, source.width as f32, source.height as f32)
    } else {
        (0.0, 0.0, full_w, full_h)
    };
    let start = point(corner.x + px(placement.offset_x as f32), corner.y + px(placement.offset_y as f32));
    let shown = match placement.extent {
        Extent::Fill => Bounds::new(
            start,
            size(
                (cells.width - px(placement.offset_x as f32)).max(px(1.0)),
                (cells.height - px(placement.offset_y as f32)).max(px(1.0)),
            ),
        ),
        Extent::Fit => {
            let (cols, rows) =
                (placement.cols.min(usize::from(u16::MAX)), placement.rows.min(usize::from(u16::MAX)));
            fitted_rect(corner, cols as u16, rows as u16, (src.2, src.3), geometry)
        }
        Extent::Pixels { width, height } => Bounds::new(start, size(px(width as f32), px(height as f32))),
    };
    let image_bounds = lay_over(shown, src, (full_w, full_h));
    let bounds = shown.intersect(&clip);
    (bounds.size.width > Pixels::ZERO && bounds.size.height > Pixels::ZERO).then_some(ImagePaint {
        bounds,
        image_bounds,
        image: render_image,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use gpui_kit::{Bounds, point, px, size};
    use wormhole_term::images::{Image, ImageStore, Tiles};
    use wormhole_term::types::{Extent, ImageHandle, ImagePlacement, Line, SourceRect};

    use super::super::frame::Geometry;
    use super::super::rows::ImageRun;
    use super::{ImageCache, fitted_rect, image_paints, placement_paint};

    const GEOMETRY: Geometry = Geometry {
        origin: gpui_kit::Point { x: px(10.0), y: px(40.0) },
        cell_width: px(8.0),
        line_height: px(16.0),
        scale: 2.0,
        font_size: 14.0,
    };

    fn image(width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image { width, height, rgba: vec![0; (width * height * 4) as usize] })
    }

    fn rect(bounds: &Bounds<gpui_kit::Pixels>) -> (f32, f32, f32, f32) {
        (bounds.origin.x.into(), bounds.origin.y.into(), bounds.size.width.into(), bounds.size.height.into())
    }

    #[test]
    fn a_fitted_picture_is_centred_in_its_cells() {
        let origin = point(px(0.0), px(0.0));
        // 4×2 cells are 32×32 px; a 64×16 picture fits the width at 32×8, centred vertically.
        assert_eq!(rect(&fitted_rect(origin, 4, 2, (64.0, 16.0), GEOMETRY)), (0.0, 12.0, 32.0, 8.0));
        // A picture of no size fills them.
        assert_eq!(rect(&fitted_rect(origin, 4, 2, (0.0, 0.0), GEOMETRY)), (0.0, 0.0, 32.0, 32.0));
    }

    #[test]
    fn placeholders_find_their_picture_through_the_virtual_placement() {
        let mut store = ImageStore::default();
        store.insert(9, image(16, 32));
        let mut cache = ImageCache::default();
        // The cells say picture 5; the engine says picture 5 is the store's 9, two tiles across
        // and two down.
        let tiles = HashMap::from([((5, 0), Tiles { key: 9, cols: 2, rows: 2 })]);
        // The second row of the placement at viewport row 3, columns 4 and 5.
        let run = ImageRun { col: 4, cells: 2, image: 5, placement: 0, tile_row: 1, tile_col: 0 };
        let paints = image_paints([(3usize, &run)].into_iter(), &tiles, &mut store, &mut cache, GEOMETRY);
        assert_eq!(paints.len(), 1);
        assert_eq!(rect(&paints[0].bounds), (42.0, 88.0, 16.0, 16.0), "the run's cells");
        assert_eq!(rect(&paints[0].image_bounds), (42.0, 72.0, 16.0, 32.0), "the picture a row higher");
        // The second tile of a one-row placement: the picture a cell to the left.
        let tiles = HashMap::from([((5, 0), Tiles { key: 9, cols: 2, rows: 1 })]);
        let run = ImageRun { col: 0, cells: 1, image: 5, placement: 0, tile_row: 0, tile_col: 1 };
        let paints = image_paints([(0usize, &run)].into_iter(), &tiles, &mut store, &mut cache, GEOMETRY);
        assert_eq!(rect(&paints[0].bounds), (10.0, 40.0, 8.0, 16.0), "the second tile's cell");
        assert_eq!(rect(&paints[0].image_bounds), (2.0, 40.0, 16.0, 16.0), "the picture, a cell to the left");
        // A cell the engine knows no placement for draws nothing, whatever the store has under 5.
        store.insert(5, image(8, 8));
        let none = HashMap::new();
        assert!(
            image_paints([(0usize, &run)].into_iter(), &none, &mut store, &mut cache, GEOMETRY).is_empty()
        );
        // Still decoding: nothing to paint; the cache follows the store.
        store.reserve(6, 8, 8);
        let decoding = HashMap::from([((6, 0), Tiles { key: 6, cols: 1, rows: 1 })]);
        let waiting = ImageRun { col: 0, cells: 1, image: 6, placement: 0, tile_row: 0, tile_col: 0 };
        assert!(
            image_paints([(0usize, &waiting)].into_iter(), &decoding, &mut store, &mut cache, GEOMETRY)
                .is_empty()
        );
        assert_eq!(cache.frames.len(), 1);
        store.remove(9);
        image_paints(std::iter::empty(), &tiles, &mut store, &mut cache, GEOMETRY);
        assert!(cache.frames.is_empty());
    }

    fn placed(line: i32, column: usize, extent: Extent) -> ImagePlacement {
        ImagePlacement {
            image: ImageHandle::from_raw(1),
            image_id: 0,
            placement_id: 0,
            line: Line(line),
            column,
            cols: 2,
            rows: 2,
            source: SourceRect { x: 0, y: 0, width: 16, height: 32 },
            offset_x: 0,
            offset_y: 0,
            extent,
            z: 0,
        }
    }

    #[test]
    fn a_placed_picture_fills_its_cells_and_is_cut_where_it_scrolled_out_of_the_pane() {
        let clip = Bounds::new(point(px(0.0), px(40.0)), size(px(400.0), px(200.0)));
        let image = image(16, 32);
        let render = Arc::new(super::to_render_image(&image));
        // Two cells right of the origin and one row down; the viewport's top row is line 0.
        let paint = placement_paint(&placed(1, 2, Extent::Fill), 0, &image, render.clone(), clip, GEOMETRY)
            .expect("on screen");
        assert_eq!(rect(&paint.bounds), (26.0, 56.0, 16.0, 32.0));
        assert_eq!(rect(&paint.image_bounds), (26.0, 56.0, 16.0, 32.0));
        // Scrolled up a row into the history: its first row is above the pane and not drawn, the
        // picture stays where it was laid.
        let paint = placement_paint(&placed(-1, 0, Extent::Fill), 0, &image, render.clone(), clip, GEOMETRY)
            .expect("half of it shows");
        assert_eq!(rect(&paint.bounds), (10.0, 40.0, 16.0, 16.0));
        assert_eq!(rect(&paint.image_bounds), (10.0, 24.0, 16.0, 32.0));
        // A picture at its own pixel size, nudged by the offset, is not stretched to the cells.
        let mut small = placed(0, 0, Extent::Pixels { width: 4, height: 8 });
        small.offset_x = 2;
        small.offset_y = 3;
        let paint = placement_paint(&small, 0, &image, render.clone(), clip, GEOMETRY).expect("on screen");
        assert_eq!(rect(&paint.bounds), (12.0, 43.0, 4.0, 8.0));
        // Entirely above the pane: nothing.
        assert!(placement_paint(&placed(-5, 0, Extent::Fill), 0, &image, render, clip, GEOMETRY).is_none());
    }

    #[test]
    fn the_same_pixels_convert_once_and_new_pixels_again() {
        let mut cache = ImageCache::default();
        let a = image(2, 2);
        let first = cache.render_image(1, &a);
        assert!(Arc::ptr_eq(&first, &cache.render_image(1, &a)));
        let b = image(2, 2);
        assert!(!Arc::ptr_eq(&first, &cache.render_image(1, &b)), "re-sent: a new frame");
        assert_eq!(first.size(0), size(gpui_kit::DevicePixels(2), gpui_kit::DevicePixels(2)));
    }
}
