//! A terminal's images, under a budget.
//!
//! Each terminal keeps its decoded images here, least recently drawn first to go, within
//! [`STORE_BUDGET`] of its own and [`GLOBAL_BUDGET`] across every terminal of the process. An
//! image that doesn't fit after evicting everything else of this terminal is dropped (another
//! terminal holds the rest of the global budget). An image is reserved with its size while it is
//! being decoded, so a placement can already refer to it. Where a picture is shown is the
//! engine's (placements in its grid, virtual placements); this holds the pixels, by the key the
//! host gave each picture.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::MAX_IMAGES;

/// Most one terminal's images may take, decoded: what the engine allows each of its two screens
/// (`IMAGE_BUDGET_BYTES`, frames of animations counted), twice, so that what the engine keeps the
/// store can hold. The engine evicts by its own count, and the frames it accepted must find room
/// here without this evicting anything to make it.
pub const STORE_BUDGET: usize = 2 * (wormhole_vt::image::IMAGE_BUDGET_BYTES as usize);
/// Most all terminals' images may take together.
pub const GLOBAL_BUDGET: usize = 512 * 1024 * 1024;

static GLOBAL_BYTES: AtomicUsize = AtomicUsize::new(0);

/// How much every terminal's images take together right now.
pub fn global_bytes() -> usize {
    GLOBAL_BYTES.load(Ordering::Relaxed)
}

/// A decoded image: straight (not premultiplied) RGBA, row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn bytes(&self) -> usize {
        self.rgba.len()
    }
}

struct Slot {
    /// `None` while the image is being decoded.
    image: Option<Arc<Image>>,
    /// The canvases of an animation's frames after the first (frame 2 is `frames[0]`). `None`:
    /// a frame the engine counts that has no canvas here (it did not fit, or was dropped under
    /// load): the one before it shows in its place ([`ImageStore::frame_or_earlier`]).
    frames: Vec<Option<Arc<Image>>>,
    /// The picture is reserved and its own decode has not finished: its pixels are on the way.
    /// (Not "has no pixels": a picture whose first frame was deleted may have none and is not
    /// waiting for any.)
    pending: bool,
    width: u32,
    height: u32,
    /// The tick it was last drawn or stored at (the eviction order).
    used: u64,
}

impl Slot {
    fn bytes(&self) -> usize {
        self.image.as_ref().map_or(0, |image| image.bytes())
            + self.frames.iter().flatten().map(|frame| frame.bytes()).sum::<usize>()
    }
}

pub struct ImageStore {
    images: HashMap<u32, Slot>,
    bytes: usize,
    tick: u64,
    /// What this store may hold: [`STORE_BUDGET`] (smaller in tests).
    budget: usize,
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::with_budget(STORE_BUDGET)
    }
}

impl ImageStore {
    /// A store that holds at most `budget` bytes.
    pub fn with_budget(budget: usize) -> Self {
        Self { images: HashMap::new(), bytes: 0, tick: 0, budget }
    }

    /// How much this terminal's images take.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    fn touch(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }

    /// Make room for `bytes` more by evicting the least recently drawn images of this terminal.
    /// `false` when there is no room even then.
    fn make_room(&mut self, bytes: usize) -> bool {
        if bytes > self.budget {
            return false;
        }
        while self.bytes + bytes > self.budget
            || global_bytes() + bytes > GLOBAL_BUDGET
            || self.images.len() >= MAX_IMAGES
        {
            if !self.evict_oldest() {
                break;
            }
        }
        self.bytes + bytes <= self.budget && global_bytes() + bytes <= GLOBAL_BUDGET
    }

    /// An image of this size is on its way (being decoded): its id is taken and placements may
    /// refer to it. `false` when the store is full of images.
    pub fn reserve(&mut self, id: u32, width: u32, height: u32) -> bool {
        self.remove(id);
        if !self.make_room(0) {
            return false;
        }
        let used = self.touch();
        self.images.insert(id, Slot { image: None, frames: Vec::new(), pending: true, width, height, used });
        true
    }

