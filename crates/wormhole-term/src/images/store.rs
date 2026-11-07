//! A terminal's images, under a budget.
//!
//! Each terminal keeps its decoded images here, least recently drawn first to go, within a
//! budget of its own ([`STORE_BUDGET`] unless the terminal was started with another, always within
//! [`STORE_BUDGET_MIN`]..=[`STORE_BUDGET_MAX`]) and a budget across every terminal of the process
//! that follows from it ([`global_budget_for`]). An image that doesn't fit after evicting
//! everything else of this terminal is dropped (another terminal holds the rest of the global
//! budget). An image is reserved with its size while it is
//! being decoded, so a placement can already refer to it. Where a picture is shown is the
//! engine's (placements in its grid, virtual placements); this holds the pixels, by the key the
//! host gave each picture.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::MAX_IMAGES;

/// Most one terminal's images may take, decoded, unless it was started with another budget.
pub const STORE_BUDGET: usize = 128 * MIB;
/// The least a terminal may be given: room for a few screenshots.
pub const STORE_BUDGET_MIN: usize = 64 * MIB;
/// The most a terminal may be given, whatever is asked: a hard ceiling for a value from a
/// settings file, which anyone may edit and a cloned repository may try to.
pub const STORE_BUDGET_MAX: usize = 1024 * MIB;
/// How many terminals' worth of images all terminals may hold together.
const GLOBAL_BUDGET_TERMINALS: usize = 4;
/// Most all terminals' images may take together, whatever the terminals' budgets.
pub const GLOBAL_BUDGET_MAX: usize = 2048 * MIB;
/// Most all terminals' images may take together at the default budget per terminal.
pub const GLOBAL_BUDGET: usize = global_budget_for(STORE_BUDGET);

const MIB: usize = 1024 * 1024;

/// `bytes` brought into the range a terminal's budget may have.
pub const fn clamp_budget(bytes: usize) -> usize {
    if bytes < STORE_BUDGET_MIN {
        STORE_BUDGET_MIN
    } else if bytes > STORE_BUDGET_MAX {
        STORE_BUDGET_MAX
    } else {
        bytes
    }
}

/// What all terminals may hold together for a terminal budget of `store`: four terminals' worth,
/// never more than [`GLOBAL_BUDGET_MAX`]. Each store applies the figure of its own budget to the
/// process's total, so a terminal started with a small budget also refuses to add to a large
/// total.
pub const fn global_budget_for(store: usize) -> usize {
    let budget = clamp_budget(store) * GLOBAL_BUDGET_TERMINALS;
    if budget > GLOBAL_BUDGET_MAX { GLOBAL_BUDGET_MAX } else { budget }
}

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

pub struct ImageStore {
    images: HashMap<u32, Slot>,
    bytes: usize,
    tick: u64,
    /// Most this terminal's images may take; within [`STORE_BUDGET_MIN`]..=[`STORE_BUDGET_MAX`].
    budget: usize,
}

impl Default for ImageStore {
    fn default() -> Self {
        Self { images: HashMap::new(), bytes: 0, tick: 0, budget: STORE_BUDGET }
    }
}

impl ImageStore {
    /// Give this terminal another budget, clamped into its range. A store that holds more than the
    /// new budget keeps it until the next image arrives, which evicts the least recently drawn
    /// until it fits.
    pub fn set_budget(&mut self, bytes: usize) {
        self.budget = clamp_budget(bytes);
    }

    /// Most this terminal's images may take.
    pub fn budget(&self) -> usize {
        self.budget
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
        let global = global_budget_for(self.budget);
        if bytes > self.budget {
            return false;
        }
        while self.bytes + bytes > self.budget
            || global_bytes() + bytes > global
            || self.images.len() >= MAX_IMAGES
        {
            if !self.evict_oldest() {
                break;
            }
        }
        self.bytes + bytes <= self.budget && global_bytes() + bytes <= global
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
        true
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

    use super::{
        GLOBAL_BUDGET, GLOBAL_BUDGET_MAX, Image, ImageStore, STORE_BUDGET, STORE_BUDGET_MAX,
        STORE_BUDGET_MIN, clamp_budget, global_budget_for,
    };

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
    fn a_budget_asked_for_stays_within_its_range() {
        // A settings file is hostile input: nothing it says gets past the ceiling or under the floor.
        for (asked, kept) in [
            (0, STORE_BUDGET_MIN),
            (STORE_BUDGET_MIN - 1, STORE_BUDGET_MIN),
            (200 * 1024 * 1024, 200 * 1024 * 1024),
            (STORE_BUDGET_MAX, STORE_BUDGET_MAX),
            (STORE_BUDGET_MAX + 1, STORE_BUDGET_MAX),
            (usize::MAX, STORE_BUDGET_MAX),
        ] {
            assert_eq!(clamp_budget(asked), kept);
            let mut store = ImageStore::default();
            store.set_budget(asked);
            assert_eq!(store.budget(), kept);
        }
        assert_eq!(ImageStore::default().budget(), STORE_BUDGET);
    }

    #[test]
    fn all_terminals_together_get_four_budgets_up_to_a_ceiling() {
        assert_eq!(GLOBAL_BUDGET, 4 * STORE_BUDGET, "the default keeps its 512 MiB");
        assert_eq!(global_budget_for(STORE_BUDGET_MIN), 4 * STORE_BUDGET_MIN);
        assert_eq!(global_budget_for(STORE_BUDGET_MAX), GLOBAL_BUDGET_MAX);
        assert_eq!(global_budget_for(usize::MAX), GLOBAL_BUDGET_MAX);
        const { assert!(GLOBAL_BUDGET_MAX < 4 * STORE_BUDGET_MAX, "the ceiling is what caps the largest") };
    }

    #[test]
    fn a_smaller_budget_evicts_instead_of_growing() {
        let mut store = ImageStore::default();
        store.set_budget(STORE_BUDGET_MIN);
        let quarter = STORE_BUDGET_MIN / 4;
        for id in 1..=4 {
            assert!(store.insert(id, image(quarter)));
        }
        assert_eq!(store.bytes(), STORE_BUDGET_MIN);
        // One more evicts the oldest; the total never passes the budget.
        assert!(store.insert(5, image(quarter)));
        assert!(!store.contains(1) && store.contains(5));
        assert_eq!(store.bytes(), STORE_BUDGET_MIN);
        // Bigger than the budget, though it would fit the default one: refused, nothing evicted.
        assert!(!store.insert(6, image(STORE_BUDGET_MIN + 4)));
        assert_eq!(store.len(), 4);
        assert_eq!(store.bytes(), STORE_BUDGET_MIN);
    }

    #[test]
    fn a_store_given_a_smaller_budget_makes_room_with_its_next_image() {
        let mut store = ImageStore::default();
        let half = STORE_BUDGET / 2;
        assert!(store.insert(1, image(half)));
        assert!(store.insert(2, image(half)));
        store.set_budget(STORE_BUDGET_MIN);
        assert_eq!(store.bytes(), STORE_BUDGET, "nothing is dropped until something arrives");
        assert!(store.insert(3, image(8)));
        assert_eq!(store.len(), 1);
        assert!(store.bytes() <= STORE_BUDGET_MIN);
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
