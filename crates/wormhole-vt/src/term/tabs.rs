//! Horizontal tab stops: HTS, TBC, DECST8C, HT, CHT and CBT.

/// The columns that stop a tab, one flag per column.
#[derive(Clone, Debug)]
pub(super) struct TabStops {
    stops: Vec<bool>,
}

impl TabStops {
    /// A stop every eighth column (9, 17, … counting from 1), as after DECST8C.
    pub(super) fn new(cols: usize) -> TabStops {
        let mut tabs = TabStops { stops: Vec::new() };
        tabs.reset(cols);
        tabs
    }

    /// DECST8C and RIS: every eighth column, for `cols` columns.
    pub(super) fn reset(&mut self, cols: usize) {
        self.stops.clear();
        self.stops.extend((0..cols).map(|x| x % 8 == 0 && x > 0));
    }

    /// A resize: columns that come keep the default stops, columns that go take theirs along.
    pub(super) fn resize(&mut self, cols: usize) {
        let old = self.stops.len();
        self.stops.truncate(cols);
        self.stops.extend((old..cols).map(|x| x % 8 == 0 && x > 0));
    }

    /// How many columns it has a slot for.
    pub(super) fn len(&self) -> usize {
        self.stops.len()
    }

    /// HTS.
    pub(super) fn set(&mut self, x: usize) {
        if let Some(stop) = self.stops.get_mut(x) {
            *stop = true;
        }
    }

    /// TBC 0.
    pub(super) fn clear(&mut self, x: usize) {
        if let Some(stop) = self.stops.get_mut(x) {
            *stop = false;
        }
    }

    /// TBC 3.
    pub(super) fn clear_all(&mut self) {
        self.stops.fill(false);
    }

    /// The columns that have a stop, from 0, in order.
    pub(super) fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.stops.iter().enumerate().filter(|&(_, &stop)| stop).map(|(x, _)| x)
    }

    /// The first stop after `x`, or `limit` when there is none before it.
    pub(super) fn next(&self, x: usize, limit: usize) -> usize {
        let end = limit.min(self.stops.len().saturating_sub(1));
        (x + 1..=end).find(|&at| self.stops[at]).unwrap_or(limit)
    }

    /// The last stop before `x`, or `floor` when there is none after it.
    pub(super) fn prev(&self, x: usize, floor: usize) -> usize {
        (floor + 1..x.min(self.stops.len())).rev().find(|&at| self.stops[at]).unwrap_or(floor)
    }
}

#[cfg(test)]
mod tests {
    use super::TabStops;

    #[test]
    fn stops_every_eighth_column_and_a_limit() {
        let tabs = TabStops::new(20);
        assert_eq!(tabs.next(0, 19), 8);
        assert_eq!(tabs.next(8, 19), 16);
        assert_eq!(tabs.next(16, 19), 19);
        assert_eq!(tabs.prev(12, 0), 8);
        assert_eq!(tabs.prev(8, 0), 0);
    }

    #[test]
    fn growing_adds_the_default_stops() {
        let mut tabs = TabStops::new(10);
        tabs.clear_all();
        tabs.resize(20);
        assert_eq!(tabs.next(0, 19), 16, "only the new columns have stops");
    }
}
