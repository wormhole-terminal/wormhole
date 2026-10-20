//! The pictures a screen knows: what the engine keeps of them (the host has the pixels).
//!
//! Each screen has its own table, as kitty has it: ids and numbers mean something on one screen
//! only. An entry is small (sizes and ids, no pixels); the table is bounded by a count and by
//! the bytes the pictures take decoded, and what does not fit evicts the least recently used
//! picture, one nothing is placed from first.

use crate::types::ImageHandle;

use super::upload::VirtualPlacement;

/// Most pictures one screen keeps, whatever their size.
pub const MAX_IMAGES: usize = 1024;
/// Most bytes (RGBA, decoded) the pictures of one screen may take together.
pub const IMAGE_BUDGET_BYTES: u64 = 128 * 1024 * 1024;

/// One picture.
#[derive(Clone, Debug)]
pub(super) struct Image {
    pub handle: ImageHandle,
    /// The id the program gave it, or the terminal gave a numbered one; 0 for none.
    pub id: u32,
    /// The number the program asked for (kitty's `I`), 0 for none.
    pub number: u32,
    pub width: u32,
    pub height: u32,
    /// The tick of the last time it was made or placed.
    pub used: u64,
    pub virtuals: Vec<VirtualPlacement>,
}

impl Image {
    /// Bytes it takes decoded, as RGBA.
    fn cost(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4
    }
}

/// The pictures of one screen, oldest first.
#[derive(Debug, Default)]
pub(super) struct ImageTable {
    list: Vec<Image>,
    bytes: u64,
    tick: u64,
}

impl ImageTable {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Image> {
        self.list.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Image> {
        self.list.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Image> {
        self.list.get_mut(index)
    }

    pub fn find(&self, handle: ImageHandle) -> Option<usize> {
        self.list.iter().position(|image| image.handle == handle)
    }

    pub fn by_id(&self, id: u32) -> Option<usize> {
        (id != 0).then(|| self.list.iter().position(|image| image.id == id)).flatten()
    }

    /// The newest picture that was sent with this number.
    pub fn by_number(&self, number: u32) -> Option<usize> {
        (number != 0).then(|| self.list.iter().rposition(|image| image.number == number)).flatten()
    }

    /// The smallest id above 0 that no picture has: what a picture sent by number is given.
    pub fn free_id(&self) -> u32 {
        let mut ids: Vec<u32> = self.list.iter().map(|image| image.id).filter(|&id| id != 0).collect();
        ids.sort_unstable();
        let mut next = 1;
        for id in ids {
            if id == next {
                next += 1;
            } else if id > next {
                break;
            }
        }
        next
    }

    /// Whether a picture of `cost` bytes needs one to leave first, with at most `count` pictures
    /// and `budget` bytes kept (the host's limits, never above `MAX_IMAGES` and
    /// `IMAGE_BUDGET_BYTES`).
    pub fn is_full(&self, cost: u64, count: usize, budget: u64) -> bool {
        self.list.len() >= count || self.bytes + cost > budget
    }

    /// Whether the table holds more than `count` pictures or `budget` bytes: the limits were
    /// lowered under it.
    pub fn is_over(&self, count: usize, budget: u64) -> bool {
        self.list.len() > count || self.bytes > budget
    }

    /// Add a picture. The caller made room (`is_full`).
    pub fn insert(&mut self, mut image: Image) {
        self.tick += 1;
        image.used = self.tick;
        self.bytes += image.cost();
        self.list.push(image);
    }

    pub fn remove(&mut self, index: usize) -> Image {
        let image = self.list.remove(index);
        self.bytes = self.bytes.saturating_sub(image.cost());
        image
    }

    /// Note a use, so it is evicted last.
    pub fn touch(&mut self, index: usize) {
        self.tick += 1;
        if let Some(image) = self.list.get_mut(index) {
            image.used = self.tick;
        }
    }

    /// The picture to evict: the least recently used among those `in_use` says nothing is placed
    /// from, else the least recently used of all.
    pub fn victim(&self, in_use: impl Fn(&Image) -> bool) -> Option<usize> {
        let oldest = |keep: &dyn Fn(&Image) -> bool| {
            self.list.iter().enumerate().filter(|(_, image)| keep(image)).min_by_key(|(_, image)| image.used)
        };
        oldest(&|image| !in_use(image)).or_else(|| oldest(&|_| true)).map(|(index, _)| index)
    }

    /// The table's own bookkeeping, for [`Terminal::check_invariants`](crate::terminal::Terminal::check_invariants).
    #[cfg(any(test, feature = "internals"))]
    pub fn check(&self, max_images: usize, max_bytes: u64) -> Result<(), String> {
        if self.list.len() > max_images {
            return Err(format!("{} pictures, at most {max_images} allowed", self.list.len()));
        }
        let sum: u64 = self.list.iter().map(Image::cost).sum();
        if sum != self.bytes {
            return Err(format!("the pictures take {sum} bytes, the table says {}", self.bytes));
        }
        if sum > max_bytes {
            return Err(format!("the pictures take {sum} bytes, at most {max_bytes} allowed"));
        }
        for (i, image) in self.list.iter().enumerate() {
            if self.list[..i].iter().any(|other| other.handle == image.handle) {
                return Err(format!("handle {:?} is used twice", image.handle));
            }
            if image.id != 0 && self.list[..i].iter().any(|other| other.id == image.id) {
                return Err(format!("picture id {} is used twice", image.id));
            }
        }
        Ok(())
    }

    pub fn drain(&mut self) -> Vec<Image> {
        self.bytes = 0;
        std::mem::take(&mut self.list)
    }
}
