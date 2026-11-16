//! CHANGE C8 (S17, D13): the shared-volume LEDGER of our live orders on one instrument at one level (scope: account, instrument, level;
//! transient per instrument-day). CHANGE C10 (crate v2.1): each live order carries its own block length B (the tape's: 2 s by default,
//! 1 s for REQ 73b), so its takes are keyed on its own B-second lattice; the reading below is unchanged at B = 2 (crate v2) and at B = 1
//! a block has one volume second. Our take from block [a, a + 2 s) of a live order is charged to BOTH of its volume seconds (a clipped
//! block: only its seconds before the stop, S9); the charge on a second s is read from each LIVE order's own lattice; any other order's
//! block capacity is then its share of the block's volume minus the LARGEST charge on its volume seconds, never below 0, so the total is
//! never above the level's share of the block. O(live orders) a block (each a binary search of its own filled blocks): at most 3 live
//! orders by S13 (an entry's rest, a still-working exit, the square-off). Nothing overflows: no refusal. Overlaps between orders of
//! different trades are counted by the host; a two-phase host finds them by `FillPath::span` and re-prices by [`crate::walk_ledger`].

use crate::tape::Block;
use crate::{BLOCK_MAX_S, BLOCK_US, FillErr, MAXL, US_S, Units, Us};

/// One live order in the ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Live {
    id: u32,
    anchor: Us,
    stop: Us,
    /// CHANGE C10: the order's block length (microseconds, whole seconds 1..=60; [`BLOCK_US`] for [`Ledger::open`]).
    blk: Us,
    open: bool,
    /// Per GLOBAL level: (block number on the order's lattice, units taken), in time order.
    takes: [Vec<(u32, Units)>; MAXL],
}

/// The shared-volume ledger of one (account, instrument) for every level (design 4.1 `Ledger`; S17).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    orders: Vec<Live>,
    next: u32,
    /// Blocks whose capacity a NONZERO charge lowered (the host's "every charge 0" assertion reads it).
    pub charged_blocks: u64,
    /// The largest charge read.
    pub max_charge: Units,
}

impl Ledger {
    /// An empty ledger.
    #[must_use]
    pub const fn new() -> Self {
        Self { orders: Vec::new(), next: 0, charged_blocks: 0, max_charge: 0 }
    }

    /// Registers a live order (its anchor and stop) on crate v2's 2-second lattice; returns its id.
    pub fn open(&mut self, anchor: Us, stop: Us) -> u32 {
        self.push(anchor, stop, BLOCK_US)
    }

    /// CHANGE C10: registers a live order (its anchor and stop) on the block length `blk` (microseconds: the tape's, [`crate::Tape::block_us`];
    /// [`crate::walk_ledger`] passes it); returns its id. Orders of different lengths may share one ledger: a charge on a second reads each
    /// live order's take on its OWN lattice.
    ///
    /// # Errors
    /// [`FillErr::BlockInvalid`] for a length that is not 1..=[`BLOCK_MAX_S`] whole seconds (nothing is registered).
    pub fn open_blk(&mut self, anchor: Us, stop: Us, blk: Us) -> Result<u32, FillErr> {
        if blk <= 0 || blk.rem_euclid(US_S) != 0 || blk / US_S > BLOCK_MAX_S {
            return Err(FillErr::BlockInvalid);
        }
        Ok(self.push(anchor, stop, blk))
    }

    fn push(&mut self, anchor: Us, stop: Us, blk: Us) -> u32 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        self.orders.push(Live { id, anchor, stop, blk, open: true, takes: Default::default() });
        id
    }

    /// The order is no longer live: its takes stop charging.
    pub fn retire(&mut self, id: u32) {
        if let Some(o) = self.orders.iter_mut().find(|o| o.id == id) {
            o.open = false;
        }
    }

    /// One level of the order is replaced (a host's per-level re-walk): its takes at that level stop charging.
    pub fn retire_level(&mut self, id: u32, l: usize) {
        if let Some(t) = self.orders.iter_mut().find(|o| o.id == id).and_then(|o| o.takes.get_mut(l)) {
            t.clear();
        }
    }

    /// The live orders now.
    #[must_use]
    pub fn live(&self) -> usize {
        self.orders.iter().filter(|o| o.open).count()
    }

    fn take_at(o: &Live, l: usize, s: Us) -> Units {
        if !o.open || s < o.anchor || s >= o.stop {
            return 0;
        }
        // C10: the order's own lattice (crate v2: BLOCK_US for every order).
        let Some(q) = (s - o.anchor).checked_div(o.blk) else { return 0 };
        let Ok(k) = u32::try_from(q) else { return 0 };
        let Some(v) = o.takes.get(l) else { return 0 };
        v.binary_search_by_key(&k, |x| x.0).ok().and_then(|i| v.get(i)).map_or(0, |x| x.1)
    }

    /// The LARGEST charge on block `b`'s volume seconds at level `l` from every OTHER live order (S17), counted in `reads`.
    pub(crate) fn charge(&mut self, me: u32, l: usize, b: &Block, reads: &mut u32) -> Units {
        let mut worst = 0_u64;
        let mut s = b.at;
        while s < b.to {
            let mut sum = 0_u64;
            for o in self.orders.iter().filter(|o| o.id != me) {
                *reads = reads.saturating_add(1);
                sum = sum.saturating_add(Self::take_at(o, l, s));
            }
            worst = worst.max(sum);
            s += US_S;
        }
        if worst > 0 {
            self.charged_blocks = self.charged_blocks.saturating_add(1);
            self.max_charge = self.max_charge.max(worst);
        }
        worst
    }

    /// Records the order's take at level `l` in its block `k`.
    pub(crate) fn add(&mut self, me: u32, l: usize, k: u32, units: Units) {
        if units == 0 {
            return;
        }
        if let Some(v) = self.orders.iter_mut().find(|o| o.id == me).and_then(|o| o.takes.get_mut(l)) {
            match v.last_mut() {
                Some(last) if last.0 == k => last.1 = last.1.saturating_add(units),
                _ => v.push((k, units)),
            }
        }
    }
}