    /// Keep `image` as `id` (replacing one with that id, filling a reservation), making room by
    /// evicting the least recently drawn. `false` when it can't fit even then.
    pub fn insert(&mut self, id: u32, image: Arc<Image>) -> bool {
        let bytes = image.bytes();
        let (width, height) = (image.width, image.height);
        self.remove_pixels(id);
        if !self.make_room(bytes) {
            self.images.remove(&id);
            return false;
        }
        GLOBAL_BYTES.fetch_add(bytes, Ordering::Relaxed);
        self.bytes += bytes;
        let used = self.touch();
        self.images
            .insert(id, Slot { image: Some(image), frames: Vec::new(), pending: false, width, height, used });
        true
    }

    /// Fill a reservation; nothing happens when the image was deleted meanwhile.
    pub fn insert_decoded(&mut self, id: u32, image: Arc<Image>) -> bool {
        self.images.contains_key(&id) && self.insert(id, image)
    }

    fn evict_oldest(&mut self) -> bool {
        let Some(id) = self.images.iter().min_by_key(|(_, slot)| slot.used).map(|(id, _)| *id) else {
            return false;
        };
        self.remove(id)
    }

    /// Drop the pixels and keep the slot (for a replacement).
    fn remove_pixels(&mut self, id: u32) {
        if let Some(slot) = self.images.get_mut(&id) {
            let bytes = slot.bytes();
            slot.image = None;
            slot.frames.clear();
            self.bytes -= bytes;
            GLOBAL_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        }
    }

    /// Forget the image and its placements (its placeholders then draw nothing).
    pub fn remove(&mut self, id: u32) -> bool {
        let Some(slot) = self.images.remove(&id) else { return false };
        let bytes = slot.bytes();
        self.bytes -= bytes;
        GLOBAL_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        true
    }

    /// The image, counted as drawn now; `None` while it is still being decoded.
    pub fn get(&mut self, id: u32) -> Option<Arc<Image>> {
        let tick = self.touch();
        let slot = self.images.get_mut(&id)?;
        slot.used = tick;
        slot.image.clone()
    }

    /// Whether any image has animation frames: a frame that draws only still pictures skips
    /// everything about animation.
    pub fn any_animated(&self) -> bool {
        self.images.values().any(|slot| !slot.frames.is_empty())
    }

    /// How many frames the image has here, the first included; 0 when there is no such image.
    pub fn frame_count(&self, id: u32) -> usize {
        self.images.get(&id).map_or(0, |slot| 1 + slot.frames.len())
    }

    /// Whether the image is reserved and still being decoded.
    pub fn is_pending(&self, id: u32) -> bool {
        self.images.get(&id).is_some_and(|slot| slot.pending)
    }

    /// Frame `frame` of the image (counted from 0, the picture itself), counted as drawn now.
    /// `None` while the picture is being decoded, and for a frame without pixels.
    pub fn frame(&mut self, id: u32, frame: usize) -> Option<Arc<Image>> {
        let tick = self.touch();
        let slot = self.images.get_mut(&id)?;
        slot.used = tick;
        match frame {
            0 => slot.image.clone(),
            n => slot.frames.get(n - 1)?.clone(),
        }
    }

    /// Frame `frame` of the image, or the newest frame before it that has pixels: what to draw
    /// while a frame has none (counted as drawn now). `None` when none of them does.
    pub fn frame_or_earlier(&mut self, id: u32, frame: usize) -> Option<Arc<Image>> {
        let tick = self.touch();
        let slot = self.images.get_mut(&id)?;
        slot.used = tick;
        let last = slot.frames.len().min(frame);
        (1..=last).rev().find_map(|n| slot.frames[n - 1].clone()).or_else(|| slot.image.clone())
    }

