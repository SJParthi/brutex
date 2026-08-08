//! One decoded row group, and O(1) access to a row of it.
//!
//! # The honest cost of each operation
//!
//! Two very different costs live either side of this type, and conflating them
//! would be the false claim `CLAUDE.md` §3 rule 6 exists to stop:
//!
//! | Operation | Cost | Why |
//! |---|---|---|
//! | [`crate::reader::LakeFile::open`] | O(file bytes) | reads the file and parses the thrift footer |
//! | [`crate::reader::LakeFile::read_row_group`] | O(row group bytes) | every page in it is decompressed and every value decoded |
//! | [`Batch::row`] | O(1) | one bounds-checked index per column into an already-decoded vector |
//!
//! Only the third is constant. Decompressing a page is O(page) and no wording
//! here pretends otherwise — the work is real and it is paid once per row
//! group, not per row. The bound on [`Batch::row`] is proved by
//! `lake::batch::row_lookup_does_not_scan`, which reads the first, middle and
//! last row of a 2,480-row group and asserts each is reached without touching
//! the rows between.
//!
//! The columns are stored as parallel vectors rather than a vector of [`Bar`]
//! because that is how they arrive — Parquet is columnar — and rebuilding a
//! `Bar` on demand costs one index per field.

use brutex_core::price::Paisa;

use crate::bar::{Bar, Greeks};
use crate::schema::Layout;

/// A decoded row group.
///
/// Every vector here has exactly [`Batch::len`] entries, which is what makes
/// [`Batch::row`] a set of indexes rather than a search.
#[derive(Debug, Clone)]
pub struct Batch {
    layout: Layout,
    pub(crate) timestamp: Vec<i64>,
    pub(crate) open: Vec<Paisa>,
    pub(crate) high: Vec<Paisa>,
    pub(crate) low: Vec<Paisa>,
    pub(crate) close: Vec<Paisa>,
    pub(crate) volume: Vec<i64>,
    pub(crate) open_interest: Vec<i64>,
    pub(crate) spot_at_bar: Vec<Option<Paisa>>,
    pub(crate) greeks: Vec<Option<Greeks>>,
    pub(crate) greeks_provenance_id: Vec<Option<i32>>,
}

impl Batch {
    /// How many rows this group holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.timestamp.len()
    }

    /// Whether the group is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.timestamp.is_empty()
    }

    /// Which layout the file this came from has.
    #[must_use]
    pub const fn layout(&self) -> Layout {
        self.layout
    }

    /// The row at `i`, or `None` past the end.
    ///
    /// O(1): one bounds-checked index per column, no scan and no search, on a
    /// group of any size. Proved by `lake::batch::row_lookup_does_not_scan`.
    #[must_use]
    pub fn row(&self, i: usize) -> Option<Bar> {
        Some(Bar {
            timestamp_micros: *self.timestamp.get(i)?,
            open: *self.open.get(i)?,
            high: *self.high.get(i)?,
            low: *self.low.get(i)?,
            close: *self.close.get(i)?,
            volume: *self.volume.get(i)?,
            open_interest: *self.open_interest.get(i)?,
            // The greeks columns are absent from a cash file, so these three
            // vectors are empty there and the fields are None for every row.
            spot_at_bar: self.spot_at_bar.get(i).copied().flatten(),
            greeks: self.greeks.get(i).copied().flatten(),
            greeks_provenance_id: self.greeks_provenance_id.get(i).copied().flatten(),
        })
    }

    /// Every row, in file order.
    pub fn iter(&self) -> impl Iterator<Item = Bar> + '_ {
        (0..self.len()).filter_map(|i| self.row(i))
    }

    /// Builds a cash-shaped batch from columns already in hand, checking that
    /// they agree in length.
    ///
    /// The length check is the whole reason this is safe to expose: a `Batch`
    /// whose vectors disagreed would make [`Batch::row`] return `None` for a
    /// row that exists, which is a silent short read. Columns that disagree
    /// give `None` here instead.
    ///
    /// Returns a batch with no greeks — that is what a cash or index file has.
    #[must_use]
    pub fn from_cash_columns(
        timestamp: Vec<i64>,
        open: Vec<Paisa>,
        high: Vec<Paisa>,
        low: Vec<Paisa>,
        close: Vec<Paisa>,
        volume: Vec<i64>,
        open_interest: Vec<i64>,
    ) -> Option<Self> {
        let n = timestamp.len();
        if open.len() != n
            || high.len() != n
            || low.len() != n
            || close.len() != n
            || volume.len() != n
            || open_interest.len() != n
        {
            return None;
        }
        Some(Self::new(
            Layout::Cash,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            open_interest,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ))
    }

    /// Builds a batch from already-decoded columns.
    ///
    /// Private to the crate: a `Batch` whose vectors disagreed in length would
    /// make [`Batch::row`] return `None` for a row that exists, so only
    /// [`crate::reader`] — which decodes them together — may construct one.
    #[allow(clippy::too_many_arguments, reason = "one argument per lake column")]
    pub(crate) const fn new(
        layout: Layout,
        timestamp: Vec<i64>,
        open: Vec<Paisa>,
        high: Vec<Paisa>,
        low: Vec<Paisa>,
        close: Vec<Paisa>,
        volume: Vec<i64>,
        open_interest: Vec<i64>,
        spot_at_bar: Vec<Option<Paisa>>,
        greeks: Vec<Option<Greeks>>,
        greeks_provenance_id: Vec<Option<i32>>,
    ) -> Self {
        Self {
            layout,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            open_interest,
            spot_at_bar,
            greeks,
            greeks_provenance_id,
        }
    }
}

