//! A terminal's images, under a budget.
//!
//! Each terminal keeps its decoded images here, least recently drawn first to go, within
//! [`STORE_BUDGET`] of its own and [`GLOBAL_BUDGET`] across every terminal of the process. An
//! image that doesn't fit after evicting everything else of this terminal is dropped (another
//! terminal holds the rest of the global budget). An image is reserved with its size while it is
//! being decoded, so a placement can already refer to it. Placements are the grid's placeholders;
//! the store remembers each one's shape (how many cells, whether the picture is fitted inside,
//! which part of the image) and which ones a program deleted (`hide`), since the cells stay.

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
#[derive(Debug, PartialEq, Eq)]
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

/// How a placement shows its image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementSpec {
    /// The cells the placement takes.
    pub cols: u16,
    pub rows: u16,
    /// The picture keeps its aspect ratio inside those cells (letterboxed), rather than
    /// filling them.
    pub fit: bool,
    /// The part of the image shown, in pixels (`x, y, width, height`); the whole image if `None`.
    pub source: Option<(u32, u32, u32, u32)>,
}

struct Slot {
    /// `None` while the image is being decoded.
    image: Option<Arc<Image>>,
    width: u32,
    height: u32,
    /// The tick it was last drawn or stored at (the eviction order).
    used: u64,
}

impl Slot {
    fn bytes(&self) -> usize {
        self.image.as_ref().map_or(0, |image| image.bytes())
    }
}

/// What a program deleted of an image's placements.
#[derive(Debug, Default)]
struct Hidden {
    all: bool,
    placements: Vec<u32>,
}

#[derive(Default)]
pub struct ImageStore {
    images: HashMap<u32, Slot>,
    /// kitty's image numbers (`I=`), each to the newest id made for it.
    numbers: HashMap<u32, u32>,
    placements: HashMap<(u32, u32), PlacementSpec>,
    hidden: HashMap<u32, Hidden>,
    bytes: usize,
    tick: u64,
    /// Bumped by every change: a view compares it to know whether to look again.
    generation: u64,
    /// The ids this terminal hands out, from the top down.
    next_id: u32,
}

impl ImageStore {
    pub fn generation(&self) -> u64 {
        self.generation
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
        self.images.insert(id, Slot { image: None, width, height, used });
        self.generation += 1;
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
        self.images.insert(id, Slot { image: Some(image), width, height, used });
        self.generation += 1;
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
        self.hidden.remove(&id);
        self.placements.retain(|(image, _), _| *image != id);
        self.generation += 1;
        true
    }

    /// Forget every image.
    pub fn clear(&mut self) {
        for id in self.images.keys().copied().collect::<Vec<_>>() {
            self.remove(id);
        }
    }

    /// The image, counted as drawn now; `None` while it is still being decoded.
    pub fn get(&mut self, id: u32) -> Option<Arc<Image>> {
        let tick = self.touch();
        let slot = self.images.get_mut(&id)?;
        slot.used = tick;
        slot.image.clone()
    }

    /// Whether the id is taken (an image, decoded or on its way).
    pub fn contains(&self, id: u32) -> bool {
        self.images.contains_key(&id)
    }

