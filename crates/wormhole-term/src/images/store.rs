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

/// Most one terminal's images may take, decoded.
pub const STORE_BUDGET: usize = 128 * 1024 * 1024;
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
    /// a frame the engine counts that has no pixels here, because its data could not be decoded
    /// or was dropped under load; it shows nothing.
    frames: Vec<Option<Arc<Image>>>,
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

#[derive(Default)]
pub struct ImageStore {
    images: HashMap<u32, Slot>,
    bytes: usize,
    tick: u64,
}

impl ImageStore {
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
        if bytes > STORE_BUDGET {
            return false;
        }
        while self.bytes + bytes > STORE_BUDGET
            || global_bytes() + bytes > GLOBAL_BUDGET
            || self.images.len() >= MAX_IMAGES
        {
            if !self.evict_oldest() {
                break;
            }
        }
        self.bytes + bytes <= STORE_BUDGET && global_bytes() + bytes <= GLOBAL_BUDGET
    }

    /// An image of this size is on its way (being decoded): its id is taken and placements may
    /// refer to it. `false` when the store is full of images.
    pub fn reserve(&mut self, id: u32, width: u32, height: u32) -> bool {
        self.remove(id);
        if !self.make_room(0) {
            return false;
        }
        let used = self.touch();
        self.images.insert(id, Slot { image: None, frames: Vec::new(), width, height, used });
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
        self.images.insert(id, Slot { image: Some(image), frames: Vec::new(), width, height, used });
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
        self.images.get(&id).is_some_and(|slot| slot.image.is_none())
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

    /// Keep `image` as frame `frame` (counted from 0, the picture itself) of the image, replacing
    /// what was there and making room as `insert` does. Frames before it that were never given
    /// pixels stay empty. `false` when there is no such image or the frame doesn't fit even then.
    pub fn set_frame(&mut self, id: u32, frame: usize, image: Arc<Image>) -> bool {
        if !self.images.contains_key(&id) {
            return false;
        }
        let bytes = image.bytes();
        // The old canvas goes first, so replacing a frame does not need room for both.
        if let Some(slot) = self.images.get_mut(&id) {
            let old = match frame {
                0 => slot.image.take(),
                n => slot.frames.get_mut(n - 1).and_then(Option::take),
            };
            if let Some(old) = old {
                self.bytes -= old.bytes();
                GLOBAL_BYTES.fetch_sub(old.bytes(), Ordering::Relaxed);
            }
        }
        // Newest, so that making room takes other images first.
        let used = self.touch();
        if let Some(slot) = self.images.get_mut(&id) {
            slot.used = used;
        }
        if !self.make_room(bytes) {
            return false;
        }
        let Some(slot) = self.images.get_mut(&id) else { return false };
        match frame {
            0 => slot.image = Some(image),
            n => {
                if slot.frames.len() < n {
                    slot.frames.resize(n, None);
                }
                slot.frames[n - 1] = Some(image);
            }
        }
        GLOBAL_BYTES.fetch_add(bytes, Ordering::Relaxed);
        self.bytes += bytes;
        true
    }

    /// Drop all the frames after the first.
    pub fn clear_frames(&mut self, id: u32) {
        let Some(slot) = self.images.get_mut(&id) else { return };
        let freed: usize = slot.frames.drain(..).flatten().map(|frame| frame.bytes()).sum();
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

    use super::{Image, ImageStore, STORE_BUDGET};

    fn image(bytes: usize) -> Arc<Image> {
        Arc::new(Image { width: (bytes / 4) as u32, height: 1, rgba: vec![0; bytes] })
    }

    #[test]
    fn the_least_recently_drawn_image_goes_first_when_the_budget_is_full() {
        let mut store = ImageStore::default();
        let third = STORE_BUDGET / 3;
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
        assert!(!store.insert(5, image(STORE_BUDGET + 1)));
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
        store.clear_frames(1);
        assert_eq!((store.frame_count(1), store.bytes()), (1, 16));
        assert!(store.set_frame(1, 0, image(8)), "frame 0 is the picture itself");
        assert_eq!(store.bytes(), 8);
        assert!(!store.set_frame(5, 1, image(8)), "no such image");
        // Removing the image takes its frames' bytes along.
        assert!(store.set_frame(1, 1, image(8)));
        assert_eq!(store.bytes(), 16);
        store.remove(1);
        assert_eq!(store.bytes(), 0);
    }

    #[test]
    fn a_frame_makes_room_by_evicting_other_images_and_never_its_own() {
        let mut store = ImageStore::default();
        let half = STORE_BUDGET / 2;
        assert!(store.insert(1, image(half)));
        assert!(store.insert(2, image(half)));
        assert!(store.set_frame(2, 1, image(half)), "image 1, the older, makes room");
        assert!(!store.contains(1) && store.contains(2));
        assert!(!store.set_frame(2, 2, image(half + 1)), "more than the budget has");
        assert!(store.contains(2) || store.is_empty());
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
    }
}
