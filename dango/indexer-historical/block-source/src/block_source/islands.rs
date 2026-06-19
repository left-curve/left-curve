use rangemap::RangeInclusiveSet;

/// Blocks already stored but **above** the contiguous frontier — durable, yet
/// not broadcastable until the gap below them is filled and the frontier
/// reaches them.
///
/// Held as coalesced inclusive ranges (one entry per contiguous stretch), so a
/// live tail of thousands of blocks is a single range and a whole island is
/// crossed in one step. Tracking them in memory is what lets the coordinator
/// advance without probing the store on every block, and bulk-advance across a
/// backlog instead of broadcasting it block by block.
#[derive(Debug, Default)]
pub(crate) struct Islands(RangeInclusiveSet<u64>);

impl Islands {
    /// Seed from what a previous run left in `[lo, hi]`: the present ranges,
    /// i.e. the complement of `gaps` — take the full span and carve the gaps
    /// out of it.
    pub(crate) fn from_gaps(lo: u64, hi: u64, gaps: &[(u64, u64)]) -> Self {
        let mut ranges = RangeInclusiveSet::new();
        ranges.insert(lo..=hi);
        for &(start, end) in gaps {
            ranges.remove(start..=end);
        }
        Self(ranges)
    }

    /// Remember a single stored height, coalescing with any adjacent range.
    pub(crate) fn insert(&mut self, height: u64) {
        self.0.insert(height..=height);
    }

    /// If an island starts exactly at `height`, remove it and return its
    /// inclusive top; otherwise `None`. Used to cross an island once the
    /// frontier reaches its bottom.
    pub(crate) fn take_starting_at(&mut self, height: u64) -> Option<u64> {
        let range = self.0.get(&height)?.clone();
        if *range.start() != height {
            return None;
        }
        let end = *range.end();
        self.0.remove(range);
        Some(end)
    }
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_contiguous_inserts() {
        let mut islands = Islands::default();
        islands.insert(5);
        islands.insert(6);
        islands.insert(7);
        // The three collapse into one range [5, 7].
        assert_eq!(islands.take_starting_at(5), Some(7));
        assert_eq!(islands.take_starting_at(5), None); // consumed
    }

    #[test]
    fn bridges_two_ranges() {
        let mut islands = Islands::default();
        islands.insert(5);
        islands.insert(6);
        islands.insert(7);
        islands.insert(10);
        islands.insert(9);
        islands.insert(8); // bridges [5, 7] and [9, 10]
        assert_eq!(islands.take_starting_at(5), Some(10));
    }

    #[test]
    fn takes_only_at_a_range_start() {
        let mut islands = Islands::default();
        islands.insert(5);
        islands.insert(6);
        // A point inside the range is not its start.
        assert_eq!(islands.take_starting_at(6), None);
        // No island there at all.
        assert_eq!(islands.take_starting_at(8), None);
        assert_eq!(islands.take_starting_at(5), Some(6));
    }

    #[test]
    fn from_gaps_is_the_complement() {
        // Store {1..=50, 200..=210} above frontier 50 → island [200, 210].
        let mut islands = Islands::from_gaps(51, 210, &[(51, 199)]);
        assert_eq!(islands.take_starting_at(200), Some(210));

        // Two stored stretches separated by a gap.
        let mut islands = Islands::from_gaps(51, 210, &[(51, 99), (111, 199)]);
        assert_eq!(islands.take_starting_at(100), Some(110));
        assert_eq!(islands.take_starting_at(200), Some(210));
    }
}