    /// Every image's id, in no order.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.images.keys().copied()
    }

    /// The placements of record for an image: (placement id, its shape).
    pub fn placements_of(&self, image: u32) -> impl Iterator<Item = (u32, PlacementSpec)> + '_ {
        self.placements.iter().filter(move |((i, _), _)| *i == image).map(|((_, p), spec)| (*p, *spec))
    }

    /// The image's size in pixels, known from the moment it is reserved.
    pub fn dimensions(&self, id: u32) -> Option<(u32, u32)> {
        self.images.get(&id).map(|slot| (slot.width, slot.height))
    }

    /// An id of this terminal's own, from the top of the range down (a program's ids are
    /// small), for an image the program numbered (`I=`) or left nameless.
    pub fn fresh_id(&mut self) -> u32 {
        loop {
            let id = if self.next_id == 0 { u32::MAX } else { self.next_id };
            self.next_id = id.wrapping_sub(1);
            if self.next_id < 1 << 31 {
                self.next_id = 0;
            }
            if !self.images.contains_key(&id) {
                return id;
            }
        }
    }

    /// A fresh id for an image a program numbered: the newest one by that number from now on.
    pub fn assign_number(&mut self, number: u32) -> u32 {
        let id = self.fresh_id();
        self.numbers.insert(number, id);
        id
    }

    /// The newest image made for `number`.
    pub fn id_for_number(&self, number: u32) -> Option<u32> {
        self.numbers.get(&number).copied()
    }

    /// A program placed the image: the placement's shape, and it shows (again).
    pub fn place(&mut self, image: u32, placement: u32, spec: PlacementSpec) {
        self.placements.insert((image, placement), spec);
        if let Some(hidden) = self.hidden.get_mut(&image) {
            hidden.all = false;
            hidden.placements.retain(|p| *p != placement);
            if hidden.placements.is_empty() {
                self.hidden.remove(&image);
            }
        }
        self.generation += 1;
    }

    /// How a placement shows its image, when the program said.
    pub fn placement(&self, image: u32, placement: u32) -> Option<PlacementSpec> {
        self.placements.get(&(image, placement)).copied()
    }

    /// A program deleted the image's placements: `None` all of them, else the one.
    pub fn hide(&mut self, image: u32, placement: Option<u32>) {
        let hidden = self.hidden.entry(image).or_default();
        match placement {
            None => hidden.all = true,
            Some(placement) if !hidden.placements.contains(&placement) => hidden.placements.push(placement),
            Some(_) => {}
        }
        self.generation += 1;
    }

    /// A program deleted every placement there is.
    pub fn hide_all(&mut self) {
        for id in self.images.keys().copied().collect::<Vec<_>>() {
            self.hidden.entry(id).or_default().all = true;
        }
        self.generation += 1;
    }

    /// Whether a placeholder of `image` with this placement id is still to be drawn.
    pub fn is_shown(&self, image: u32, placement: u32) -> bool {
        match self.hidden.get(&image) {
            None => true,
            Some(hidden) => !hidden.all && !hidden.placements.contains(&placement),
        }
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

    use super::{Image, ImageStore, PlacementSpec, STORE_BUDGET};

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
        let generation = store.generation();
        assert!(store.insert(4, image(8)));
        assert_eq!(store.bytes(), 2 * third + 8);
        assert!(store.generation() > generation);
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

    #[test]
    fn deleted_placements_stay_hidden_until_the_image_is_placed_again() {
        let mut store = ImageStore::default();
        store.insert(7, image(16));
        let spec = PlacementSpec { cols: 2, rows: 1, fit: false, source: None };
        store.place(7, 1, spec);
        assert_eq!(store.placement(7, 1), Some(spec));
        assert!(store.is_shown(7, 1) && store.is_shown(7, 2));
        store.hide(7, Some(1));
        assert!(!store.is_shown(7, 1) && store.is_shown(7, 2));
        store.hide(7, None);
        assert!(!store.is_shown(7, 2));
        store.place(7, 2, spec);
        assert!(store.is_shown(7, 2) && !store.is_shown(7, 1), "the old placement stays deleted");
        store.hide_all();
        assert!(!store.is_shown(7, 2));
        assert!(store.is_shown(8, 1), "an unknown image draws nothing anyway");
        store.remove(7);
        assert!(store.is_shown(7, 1), "forgotten with the image");
        assert_eq!(store.placement(7, 1), None);
    }

    #[test]
    fn numbered_and_nameless_images_get_ids_from_the_top() {
        let mut store = ImageStore::default();
        let a = store.assign_number(3);
        let b = store.assign_number(3);
        assert_eq!((a, b), (u32::MAX, u32::MAX - 1));
        assert_eq!(store.id_for_number(3), Some(b), "the newest wins");
        assert_eq!(store.id_for_number(4), None);
        assert_eq!(store.fresh_id(), u32::MAX - 2);
        store.clear();
        assert!(store.is_empty());
    }
}