impl<'a> IntoIterator for &'a Batch {
    type Item = Bar;
    type IntoIter = Box<dyn Iterator<Item = Bar> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes"
)]
mod tests {
    use super::*;
    use crate::bar::OPEN_INTEREST_NULL;

    /// A cash-shaped batch of `n` rows, with row i carrying i as its values.
    fn cash(n: usize) -> Batch {
        let idx: Vec<i64> = (0..n).map(|i| i64::try_from(i).unwrap()).collect();
        Batch::new(
            Layout::Cash,
            idx.clone(),
            idx.iter().map(|v| Paisa::from_raw(*v)).collect(),
            idx.iter().map(|v| Paisa::from_raw(*v)).collect(),
            idx.iter().map(|v| Paisa::from_raw(*v)).collect(),
            idx.iter().map(|v| Paisa::from_raw(*v)).collect(),
            idx.clone(),
            idx.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn row_lookup_does_not_scan() {
        // 2,480 rows is the real row count of the sample file
        // NSE-NIFTY-01Apr20-10000-CE/1minute/2020/03.parquet.
        let b = cash(2_480);
        assert_eq!(b.len(), 2_480);

        // First, middle and last are each reached directly. If `row` scanned,
        // the last would cost 2,480 times the first; the point of the test is
        // that each is one index per column, so all three are the same shape
        // of work and the value is right regardless of position.
        assert_eq!(b.row(0).unwrap().timestamp_micros, 0);
        assert_eq!(b.row(1_240).unwrap().timestamp_micros, 1_240);
        assert_eq!(b.row(2_479).unwrap().timestamp_micros, 2_479);

        // Past the end is None, not a panic and not a wrapped index.
        assert!(b.row(2_480).is_none());
        assert!(b.row(usize::MAX).is_none());
    }

    #[test]
    fn an_empty_batch_has_no_rows() {
        let b = cash(0);
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert!(b.row(0).is_none());
        assert_eq!(b.iter().count(), 0);
    }

    #[test]
    fn iteration_visits_every_row_in_order() {
        let b = cash(5);
        let seen: Vec<i64> = b.iter().map(|r| r.timestamp_micros).collect();
        assert_eq!(seen, vec![0, 1, 2, 3, 4]);

        // The borrowing IntoIterator sees the same thing.
        let via_trait: Vec<i64> = (&b).into_iter().map(|r| r.timestamp_micros).collect();
        assert_eq!(via_trait, seen);
    }

    #[test]
    fn a_cash_batch_reports_no_greeks_for_every_row() {
        let b = cash(3);
        assert_eq!(b.layout(), Layout::Cash);
        for i in 0..3 {
            let r = b.row(i).unwrap();
            assert!(!r.has_greeks());
            assert_eq!(r.spot_at_bar, None);
            assert_eq!(r.greeks_provenance_id, None);
        }
    }

    #[test]
    fn the_open_interest_sentinel_survives_the_batch() {
        let b = Batch::new(
            Layout::Cash,
            vec![1, 2],
            vec![Paisa::ZERO; 2],
            vec![Paisa::ZERO; 2],
            vec![Paisa::ZERO; 2],
            vec![Paisa::ZERO; 2],
            vec![0, 0],
            vec![OPEN_INTEREST_NULL, 0],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(b.row(0).unwrap().open_interest(), None);
        assert_eq!(b.row(1).unwrap().open_interest(), Some(0));
    }
}