    /// Keep `image` as frame `frame` (counted from 0, the picture itself) of the image, replacing
    /// what was there. Frames before it that were never given pixels stay empty. **No room is made
    /// by evicting**: the engine counts the same canvases against a budget of its own and has made
    /// room by it, and a frame is never paid for with another picture, nor with its own. `false`,
    /// and nothing changed (the old canvas stays), when there is no such image or it does not fit.
    pub fn set_frame(&mut self, id: u32, frame: usize, image: Arc<Image>) -> bool {
        let Some(slot) = self.images.get(&id) else { return false };
        let old = match frame {
            0 => slot.image.as_ref(),
            n => slot.frames.get(n - 1).and_then(Option::as_ref),
        }
        .map_or(0, |old| old.bytes());
        let bytes = image.bytes();
        let grow = bytes.saturating_sub(old);
        if self.bytes + grow > self.budget || global_bytes() + grow > GLOBAL_BUDGET {
            return false;
        }
        let used = self.touch();
        let Some(slot) = self.images.get_mut(&id) else { return false };
        slot.used = used;
        match frame {
            0 => {
                slot.image = Some(image);
                slot.pending = false;
            }
            n => {
                if slot.frames.len() < n {
                    slot.frames.resize(n, None);
                }
                slot.frames[n - 1] = Some(image);
            }
        }
        self.bytes = self.bytes - old + bytes;
        GLOBAL_BYTES.fetch_add(bytes, Ordering::Relaxed);
        GLOBAL_BYTES.fetch_sub(old, Ordering::Relaxed);
        true
    }

    /// Delete frame `frame` (counted from 0) as the engine does: the later frames move up one, and
    /// when it is the first the next one becomes the picture. The only frame stays.
    pub fn remove_frame(&mut self, id: u32, frame: usize) {
        let Some(slot) = self.images.get_mut(&id) else { return };
        let removed = if frame == 0 {
            if slot.frames.is_empty() {
                return;
            }
            let next = slot.frames.remove(0);
            std::mem::replace(&mut slot.image, next)
        } else if frame <= slot.frames.len() {
            slot.frames.remove(frame - 1)
        } else {
            return;
        };
        let freed = removed.map_or(0, |image| image.bytes());
        self.bytes -= freed;
        GLOBAL_BYTES.fetch_sub(freed, Ordering::Relaxed);
    }

    /// Whether the id is taken (an image, decoded or on its way).
    pub fn contains(&self, id: u32) -> bool {
        self.images.contains_key(&id)
    }

    /// Every image's id, in no order.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.images.keys().copied()
    }

    /// The image's size in pixels, known from the moment it is reserved.
    pub fn dimensions(&self, id: u32) -> Option<(u32, u32)> {
        self.images.get(&id).map(|slot| (slot.width, slot.height))
    }
}

impl Drop for ImageStore {
    fn drop(&mut self) {
        GLOBAL_BYTES.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Image, ImageStore};

    fn image(bytes: usize) -> Arc<Image> {
        Arc::new(Image { width: (bytes / 4) as u32, height: 1, rgba: vec![0; bytes] })
    }

    #[test]
    fn the_least_recently_drawn_image_goes_first_when_the_budget_is_full() {
        let mut store = ImageStore::with_budget(3_000);
        let third = 1_000;
        assert!(store.insert(1, image(third)));
        assert!(store.insert(2, image(third)));
        assert!(store.insert(3, image(third)));
        assert_eq!(store.len(), 3);
        // Drawing 1 makes 2 the oldest.
        assert!(store.get(1).is_some());
        assert!(store.insert(4, image(third)));
        assert!(!store.contains(2) && store.contains(1) && store.contains(3));
        assert_eq!(store.bytes(), 3 * third);
        // Too big for the budget at all.
        assert!(!store.insert(5, image(3_001)));
        assert_eq!(store.len(), 3);
        // Replacing an image frees the old one.
        assert!(store.insert(4, image(8)));
        assert_eq!(store.bytes(), 2 * third + 8);
    }

    #[test]
    fn frames_are_kept_with_the_image_and_count_in_the_budget() {
        let mut store = ImageStore::default();
        assert!(store.insert(1, image(16)));
        assert_eq!((store.frame_count(1), store.frame_count(9)), (1, 0));
        assert!(!store.any_animated());
        assert!(store.set_frame(1, 2, image(16)), "a frame may skip ahead; the one before stays empty");
        assert_eq!(store.frame_count(1), 3);
        assert!(store.any_animated());
        assert!(store.frame(1, 1).is_none() && store.frame(1, 2).is_some() && store.frame(1, 0).is_some());
        assert!(store.frame(1, 9).is_none());
        assert_eq!(store.bytes(), 32);
        assert!(store.set_frame(1, 2, image(24)), "replacing a frame frees the old canvas");
        assert_eq!(store.bytes(), 40);
        assert!(store.set_frame(1, 0, image(8)), "frame 0 is the picture itself");
        assert_eq!(store.bytes(), 32);
        assert!(!store.set_frame(5, 1, image(8)), "no such image");
        store.remove(1);
        assert_eq!(store.bytes(), 0);
    }

    #[test]
    fn the_frame_to_draw_is_the_newest_with_pixels_at_or_before_the_one_asked_for() {
        let mut store = ImageStore::default();
        let (root, second) = (image(8), image(16));
        assert!(store.insert(1, root.clone()));
        assert!(store.set_frame(1, 1, second.clone()));
        assert!(store.set_frame(1, 4, image(24)));
        let at = |store: &mut ImageStore, n| store.frame_or_earlier(1, n).map(|i| i.bytes());
        assert_eq!([at(&mut store, 0), at(&mut store, 1)], [Some(8), Some(16)]);
        assert_eq!([at(&mut store, 2), at(&mut store, 3)], [Some(16), Some(16)], "frames 3 and 4 have none");
        assert_eq!([at(&mut store, 4), at(&mut store, 9)], [Some(24), Some(24)]);
        assert_eq!(store.frame_or_earlier(7, 0), None);
        store.reserve(2, 1, 1);
        assert_eq!(store.frame_or_earlier(2, 3), None, "a picture still being decoded has nothing to draw");
    }

    #[test]
    fn a_frame_never_evicts_and_one_that_does_not_fit_leaves_the_old_canvas() {
        let mut store = ImageStore::with_budget(100);
        assert!(store.insert(1, image(36)));
        assert!(store.insert(2, image(16)));
        assert!(store.set_frame(2, 1, image(32)), "36 + 16 + 32 fits");
        assert!(
            !store.set_frame(2, 2, image(32)),
            "it does not, and the older picture 1 is not evicted for it"
        );
        assert!(store.contains(1) && store.contains(2));
        assert_eq!((store.frame_count(2), store.bytes()), (2, 84));
        assert!(!store.set_frame(2, 1, image(100)), "a bigger canvas for a frame does not fit either");
        assert_eq!(store.frame(2, 1).map(|i| i.bytes()), Some(32), "the old canvas is still there");
        assert_eq!(store.bytes(), 84);
        assert!(!store.set_frame(2, 0, image(200)));
        assert_eq!(store.frame(2, 0).map(|i| i.bytes()), Some(16));
    }

    #[test]
    fn deleting_a_frame_moves_the_later_ones_up_and_the_next_becomes_the_picture() {
        let mut store = ImageStore::default();
        assert!(store.insert(1, image(8)));
        assert!(store.set_frame(1, 1, image(16)));
        assert!(store.set_frame(1, 2, image(24)));
        store.remove_frame(1, 1);
        assert_eq!(
            (store.frame_count(1), store.frame(1, 1).map(|i| i.bytes()), store.bytes()),
            (2, Some(24), 32)
        );
        store.remove_frame(1, 0);
        assert_eq!(
            (store.frame_count(1), store.frame(1, 0).map(|i| i.bytes()), store.bytes()),
            (1, Some(24), 24)
        );
        store.remove_frame(1, 0);
        store.remove_frame(1, 5);
        store.remove_frame(9, 0);
        assert_eq!(store.bytes(), 24, "the only frame stays");
    }

    #[test]
    fn a_reserved_image_has_a_size_but_no_pixels_until_it_is_decoded() {
        let mut store = ImageStore::default();
        assert!(store.reserve(7, 30, 20));
        assert!(store.contains(7));
        assert_eq!(store.dimensions(7), Some((30, 20)));
        assert_eq!(store.get(7), None);
        assert!(store.insert_decoded(7, image(16)));
        assert!(store.get(7).is_some());
        // Deleted while decoding: the pixels are not kept.
        store.reserve(8, 1, 1);
        store.remove(8);
        assert!(!store.insert_decoded(8, image(16)));
        assert!(!store.contains(8));
        // Pending is a state of its own, not "has no pixels yet".
        store.reserve(9, 1, 1);
        assert!(store.is_pending(9));
        assert!(store.insert_decoded(9, image(4)));
        assert!(!store.is_pending(9) && !store.is_pending(77));
    }
}
