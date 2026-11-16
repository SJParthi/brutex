//! THE STANDARD's walk `req70-2s` (design 4.3): [`walk`] = sample-6's `walk_blocks` (every share level of the dial in ONE pass over an
//! order's 2-second blocks, lifted in crate v1) with `walk_group`'s level gathering replaced by a [`LevelMask`] over the GLOBAL dial;
//! step A0 adds the gap skips of 4.3, [`first_fill`], [`capacity_between`] and the two-phase [`FillPath`] (design 4.4), with no reading
//! changed. Under the `red` feature the same walk carries the OLD rule (1-second steps, the worse of the second and the next printed
//! second, one 5-paisa tick: `crate::cmp`).

use crate::gaps::{GapTables, block_cap};
use crate::ledger::Ledger;
use crate::tape::{Block, Gap, Row, Tape};
use crate::{Dial, FillErr, LevelMask, MAXL, Paisa, Side, US_S, Units, Us};

/// What an order is (design 4.1). Every kind but [`Kind::TargetLimit`] is a market order walked by blocks; the kind is a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An entry.
    Entry,
    /// A strategy's (signal) exit.
    SignalExit,
    /// A stop exit.
    Stop,
    /// A quick cut.
    QuickCut,
    /// A go-positive timer's exit.
    GoPositive,
    /// A time exit (a clock event).
    TimeExit,
    /// A trailing stop's exit.
    Trail,
    /// The sample's give-back trail's exit (a host decision, D30).
    GiveBack,
    /// A decided target exit (crate v2, change C5).
    Target,
    /// Crate v1: sample-6's RESTING target limit at `px`: it fills only in a block traded strictly BEYOND it (a sale: a HIGH above it; a
    /// buy: a LOW below it), AT `px`, the level's share of the block's volume; every block it reads is a step (sample-6's counters).
    TargetLimit {
        /// The limit price (paisa).
        px: Paisa,
    },
    /// The 15:10:00 square-off.
    SquareOff,
    /// A leg of a paired option order.
    Leg {
        /// The leg's number.
        leg: u8,
        /// Opening (true) or closing.
        open: bool,
    },
}

/// One order side (design 4.1; sample-6's `Ord70` less its cap and pay, which are [`walk`]'s arguments).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Order {
    /// Buy or sell.
    pub side: Side,
    /// T: the first block's start (a whole second; sample-6's `origin`).
    pub anchor: Us,
    /// No fill at or after it: the block crossing it is clipped.
    pub stop: Us,
    /// The unit of every fill: 1 share, or the contract's lot (0 is read as 1, as sample-6 reads `lot.max(1)`).
    pub unit: Units,
    /// Dhan's protection limit, fixed at the send (None: none, the broker's square-off). Not read for [`Kind::TargetLimit`].
    pub limit: Option<Paisa>,
    /// What the order is.
    pub kind: Kind,
}

/// Why a block row was written (design 4.1). Blocks with no valid print are jumped or read and only counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Why {
    /// The level took units here.
    #[default]
    Filled,
    /// The block's price was past the protection limit: nothing there.
    PastLimit,
    /// `req70-2s-cap` only: the block's price was above the money cap: nothing there.
    PastCap,
    /// REQ 70a: the payable exposure left no whole unit: nothing, the rest cancelled.
    Unpaid,
    /// The level's share of the block's volume floored to 0 whole units.
    ZeroTake,
    /// A paired option leg held back by the pairing rule (S18, D14: never above the other leg's cumulative units).
    Paired,
}

impl Why {
    /// The row code (design section 12, T2 `why`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Filled => "filled",
            Self::PastLimit => "past_limit",
            Self::PastCap => "past_cap",
            Self::Unpaid => "unpaid",
            Self::ZeroTake => "zero_take",
            Self::Paired => "paired",
        }
    }
}

/// One block row of one order at one level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockFill {
    /// The block's number on the order's own lattice: (block start - anchor) / 2 s.
    pub k: u32,
    /// The block.
    pub block: Block,
    /// Units taken here (0 unless `why` is [`Why::Filled`]).
    pub units: Units,
    /// The price of each unit: a buy the block's highest HIGH, a sale its lowest LOW, a target its limit; nothing added.
    pub px: Paisa,
    /// The level's units filled after this row (its accumulated summary's).
    pub cum: Units,
    /// What happened.
    pub why: Why,
}

/// Deterministic work counters per level (the O(1) evidence, design 11): blocks read while the level had units to fill (sample-6's
/// `LvFill::steps`: blocks with a valid print for a market order; every block read for a target), rows read in them, index and table
/// reads, lattice jumps over empty blocks, gap skips (design 4.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Steps {
    /// Blocks read for this level (sample-6's steps).
    pub blocks: u32,
    /// Rows read in those blocks.
    pub rows_read: u32,
    /// Second-index and gap-table reads.
    pub table_reads: u32,
    /// O(1) jumps over empty blocks.
    pub jumps: u32,
    /// Gap skips (design 4.3): a zero-capacity run or a run past the protection limit skipped in O(1) / O(log S).
    pub skips: u32,
}

/// What one order filled at one level (design 4.1; sample-6's `LvFill` plus the design's counters). ACCUMULATED: [`walk`] adds to it, as
/// `walk_blocks` added to its `LvFill`; a fresh walk passes `LevelFill::default()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LevelFill {
    /// Units filled.
    pub units: Units,
    /// Sum of units x price (paisa), checked.
    pub value: Paisa,
    /// Blocks that filled.
    pub blocks_filled: u32,
    /// Empty lattice blocks jumped while the level had units to fill.
    pub no_trade: u32,
    /// Blocks with a print past the protection limit while the level had units to fill (sample-6's `blocked`).
    pub past_limit: u32,
    /// Blocks past the money cap (`req70-2s-cap`) while the level had units to fill (sample-6's `capped`).
    pub past_cap: u32,
    /// Blocks with a print clipped by the order's stop, read while the level had units to fill.
    pub clipped: u32,
    /// Blocks where the level's share floored to 0 whole units.
    pub zero_take_blocks: u32,
    /// The first and the last filling block.
    pub first: Option<BlockFill>,
    /// See `first`.
    pub last: Option<BlockFill>,
    /// E0: the first filling block's start (0 = none).
    pub e0: Us,
    /// E1: the first filling block's known end (0 = none).
    pub e1: Us,
    /// R22: the known end of the last filling block (0 = none).
    pub known_at: Us,
    /// The rest when the walk ended (at the stop, or out of rows).
    pub rest_at_stop: Units,
    /// REQ 70a: units the payable exposure cancelled.
    pub unpaid: Units,
    /// Traded seconds of the filling blocks (S1's count).
    pub filled_secs: u32,
    /// Of them, the seconds with one recorded price (S1's count).
    pub one_price_secs: u32,
    /// S3: the order's walked span [anchor, the end of its last block read) overlaps a suspected feed hole (change C6).
    pub hole_span: bool,
    /// Work counters.
    pub steps: Steps,
    /// CHANGE S1 (crate v2.6, strict views only; 0 otherwise): blocks whose re-send NSE's limit price protection rejected (0 capacity), blocks
    /// whose protection reference could not be bounded (the check skipped), seconds the band lock left out, blocks whose limit was clipped to
    /// NSE's exact band.
    pub lpp_rejected: u32,
    /// CHANGE S1: blocks whose protection reference could not be bounded (the check skipped, counted).
    pub lpp_unknown: u32,
    /// CHANGE S1: traded seconds the band lock left out (0 usable volume).
    pub locked_secs: u32,
    /// CHANGE S1: blocks whose limit was clipped to NSE's exact band (no print had passed it).
    pub band_clipped: u32,
}

/// The block-row recorder a host passes to [`walk`]: called with the GLOBAL level and each row, in time order.
pub type Recorder<'r> = &'r mut dyn FnMut(usize, &BlockFill);

/// The red build's (the old rule's) tick against the trader, paisa: REQ 58b's one tick at the fixed 5 paise (design 3, `crate::cmp`).
pub(crate) const RED_TICK: Paisa = 5;
/// The gap skip is tried after this many unproductive blocks in a row (design 4.3: "the first gap longer than 16 blocks").
pub(crate) const SKIP_AFTER: u32 = 16;

/// One level's take in one priced block (sample-6's lines, exactly): the share, the payable cut, the fill, the rest. Returns the units
/// taken. `used` (mutant m6 only) is the volume the lower levels took in this block.
#[allow(clippy::too_many_arguments, reason = "one level's take")]
fn take_level(f: &mut LevelFill, w: &mut Units, b: &Block, k: u32, px: Paisa, bp: u16, unit: Units, pay: Option<Paisa>, rec: &mut Option<Recorder<'_>>, l: usize, used: Units, charge: Units, carried: Option<Units>) -> Result<Units, FillErr> {
    // C8 (S17): the level's share of the block's volume, less the largest charge of our other live orders on its volume seconds.
    // CHANGE FA1: the block capacity under the tape's reading (crate v2.1's cap_units of the recorded volume without one).
    // CHANGE K1: on a carrying view, the level's carried entitlement (every block's share in whole units since the order instant, this
    // block included) in whole lots, less the units it already filled.
    let mut capu = match carried {
        Some(cum) => (cum / unit * unit).saturating_sub(f.units),
        None => block_cap(b, bp, unit)?.saturating_sub(charge) / unit * unit,
    };
    if cfg!(feature = "m6") {
        // MUTANT m6 (design 10.4): levels walked sequentially with a shared rest (the lower levels' take leaves less for this one).
        capu = capu.saturating_sub(used) / unit * unit;
    }
    let mut take = capu.min(*w);
    if capu == 0 {
        f.zero_take_blocks = f.zero_take_blocks.saturating_add(1);
    }
    let mut cut = false;
    if let Some(p) = pay
        && take > 0
        && px > 0
    {
        let room = p.checked_sub(f.value).ok_or(FillErr::Overflow)?.max(0);
        let fit = Units::try_from(room / px).map_err(|_| FillErr::Overflow)? / unit * unit;
        if fit < take {
            take = fit;
            cut = true;
        }
    }
    let known = if cfg!(feature = "m5") { b.at } else { b.known };
    let why = if take > 0 {
        let units = f.units.checked_add(take).ok_or(FillErr::Overflow)?;
        let add = i64::try_from(take).map_err(|_| FillErr::Overflow)?.checked_mul(px).ok_or(FillErr::Overflow)?;
        let value = f.value.checked_add(add).ok_or(FillErr::Overflow)?;
        let bf = BlockFill { k, block: *b, units: take, px, cum: units, why: Why::Filled };
        if f.blocks_filled == 0 {
            f.first = Some(bf);
            f.e0 = b.at;
            f.e1 = known;
        }
        f.blocks_filled = f.blocks_filled.checked_add(1).ok_or(FillErr::Overflow)?;
        f.last = Some(bf);
        f.known_at = f.known_at.max(known);
        f.filled_secs = f.filled_secs.saturating_add(u32::from(b.n_secs));
        f.one_price_secs = f.one_price_secs.saturating_add(u32::from(b.one_price));
        f.units = units;
        f.value = value;
        *w -= take;
        Why::Filled
    } else if cut {
        Why::Unpaid
    } else {
        Why::ZeroTake
    };
    if let Some(r) = rec.as_mut() {
        let units = if why == Why::Filled { take } else { 0 };
        r(l, &BlockFill { k, block: *b, units, px, cum: f.units, why });
    }
    if cut {
        f.unpaid = f.unpaid.checked_add(*w).ok_or(FillErr::Overflow)?;
        *w = 0;
    }
    Ok(if why == Why::Filled { take } else { 0 })
}

/// REQ 70 + 70a + 70b: ONE order's blocks for every level in `mask` of the GLOBAL `dial` at once, each level with its own rest `want[l]`
/// (in, out) and its accumulated summary `out[l]`. From `o.anchor` the 2-second lattice runs to `o.stop`: a block with no row is jumped
/// in O(1) to the block holding the next row (counted `no_trade`); a block with no valid print fills nothing; a block priced past the
/// protection limit (or above the money `cap` of `req70-2s-cap`, both sides) fills nothing there and the rest waits; otherwise each level
/// takes min(rest, its share of the block's volume in whole units) at the block's price (a buy its highest HIGH, a sale its lowest LOW,
/// nothing added). `money[l]` (REQ 70a): the level's payable exposure: the order stops at the units whose cumulative value fits it, the
/// rest is cancelled (`unpaid`). A block crossing the stop is clipped (its reading: [`Tape::block_from`]). `rec` receives every block row
/// of every level, in time order. With the tape's gap tables (and no `rec`, no `cap`, a market order), a run of 16 unproductive blocks
/// is followed by a gap skip (design 4.3): the same fills and counters, fewer steps. O(1) per block and level.
///
/// # Errors
/// [`FillErr::NotWholeSecond`] for an anchor not on a whole second; [`FillErr::DialInvalid`] for a mask naming a level the dial does
/// not have; [`FillErr::LotMismatch`] for a want that is not a whole number of units; [`FillErr::Overflow`] when a checked sum leaves its
/// type (the walk stops; the host refuses the run).
#[allow(clippy::too_many_arguments, reason = "one order at every level in one pass (design 4.2's signature)")]
pub fn walk<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], money: Option<&[Option<Paisa>; MAXL]>, cap: Option<Paisa>, out: &mut [LevelFill; MAXL], rec: Option<Recorder<'_>>) -> Result<(), FillErr> {
    walk_core(tape, o, dial, mask, want, money, cap, None, out, rec, None, Opts::default())
}

/// [`walk`] with the shared-volume ledger of our live orders on this instrument (CHANGE C8, S17, D13: design 4.2's `led`): the order is
/// registered in `led` (its id returned), each block's capacity is lowered by the largest charge of our other live orders on its volume
/// seconds, and its own takes are recorded there. Never refused. CHANGE C10 (crate v2.1): the order is registered on the TAPE's block
/// length (1 s for REQ 73b), so its takes are keyed on its own lattice and EVERY-mode re-entries run at any length (C9 refused them).
///
/// # Errors
/// As [`walk`]; [`FillErr::BlockInvalid`] never (the tape's length is valid by construction).
#[allow(clippy::too_many_arguments, reason = "walk's arguments and the ledger")]
pub fn walk_ledger<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], money: Option<&[Option<Paisa>; MAXL]>, cap: Option<Paisa>, led: &mut Ledger, out: &mut [LevelFill; MAXL], rec: Option<Recorder<'_>>) -> Result<u32, FillErr> {
    // CHANGE FA1: the shared-volume ledger reads recorded volumes: refused under a capacity reading.
    if tape.cap_mode() != crate::tape::CapMode::Row {
        return Err(FillErr::CapReadNotServed);
    }
    // CHANGE K1: the ledger charges per-block capacities: refused on a carrying view.
    if tape.carry() {
        return Err(FillErr::CarryNotServed);
    }
    let id = led.open_blk(o.anchor, o.stop, tape.block_us())?;
    walk_core(tape, o, dial, mask, want, money, cap, Some((led, id)), out, rec, None, Opts::default())?;
    Ok(id)
}

/// The walk variants: `first_only` stops each level at its first filling block ([`first_fill`]); `filled_only` says the recorder keeps
/// only the filling rows ([`FillPath::record`]), so the gap skips stay allowed with it.
#[derive(Clone, Copy, Debug, Default)]
struct Opts {
    first_only: bool,
    filled_only: bool,
}

/// The price of block `b` for order `o` and whether it may fill: a market order at its own extreme within the protection limit; sample-6's
/// target leg only where the block traded strictly beyond the limit, at the limit. Under `red`: the old rule's price (the worse of the
/// second and the next printed second, one tick against the trader); mutant m3: the tick re-added.
fn price_of<R: Row>(tape: &Tape<'_, R>, o: &Order, b: &Block, next_row: usize) -> (Paisa, bool, bool, bool) {
    if let Kind::TargetLimit { px: lim } = o.kind {
        // A RESTING limit fills only on trades before its cancel: its "traded beyond" test reads the block's seconds before the stop
        // (`high_pre` / `low_pre`; C1's two-second price is a market order's worst, never a resting limit's fill condition).
        let beyond = b.n > 0
            && match o.side {
                Side::Sell => b.high_pre > lim,
                Side::Buy => b.low_pre < lim,
            };
        return (lim, true, !beyond, false);
    }
    let mut px = b.px(o.side);
    if cfg!(feature = "red") && b.n > 0 {
        // THE OLD RULE (the red reading, crate::cmp): the worse of this second and the next printed second, one tick against the trader.
        if let Some(nx) = tape.rows().get(next_row..).and_then(|w| w.iter().find(|s| crate::tape::valid(*s))) {
            px = match o.side {
                Side::Buy => px.max(nx.high()),
                Side::Sell => px.min(nx.low()),
            };
        }
        px = match o.side {
            Side::Buy => px.saturating_add(RED_TICK),
            Side::Sell => px.saturating_sub(RED_TICK).max(1),
        };
    } else if cfg!(feature = "m3") && b.n > 0 {
        // MUTANT m3 (design 10.4): the tick re-added.
        px = match o.side {
            Side::Buy => px.saturating_add(RED_TICK),
            Side::Sell => px.saturating_sub(RED_TICK).max(1),
        };
    }
    let past = o.limit.is_some_and(|lim| match o.side {
        Side::Buy => px > lim,
        Side::Sell => px < lim,
    });
    (px, b.n > 0, past, true)
}

/// CHANGE K2: the order as re-sent for the block starting at `at` (an exit order with a limit on a re-sending view: the protection of the
/// last print strictly before `at`; every other order unchanged). O(1).
fn resend_order<R: Row>(tape: &Tape<'_, R>, o: &Order, at: Us) -> Order {
    match (tape.resend(), o.kind, o.limit) {
        (Some((bp, tick)), Kind::SignalExit, Some(_)) => match crate::limits::size_ref(tape, at, o.side) {
            Some(r) => Order { limit: Some(band_clip(tape.band(), o.side, crate::limits::protection(r, o.side, bp, tick))), ..*o },
            None => *o,
        },
        _ => *o,
    }
}

/// CHANGE S1 (crate v2.6, data:F7 5.2 / 5.4): the order as (re-)sent for the block starting at `at` on a STRICT view: an exit or square-off
/// order with a limit takes the reading's limit from the last print strictly before `at` (Dhan's protection at bp on the tick, or NSE's
/// market-order cap LTP + max(20%, Rs 10), clipped to the LPP band's proven bound when the law says so), then is clipped to NSE's EXACT band
/// while no print has passed it (counted: never a fill changed, no print can pass an unflexed band); every other order unchanged. Returns
/// the order and whether the band clipped its limit. O(1).
fn resend_strict<R: Row>(tape: &Tape<'_, R>, s: &crate::tape::Strict, o: &Order, at: Us) -> (Order, bool) {
    if !matches!(o.kind, Kind::SignalExit | Kind::SquareOff) || o.limit.is_none() {
        return (*o, false);
    }
    let Some(r) = crate::limits::size_ref(tape, at, o.side) else { return (*o, false) };
    let lim = match s.law {
        crate::tape::ResendLaw::Protection { bp, tick } => crate::limits::protection(r, o.side, bp, tick),
        crate::tape::ResendLaw::Uncapped => match o.side {
            Side::Buy => Paisa::MAX / 4,
            Side::Sell => 5,
        },
        crate::tape::ResendLaw::NseMarket { lpp_clip } => {
            // NSE/FAOP/51600: LTP +/- max(20% of LTP, Rs 10) (MUTANT m_v34i_mpp_no_min_abs: the Rs 10 minimum left out)
            let m = (r.saturating_mul(20).saturating_add(99) / 100).max(if cfg!(feature = "m_v34i_mpp_no_min_abs") { 0 } else { 1_000 });
            let cap = match o.side {
                Side::Buy => r.saturating_add(m),
                Side::Sell => r.saturating_sub(m).max(5),
            };
            if lpp_clip { lpp_clip_cap(tape, at, o.side, cap) } else { cap }
        }
    };
    let (known, unflexed) = tape.known_band(s, at, o.side);
    // MUTANT m_v34i_clip_to_bound (data:F7 m_f7_clip_to_bound): a limit clipped to the no-file BOUND too (it would refuse feasible fills)
    let unflexed = unflexed || (cfg!(feature = "m_v34i_clip_to_bound") && known.is_some());
    let mut clipped = false;
    let lim = match (o.side, unflexed, known) {
        (Side::Buy, true, Some(u)) if lim > u => {
            clipped = true;
            u
        }
        (Side::Sell, true, Some(d)) if lim < d => {
            clipped = true;
            d
        }
        _ => lim,
    };
    (Order { limit: Some(lim), ..*o }, clipped)
}

/// CHANGE S1 (data:F7 2.4, NSE/FAOP/59475 from 2023-12-04): a market-order cap kept inside the LPP band's proven bound at `at` (a buy at most
/// the band of the lowest possible reference; a sale at least that of the highest); the cap unchanged when the reference is unknown.
fn lpp_clip_cap<R: Row>(tape: &Tape<'_, R>, at: Us, side: Side, cap: Paisa) -> Paisa {
    let Some(p) = tape.last_traded_before(at.saturating_sub(29 * US_S)) else { return cap };
    if p.ts < at.saturating_sub((15 * 60 + 30) * US_S) {
        return cap;
    }
    let from = p.ts.saturating_sub(30 * US_S);
    match side {
        Side::Buy => tape.extreme(from, at, false).map_or(cap, |(lo, _)| cap.min(if lo <= 5_000 { lo.saturating_add(2_000) } else { lo.saturating_mul(14) / 10 })),
        Side::Sell => tape.extreme(from, at, true).map_or(cap, |(hi, _)| cap.max(if hi <= 5_000 { hi.saturating_sub(2_000) } else { (hi.saturating_mul(6) + 9) / 10 })),
    }
}

/// CHANGE B1 (crate v2.5, spikefade v3.4 BANDS): a limit clipped one tick inside the contract-day's price band (a buy below the upper band,
/// a sale above the lower band): a block whose worst print sits AT the band is band-locked (past the limit: no fill). None = unchanged.
#[must_use]
pub fn band_clip(band: Option<(Paisa, Paisa, Paisa)>, side: Side, lim: Paisa) -> Paisa {
    match band {
        Some((lo, hi, tick)) => match side {
            Side::Buy => lim.min(hi.saturating_sub(tick)),
            Side::Sell => lim.max(lo.saturating_add(tick)),
        },
        None => lim,
    }
}

/// Whether `px` is past the money cap `c` for `side` (`req70-2s-cap`): ABOVE it on both sides (critic A B2); mutant m10 flips a sale's.
fn is_capped(side: Side, px: Paisa, c: Paisa) -> bool {
    if cfg!(feature = "m10") && side == Side::Sell { px < c } else { px > c }
}

/// The walk, with its variants ([`Opts`]).
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one order at every level in one pass")]
fn walk_core<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], money: Option<&[Option<Paisa>; MAXL]>, cap: Option<Paisa>, mut led: Option<(&mut Ledger, u32)>, out: &mut [LevelFill; MAXL], mut rec: Option<Recorder<'_>>, mut rec_cum: Option<&mut dyn FnMut(usize, Us, Units)>, opts: Opts) -> Result<(), FillErr> {
    if o.anchor.rem_euclid(US_S) != 0 {
        return Err(FillErr::NotWholeSecond);
    }
    // CHANGE FA1: every block of this order reads its capacity for THIS order's anchor (no effect without a capacity reading).
    let tape = &tape.for_order(o.anchor);
    // CHANGE C9 (REQ 73b): the block length of this tape (2 s unless the host set the dial; [`Tape::with_block`]).
    let blk = tape.block_us();
    // CHANGE S1 (crate v2.6): the view's strict order reading (None = crate v2.5)
    let strict = tape.strict();
    let nl = dial.len();
    if (nl..MAXL).any(|l| mask.has(l)) {
        return Err(FillErr::DialInvalid);
    }
    let unit = o.unit.max(1);
    if (0..nl).any(|l| mask.has(l) && want.get(l).is_some_and(|&w| w % unit != 0)) {
        return Err(FillErr::LotMismatch);
    }
    let target = matches!(o.kind, Kind::TargetLimit { .. });
    let live = |want: &[Units; MAXL], l: usize| mask.has(l) && want.get(l).is_some_and(|&w| w > 0);
    let gaps: Option<&GapTables> = if (rec.is_none() || opts.filled_only) && cap.is_none() && !target && strict.is_none() { tape.gaps().filter(|g| g.serves(dial, unit)) } else { None };
    let mut at = o.anchor;
    let mut i = tape.first_at(at);
    for (l, f) in out.iter_mut().enumerate().take(nl) {
        if live(want, l) {
            f.steps.table_reads = f.steps.table_reads.saturating_add(1);
        }
    }
    let mut run = 0_u32;
    // CHANGE K1: per level, the carried entitlement in whole units (each priced block within the limit and the cap adds the level's share
    // of its usable volume, floored ONCE to whole units: block_cap at unit 1), when the view carries.
    // MUTANT m_v31_carry_per_block: the carried entitlement dropped (each block's own lot floor, the v2.1 reading the ruling replaced)
    let carry = tape.carry() && !cfg!(feature = "m_v31_carry_per_block");
    let mut cumu = [0 as Units; MAXL];
    let mut cur_seg: Option<Us> = None;
    // CHANGE C6 (S3, S9): no block after the day's last segment ends (the square-off's stop is the segment end).
    let stop = o.stop.min(tape.session_end());
    while at < stop {
        if !(0..nl).any(|l| live(want, l)) {
            break;
        }
        // C6: a lattice block wholly in a halt gap holds nothing: jump (O(1)) to the lattice block holding the next segment's open.
        match tape.gap_jump(at) {
            Gap::Jump(next) => {
                at = next;
                i = tape.first_at(at);
                for (l, f) in out.iter_mut().enumerate().take(nl) {
                    if live(want, l) {
                        f.steps.jumps = f.steps.jumps.saturating_add(1);
                    }
                }
                continue;
            }
            Gap::After => break,
            Gap::Inside => {}
        }
        let Some(s) = tape.rows().get(i) else { break };
        let sts = s.ts();
        if sts >= stop {
            break;
        }
        let end = at.checked_add(blk).ok_or(FillErr::Overflow)?;
        if sts >= end {
            // Empty blocks fill nothing: jump the lattice to the block holding the next row.
            let k = (sts - at) / blk;
            at = if cfg!(feature = "m2") {
                // MUTANT m2 (design 10.4): the lattice re-anchored at a print.
                sts
            } else {
                at.checked_add(k.checked_mul(blk).ok_or(FillErr::Overflow)?).ok_or(FillErr::Overflow)?
            };
            let kk = u32::try_from(k).unwrap_or(u32::MAX);
            for (l, f) in out.iter_mut().enumerate().take(nl) {
                if live(want, l) {
                    f.no_trade = f.no_trade.saturating_add(kk);
                    f.steps.jumps = f.steps.jumps.saturating_add(1);
                }
            }
            continue;
        }
        // CHANGE S1 (crate v2.6): on a strict view the block's band-locked seconds give no volume (data:F7 5.4); crate v2.5 otherwise
        let (b, ni, locked) = match strict.as_ref() {
            Some(s) => {
                let (lock, _) = tape.known_band(s, at, o.side);
                tape.block_locked(i, at, stop, o.side, lock)?
            }
            None => {
                let (b, ni) = tape.block_from(i, at, stop)?;
                (b, ni, 0)
            }
        };
        let rows_read = u32::try_from(ni.saturating_sub(i)).unwrap_or(u32::MAX);
        i = ni;
        let k = u32::try_from((b.at - o.anchor) / blk).map_err(|_| FillErr::Overflow)?;
        // CHANGE K1 (matrix X24): crossing a halt or a segment end drops the carried, unused entitlement (each level keeps what it filled)
        if carry {
            let so = tape.segment_open_at(b.at);
            if cur_seg.is_some() && so != cur_seg {
                for l in 0..nl {
                    let u = out.get(l).map_or(0, |f| f.units);
                    if let Some(x) = cumu.get_mut(l) {
                        *x = u;
                    }
                    if let Some(r) = rec_cum.as_mut() {
                        r(l, b.at - 1, u);
                    }
                }
            }
            cur_seg = so;
        }
        // CHANGE K2: an exit order is re-sent every block with its limit re-priced from the last print before the block
        // CHANGE S1: on a strict view the reading's law (and the exact band's clip), and NSE's limit price protection at the send
        let (ob, clip, lpp) = match strict.as_ref() {
            Some(s) => {
                let (ob, clip) = resend_strict(tape, s, o, b.at);
                // MUTANT m_v34i_lpp_ignored (data:F7 m_x05_lpp_ignored): the protection never checked
                let lpp = match ob.limit {
                    Some(lim) if s.lpp && matches!(o.kind, Kind::SignalExit | Kind::SquareOff) && !cfg!(feature = "m_v34i_lpp_ignored") => tape.lpp_check(b.at, o.side, lim),
                    _ => crate::tape::LppCheck::Admit,
                };
                (ob, clip, lpp)
            }
            None => (resend_order(tape, o, b.at), false, crate::tape::LppCheck::Admit),
        };
        let (px, priced, past, counts_past) = price_of(tape, &ob, &b, ni);
        let capped = !past && cap.is_some_and(|c| is_capped(o.side, px, c));
        // CHANGE S1: the counters of a strict view (per live level, as the walk counts its blocks)
        if strict.is_some() {
            for (l, f) in out.iter_mut().enumerate().take(nl) {
                if live(want, l) {
                    f.locked_secs = f.locked_secs.saturating_add(locked);
                    f.band_clipped = f.band_clipped.saturating_add(u32::from(clip && priced));
                    match lpp {
                        crate::tape::LppCheck::Reject if priced => f.lpp_rejected = f.lpp_rejected.saturating_add(1),
                        crate::tape::LppCheck::Unknown if priced => f.lpp_unknown = f.lpp_unknown.saturating_add(1),
                        _ => {}
                    }
                }
            }
        }
        // CHANGE S1: a re-send NSE rejected holds nothing in this block (no fill, no carried entitlement: the order was not in the book)
        let rejected = matches!(lpp, crate::tape::LppCheck::Reject);
        let mut took_any = false;
        let mut used = 0_u64;
        for l in 0..nl {
            if !live(want, l) {
                continue;
            }
            let (Some(f), Some(w), Some(bp)) = (out.get_mut(l), want.get_mut(l), dial.bp(l)) else { continue };
            f.steps.rows_read = f.steps.rows_read.saturating_add(rows_read);
            if !priced {
                continue;
            }
            f.steps.blocks = f.steps.blocks.saturating_add(1);
            if b.clipped && b.n > 0 {
                f.clipped = f.clipped.saturating_add(1);
            }
            if rejected {
                // CHANGE S1: rejected at the send by NSE's limit price protection (counted above): nothing here, the next block re-sends
                continue;
            }
            if past {
                if counts_past {
                    f.past_limit = f.past_limit.saturating_add(1);
                    if let Some(r) = rec.as_mut() {
                        r(l, &BlockFill { k, block: b, units: 0, px, cum: f.units, why: Why::PastLimit });
                    }
                }
                continue;
            }
            if capped {
                f.past_cap = f.past_cap.saturating_add(1);
                if let Some(r) = rec.as_mut() {
                    r(l, &BlockFill { k, block: b, units: 0, px, cum: f.units, why: Why::PastCap });
                }
                continue;
            }
            let pay = money.and_then(|m| m.get(l).copied().flatten());
            let charge = match led.as_mut() {
                Some((g, id)) => g.charge(*id, l, &b, &mut f.steps.table_reads),
                None => 0,
            };
            let carried = if carry {
                let c = cumu.get(l).copied().unwrap_or(0).checked_add(block_cap(&b, bp, 1)?).ok_or(FillErr::Overflow)?;
                if let Some(x) = cumu.get_mut(l) {
                    *x = c;
                }
                if let Some(r) = rec_cum.as_mut() {
                    r(l, b.at, c);
                }
                Some(c)
            } else {
                None
            };
            let took = take_level(f, w, &b, k, px, bp, unit, pay, &mut rec, l, used, charge, carried)?;
            if let Some((g, id)) = led.as_mut() {
                g.add(*id, l, k, took);
            }
            used = used.saturating_add(took);
            if took > 0 {
                took_any = true;
                if opts.first_only {
                    *w = 0;
                }
            }
        }
        // CHANGE C6 (S3, critic A F4): after ANY clipped block the walk continues on its own lattice, at + 2 s (crate v1: at = b.to, which a
        // segment-end clip would shift). MUTANT m11 (design 10.4) keeps at = b.to.
        let next = b.at.checked_add(blk).ok_or(FillErr::Overflow)?;
        if b.to < next && !cfg!(feature = "m11") {
            at = next;
            i = tape.first_at(at);
        } else {
            at = b.to;
        }
        // THE GAP SKIP (design 4.3): after 16 unproductive blocks in a row, the tables give the next block worth reading (never across a
        // segment end, C6).
        if let Some(g) = gaps {
            run = if took_any { 0 } else { run.saturating_add(1) };
            if run >= SKIP_AFTER && !b.clipped {
                run = 0;
                let lim = stop.min(tape.segment_end_at(at));
                skip(g, tape, o, lim, nl, &live, want, out, past && priced, &mut at, &mut i)?;
            }
        }
    }
    // C6 (S3, OB40): an order whose walked span [anchor, its reach) overlaps a suspected feed hole is flagged.
    let span = tape.spans_hole(o.anchor, at.min(stop));
    for (l, f) in out.iter_mut().enumerate().take(nl) {
        if mask.has(l) {
            f.rest_at_stop = want.get(l).copied().unwrap_or(0);
            f.hole_span = f.hole_span || span;
        }
    }
    Ok(())
}

/// One gap skip from the block starting at `at` (design 4.3): (a) after a block within the limit, to the first block >= it that is either
/// capable (a whole unit at some live level) or past the limit; (b) after a block past the limit, to the first traded block within it.
/// Every block skipped is counted as the block-by-block walk counts it: an empty block `no_trade`, a traded block `zero_take_blocks` (a)
/// or `past_limit` (b), a block with rows but no print nothing. The skip ends before the stop's block and, when it would reach it, right
/// after the last block holding a row (the walk then handles the tail block by block, as before).
#[allow(clippy::too_many_arguments, reason = "the walk's state")]
fn skip<R: Row>(g: &GapTables, tape: &Tape<'_, R>, o: &Order, stop: Us, nl: usize, live: &dyn Fn(&[Units; MAXL], usize) -> bool, want: &[Units; MAXL], out: &mut [LevelFill; MAXL], past: bool, at: &mut Us, i: &mut usize) -> Result<(), FillErr> {
    let Some((p, j)) = g.locate(*at) else { return Ok(()) };
    let nb = g.blocks(p);
    if j >= nb {
        return Ok(());
    }
    // Full blocks wholly before the stop (and the segment end, C6): [j, j_stop). CHANGE C10: blocks of the tables' (= the tape's) length.
    let blk = g.block_us();
    if blk <= 0 {
        return Ok(());
    }
    let room = stop.checked_sub(*at).ok_or(FillErr::Overflow)?;
    if room < blk {
        return Ok(());
    }
    let full = usize::try_from(room / blk).map_err(|_| FillErr::Overflow)?;
    let j_stop = j.saturating_add(full).min(nb);
    let mut reads = 0_u32;
    let target = if past && let Some(lim) = o.limit {
        g.first_within(p, j, o.side, lim, &mut reads).unwrap_or(nb)
    } else {
        let mut jc = nb;
        for l in 0..nl {
            if live(want, l) {
                reads = reads.saturating_add(1);
                jc = jc.min(g.next_capable(p, l, j).unwrap_or(nb));
            }
        }
        let jp = o.limit.and_then(|lim| g.first_past(p, j, o.side, lim, &mut reads)).unwrap_or(nb);
        jc.min(jp)
    };
    let mut jj = target.min(j_stop);
    if jj == j_stop {
        jj = g.prev_any_end(p, j_stop).max(j).min(j_stop);
    }
    if jj <= j {
        return Ok(());
    }
    let span = u32::try_from(jj - j).map_err(|_| FillErr::Overflow)?;
    let any = g.any_between(p, j, jj);
    let traded = g.traded_between(p, j, jj);
    let empty = span.saturating_sub(any);
    for l in 0..nl {
        if !live(want, l) {
            continue;
        }
        let Some(f) = out.get_mut(l) else { continue };
        f.no_trade = f.no_trade.saturating_add(empty);
        if past {
            f.past_limit = f.past_limit.saturating_add(traded);
        } else {
            f.zero_take_blocks = f.zero_take_blocks.saturating_add(traded);
        }
        f.steps.skips = f.steps.skips.saturating_add(1);
        f.steps.table_reads = f.steps.table_reads.saturating_add(reads.saturating_add(1));
    }
    *at = g.start_of(p, jj).ok_or(FillErr::Overflow)?;
    *i = tape.first_at(*at);
    Ok(())
}

/// Each masked level's FIRST filling block: the SAME walk, stopped per level at its first fill (index options' cancel probe; no lane keeps
/// its own block predicate, critic B F9). O(1) per block read.
///
/// # Errors
/// As [`walk`].
pub fn first_fill<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &[Units; MAXL]) -> Result<[Option<BlockFill>; MAXL], FillErr> {
    let mut w = *want;
    let mut out = [LevelFill::default(); MAXL];
    walk_core(tape, o, dial, mask, &mut w, None, None, None, &mut out, None, None, Opts { first_only: true, filled_only: false })?;
    let mut got = [None; MAXL];
    for (g, f) in got.iter_mut().zip(out.iter()) {
        *g = f.first;
    }
    Ok(got)
}

/// The capacity of the order's lattice blocks (anchor `anchor`) lying wholly inside [from, to) at one level (`level_bp`, in whole `unit`s),
/// with no ledger and no limit (REQ 47d (1)'s absorb / exit-capacity tests, D23): O(1) through the gap tables' capacity prefix sums when
/// the tape has tables for that level and unit, else block by block (the same answer). Returns the capacity and the steps.
///
/// # Errors
/// [`FillErr::NotWholeSecond`] for an anchor off whole seconds; [`FillErr::Overflow`] when a sum leaves its type.
pub fn capacity_between<R: Row>(tape: &Tape<'_, R>, anchor: Us, from: Us, to: Us, level_bp: u16, unit: Units) -> Result<(Units, u32), FillErr> {
    if anchor.rem_euclid(US_S) != 0 {
        return Err(FillErr::NotWholeSecond);
    }
    // CHANGE FA1: the blocks read their capacity for this anchor.
    let tape = &tape.for_order(anchor);
    // CHANGE C9: the lattice of this tape's block length.
    let blk = tape.block_us();
    let from = from.max(anchor);
    if to <= from {
        return Ok((0, 0));
    }
    // The first lattice block starting at or after `from`, and the blocks ending at or before `to`.
    let k0 = (from - anchor + blk - 1) / blk;
    let a0 = anchor.checked_add(k0.checked_mul(blk).ok_or(FillErr::Overflow)?).ok_or(FillErr::Overflow)?;
    if a0.checked_add(blk).ok_or(FillErr::Overflow)? > to {
        return Ok((0, 0));
    }
    let mut nblk = (to - a0) / blk;
    let mut c = 0_u64;
    let mut steps = 0_u32;
    let mut at = a0;
    if let Some(g) = tape.gaps()
        && let Some(l) = g.level_of(level_bp, unit)
        && let Some((p, j0)) = g.locate(a0)
        && j0 < g.blocks(p)
    {
        // The blocks inside the tables in O(1); a tail beyond the index span (no rows there on a day's index) block by block.
        let j1 = j0.saturating_add(usize::try_from(nblk).map_err(|_| FillErr::Overflow)?).min(g.blocks(p));
        let inside = g.capacity(p, l, j0, j1).ok_or(FillErr::Overflow)?;
        c = inside;
        steps = 2;
        let done = i64::try_from(j1 - j0).map_err(|_| FillErr::Overflow)?;
        nblk -= done;
        at = at.checked_add(done.checked_mul(blk).ok_or(FillErr::Overflow)?).ok_or(FillErr::Overflow)?;
    }
    // CHANGE K1: on a carrying view the capacity is the carried entitlement of the blocks (each block's share in whole units, summed, then
    // whole `unit`s once; no gap tables there).
    let mut usable: Units = 0;
    for _ in 0..nblk {
        let b = tape.block(at, Us::MAX)?;
        steps = steps.saturating_add(1);
        if tape.carry() {
            if b.n > 0 {
                usable = usable.checked_add(block_cap(&b, level_bp, 1)?).ok_or(FillErr::Overflow)?;
            }
        } else {
            c = c.checked_add(block_cap(&b, level_bp, unit)?).ok_or(FillErr::Overflow)?;
        }
        at = at.checked_add(blk).ok_or(FillErr::Overflow)?;
    }
    if tape.carry() {
        let u = unit.max(1);
        c = usable / u * u;
    }
    Ok((c, steps))
}

/// One recorded filling block of a [`FillPath`] level, with the level's cumulative capacity and value through it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PathBlk {
    bf: BlockFill,
    /// Cumulative units through this block (C_k).
    c: Units,
    /// Cumulative value through this block (V_k), paisa.
    v: Paisa,
    filled_secs: u32,
    one_price_secs: u32,
}

/// Two-phase hosts (design 4.4): ONE recorded walk per order, then every quantity as a VIEW, every REQ 70a payability cut and every earlier
/// stop by binary search over the recorded blocks: O(log blocks), at most 14 probes a call at 11,250 blocks (a NAMED bounded exception,
/// D27), counted in `steps` and asserted by the benches. Ledger-free (S17): an overlapping admission is re-priced by [`walk`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FillPath {
    o: Option<Order>,
    bp: [u16; MAXL],
    lv: [Vec<PathBlk>; MAXL],
    upto: [Units; MAXL],
    /// Whether the record stopped because the level reached `upto` (a view beyond it is refused).
    full: [bool; MAXL],
    end: [LevelFill; MAXL],
    mask: LevelMask,
    /// CHANGE K1: on a carrying view, per level (block start, the level's carried entitlement through that block) of every priced block
    /// within the limit the record read while the level was live (empty otherwise): an earlier stop re-reads its straddling block against the
    /// entitlement carried to it (O(log blocks)).
    cum: [Vec<(Us, Units)>; MAXL],
}

/// The probes of one binary search (partition point) over a sorted predicate, counted.
fn search<T>(v: &[T], pred: impl Fn(&T) -> bool, steps: &mut u32) -> usize {
    let (mut lo, mut hi) = (0_usize, v.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        *steps = steps.saturating_add(1);
        if v.get(mid).is_some_and(&pred) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

impl FillPath {
    /// [`walk`] plus a recorder (never a second loop): every masked level recorded up to `upto[l]`, the largest quantity any view will ask.
    ///
    /// # Errors
    /// As [`walk`].
    pub fn record<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, upto: &[Units; MAXL], cap: Option<Paisa>) -> Result<Self, FillErr> {
        let mut w = *upto;
        let mut out = [LevelFill::default(); MAXL];
        let mut lv: [Vec<PathBlk>; MAXL] = Default::default();
        let mut bad = false;
        let mut cumv: [Vec<(Us, Units)>; MAXL] = Default::default();
        {
            let mut rc = |l: usize, at: Us, c: Units| {
                if let Some(v) = cumv.get_mut(l) {
                    v.push((at, c));
                }
            };
            let mut rec = |l: usize, b: &BlockFill| {
                if b.why != Why::Filled {
                    return;
                }
                let Some(v) = lv.get_mut(l) else { return };
                let (pc, pv, pf, po) = v.last().map_or((0, 0, 0, 0), |x| (x.c, x.v, x.filled_secs, x.one_price_secs));
                let add = i64::try_from(b.units).ok().and_then(|u| u.checked_mul(b.px));
                match (pc.checked_add(b.units), add.and_then(|a| pv.checked_add(a))) {
                    (Some(c), Some(val)) => v.push(PathBlk { bf: *b, c, v: val, filled_secs: pf.saturating_add(u32::from(b.block.n_secs)), one_price_secs: po.saturating_add(u32::from(b.block.one_price)) }),
                    _ => bad = true,
                }
            };
            walk_core(tape, o, dial, mask, &mut w, None, cap, None, &mut out, Some(&mut rec), Some(&mut rc), Opts { first_only: false, filled_only: true })?;
        }
        if bad {
            return Err(FillErr::Overflow);
        }
        let mut bp = [0_u16; MAXL];
        let mut full = [false; MAXL];
        for l in 0..MAXL {
            if let (Some(x), Some(b)) = (bp.get_mut(l), dial.bp(l)) {
                *x = b;
            }
            if let (Some(fl), Some(&r), Some(&u)) = (full.get_mut(l), w.get(l), upto.get(l)) {
                *fl = mask.has(l) && u > 0 && r == 0;
            }
        }
        Ok(Self { o: Some(*o), bp, lv, upto: *upto, full, end: out, mask, cum: cumv })
    }

    /// CHANGE S1 (crate v2.6): the record's own walk summary of a level (its counters: past_limit, and on a strict view lpp_rejected,
    /// lpp_unknown, locked_secs, band_clipped), whatever quantity a view later asks.
    #[must_use]
    pub fn ended(&self, level: usize) -> LevelFill {
        self.end.get(level).copied().unwrap_or_default()
    }

    fn level(&self, level: usize, qty: Units) -> Result<&[PathBlk], FillErr> {
        if !self.mask.has(level) {
            return Err(FillErr::DialInvalid);
        }
        let up = self.upto.get(level).copied().unwrap_or(0);
        if qty > up && self.full.get(level).copied().unwrap_or(false) {
            return Err(FillErr::Overflow);
        }
        Ok(self.lv.get(level).map_or(&[][..], Vec::as_slice))
    }

    /// The level's fill for `qty` units (<= the recorded `upto`): cum_k(qty) = min(qty, C_k) (design 4.4). O(log blocks). The skip
    /// counters (no_trade, past_limit, zero_take_blocks, clipped) are the record's.
    ///
    /// # Errors
    /// [`FillErr::DialInvalid`] for a level not recorded; [`FillErr::Overflow`] for a quantity beyond a record that stopped at `upto`.
    pub fn view(&self, level: usize, qty: Units, steps: &mut u32) -> Result<LevelFill, FillErr> {
        let v = self.level(level, qty)?;
        let base = self.end.get(level).copied().unwrap_or_default();
        // The first block whose cumulative capacity reaches qty.
        let k = search(v, |x| x.c < qty, steps);
        let mut f = LevelFill { steps: base.steps, no_trade: base.no_trade, past_limit: base.past_limit, past_cap: base.past_cap, clipped: base.clipped, zero_take_blocks: base.zero_take_blocks, unpaid: 0, ..LevelFill::default() };
        if qty == 0 || v.is_empty() {
            f.rest_at_stop = qty;
            return Ok(f);
        }
        let (prev_c, prev_v) = if k == 0 { (0, 0) } else { v.get(k - 1).map_or((0, 0), |x| (x.c, x.v)) };
        if let Some(x) = v.get(k) {
            let part = qty - prev_c;
            let add = i64::try_from(part).map_err(|_| FillErr::Overflow)?.checked_mul(x.bf.px).ok_or(FillErr::Overflow)?;
            f.units = qty;
            f.value = prev_v.checked_add(add).ok_or(FillErr::Overflow)?;
            f.blocks_filled = u32::try_from(k + 1).map_err(|_| FillErr::Overflow)?;
            f.last = Some(BlockFill { units: part, cum: qty, ..x.bf });
            f.known_at = x.bf.block.known;
            f.filled_secs = x.filled_secs;
            f.one_price_secs = x.one_price_secs;
        } else {
            let x = v.last().ok_or(FillErr::Overflow)?;
            f.units = x.c;
            f.value = x.v;
            f.blocks_filled = u32::try_from(v.len()).map_err(|_| FillErr::Overflow)?;
            f.last = Some(x.bf);
            f.known_at = x.bf.block.known;
            f.filled_secs = x.filled_secs;
            f.one_price_secs = x.one_price_secs;
        }
        if let Some(x0) = v.first() {
            let mut b0 = x0.bf;
            if qty < b0.units {
                b0.units = qty;
                b0.cum = qty;
            }
            f.first = Some(b0);
            f.e0 = b0.block.at;
            f.e1 = b0.block.known;
        }
        f.rest_at_stop = qty - f.units;
        Ok(f)
    }

    /// REQ 70a's cut (S12): the whole units of the level's `qty`-unit fill whose cumulative value fits the payable exposure `pay`, as an
    /// in-walk cut at `pay` gives them. O(log blocks): ONE search a call (D27: at most 14 probes at 11,250 blocks).
    ///
    /// # Errors
    /// As [`FillPath::view`].
    pub fn payable(&self, level: usize, qty: Units, pay: Paisa, steps: &mut u32) -> Result<Units, FillErr> {
        let v = self.level(level, qty)?;
        let unit = self.o.map_or(1, |o| o.unit.max(1));
        // The first block where the qty-unit fill ends (C_k >= qty) or its cumulative value passes `pay` (V_k > pay). C and V are both
        // non-decreasing in k, so "C_k < qty and V_k <= pay" is true then false: one partition point serves both cuts (G0 repair 1).
        let k = search(v, |x| x.c < qty && x.v <= pay, steps);
        let (pc, pv) = if k == 0 { (0, 0) } else { v.get(k - 1).map_or((0, 0), |x| (x.c, x.v)) };
        // Every recorded block fits whole and the record ends short of (or at) qty.
        let Some(x) = v.get(k) else {
            return Ok(v.last().map_or(0, |x| x.c).min(qty));
        };
        let room = pay.checked_sub(pv).ok_or(FillErr::Overflow)?.max(0);
        let fit = if x.bf.px > 0 { Units::try_from(room / x.bf.px).map_err(|_| FillErr::Overflow)? / unit * unit } else { 0 };
        if x.c >= qty {
            // The qty-unit fill ends in this block (pc < qty here): its part, whole when the rest of `pay` covers it.
            let part = qty - pc;
            let add = i64::try_from(part).map_err(|_| FillErr::Overflow)?.checked_mul(x.bf.px).ok_or(FillErr::Overflow)?;
            if pv.checked_add(add).ok_or(FillErr::Overflow)? <= pay {
                return Ok(qty);
            }
            return Ok(pc.saturating_add(fit.min(part)));
        }
        // A block the qty-unit fill uses in full, whose cumulative value passes `pay` first: the whole units the rest of `pay` buys in it.
        Ok(pc.saturating_add(fit.min(x.c.saturating_sub(pc))).min(qty))
    }

    /// The level's `qty`-unit entry as if its stop were `t` (a stop exit that wins over the signal exit, design 4.6): the recorded blocks
    /// ending at or before `t`, plus the lattice block straddling `t` re-read clipped at `t` (one O(1) read, its reading per
    /// [`Tape::block_from`]). O(log blocks): ONE search a call, its probes in `steps` (D27: at most 14 at 11,250 blocks); the one O(1)
    /// re-read is a block read, counted in the result's [`Steps`] (`blocks` and `table_reads`, as [`walk`] counts a block).
    ///
    /// # Errors
    /// As [`FillPath::view`]; [`FillErr::Overflow`] when the re-read block's volume leaves its type.
    pub fn cut_at<R: Row>(&self, tape: &Tape<'_, R>, level: usize, qty: Units, t: Us, steps: &mut u32) -> Result<LevelFill, FillErr> {
        let v = self.level(level, qty)?;
        let o = self.o.ok_or(FillErr::Overflow)?;
        if t >= o.stop {
            return self.view(level, qty, steps);
        }
        // CHANGE FA1: the straddling block re-read below reads its capacity for the recorded order's anchor.
        let tape = &tape.for_order(o.anchor);
        let unit = o.unit.max(1);
        // The first block that ends after t or where the qty-unit fill ends: block ends and cumulative units both rise with k, so
        // "ends by t and C_k < qty" is true then false, one partition point (G0 repair 1). The fill ends in that block when it ends by t.
        let k = search(v, |x| x.bf.block.to <= t && x.c < qty, steps);
        let upto = if v.get(k).is_some_and(|x| x.bf.block.to <= t) { k + 1 } else { k };
        // The head blocks the qty-unit fill takes (every recorded block ending by t when the fill does not end there).
        let head = v.get(..upto).unwrap_or(&[]);
        let mut f = LevelFill::default();
        let mut got = 0_u64;
        if upto > 0 {
            let x = head.get(upto - 1).ok_or(FillErr::Overflow)?;
            got = x.c.min(qty);
            let (pc, pv) = if upto >= 2 { head.get(upto - 2).map_or((0, 0), |y| (y.c, y.v)) } else { (0, 0) };
            let add = i64::try_from(got - pc).map_err(|_| FillErr::Overflow)?.checked_mul(x.bf.px).ok_or(FillErr::Overflow)?;
            f.units = got;
            f.value = pv.checked_add(add).ok_or(FillErr::Overflow)?;
            f.blocks_filled = u32::try_from(upto).map_err(|_| FillErr::Overflow)?;
            f.last = Some(BlockFill { units: got - pc, cum: got, ..x.bf });
            f.known_at = x.bf.block.known;
            f.filled_secs = x.filled_secs;
            f.one_price_secs = x.one_price_secs;
            if let Some(x0) = head.first() {
                f.first = Some(if upto == 1 { BlockFill { units: got, cum: got, ..x0.bf } } else { x0.bf });
                f.e0 = x0.bf.block.at;
                f.e1 = x0.bf.block.known;
            }
        }
        if got < qty && t > o.anchor {
            // CHANGE C9: the lattice of this tape's block length.
            let blk = tape.block_us();
            // The lattice block straddling t (if t is not on the lattice), re-read clipped at t.
            let off = (t - o.anchor) % blk;
            if off != 0 {
                let at = t - off;
                let after_head = head.last().is_none_or(|x| x.bf.block.to <= at);
                if after_head {
                    f.steps.blocks = f.steps.blocks.saturating_add(1);
                    f.steps.table_reads = f.steps.table_reads.saturating_add(1);
                    // CHANGE S1: on a strict view the straddling block's band-locked seconds give no volume, as in the walk
                    let b = match tape.strict() {
                        Some(s) => {
                            let (lock, _) = tape.known_band(&s, at, o.side);
                            tape.block_locked(tape.first_at(at), at, t, o.side, lock)?.0
                        }
                        None => tape.block(at, t)?,
                    };
                    let k = u32::try_from((at - o.anchor) / blk).map_err(|_| FillErr::Overflow)?;
                    let px = b.px(o.side);
                    // CHANGE K2: the straddling block's limit as re-sent for it (CHANGE S1: the strict reading's on a strict view)
                    let ro = match tape.strict() {
                        Some(s) => resend_strict(tape, &s, &o, at).0,
                        None => resend_order(tape, &o, at),
                    };
                    let past = ro.limit.is_some_and(|lim| match o.side {
                        Side::Buy => px > lim,
                        Side::Sell => px < lim,
                    });
                    let bp = self.bp.get(level).copied().unwrap_or(0);
                    if b.n > 0 && !past {
                        // CHANGE K1: on a carrying view the straddling block's take is the entitlement carried to it (every priced block within
                        // the limit before it, read from the record's cumulative series, plus this clipped block) less the units already filled.
                        let cap_here = if tape.carry() && !cfg!(feature = "m_v31_carry_per_block") {
                            let cv = self.cum.get(level).map_or(&[][..], Vec::as_slice);
                            let i = search(cv, |x| x.0 < at, steps);
                            let before = if i == 0 { 0 } else { cv.get(i - 1).map_or(0, |x| x.1) };
                            let c = before.checked_add(block_cap(&b, bp, 1)?).ok_or(FillErr::Overflow)?;
                            (c / unit * unit).saturating_sub(got)
                        } else {
                            block_cap(&b, bp, unit)?
                        };
                        let take = cap_here.min(qty - got);
                        if take > 0 {
                            let add = i64::try_from(take).map_err(|_| FillErr::Overflow)?.checked_mul(px).ok_or(FillErr::Overflow)?;
                            got += take;
                            f.units = got;
                            f.value = f.value.checked_add(add).ok_or(FillErr::Overflow)?;
                            let bf = BlockFill { k, block: b, units: take, px, cum: got, why: Why::Filled };
                            if f.blocks_filled == 0 {
                                f.first = Some(bf);
                                f.e0 = b.at;
                                f.e1 = b.known;
                            }
                            f.blocks_filled = f.blocks_filled.saturating_add(1);
                            f.last = Some(bf);
                            f.known_at = f.known_at.max(b.known);
                            f.filled_secs = f.filled_secs.saturating_add(u32::from(b.n_secs));
                            f.one_price_secs = f.one_price_secs.saturating_add(u32::from(b.one_price));
                        }
                    }
                }
            }
        }
        f.rest_at_stop = qty - got;
        Ok(f)
    }

    /// The level's recorded busy span: [first block start, last block end) (S17's busy-until check), (0, 0) when it filled nothing.
    #[must_use]
    pub fn span(&self, level: usize) -> (Us, Us) {
        match self.lv.get(level) {
            Some(v) => match (v.first(), v.last()) {
                (Some(a), Some(b)) => (a.bf.block.at, b.bf.block.to),
                _ => (0, 0),
            },
            None => (0, 0),
        }
    }

    /// The number of recorded filling blocks of a level.
    #[must_use]
    pub fn blocks(&self, level: usize) -> usize {
        self.lv.get(level).map_or(0, Vec::len)
    }

    /// CRATE v2.4 (spikefade v3.2, MAIN-RULINGS-ENGINE-v22 addendum 10, change O1): the EXECUTED ORDERS of the level fill `f` (a
    /// [`FillPath::view`] or [`FillPath::cut_at`] result of THIS path at THIS level): every filling block is an order (the rest is re-sent to
    /// the next block, REQ 70a) and inside a block every NSE freeze slice of the units it filled is an order of its own: the sum over the
    /// filled blocks of ceil(block units / `slice`), where `slice` = the largest whole-lot quantity one order may carry (0 = no freeze limit
    /// known: one order a block). The first `blocks_filled - 1` blocks of a view or a cut are this record's blocks taken in full; the last
    /// one carries `f.last`'s units (a view's partial block, a cut's partial head block or its re-read straddling block). O(1) when the
    /// whole fill fits one slice (every block then fits it); otherwise O(blocks filled).
    ///
    /// # Errors
    /// [`FillErr::DialInvalid`] for a level not recorded; [`FillErr::Overflow`] when `f` claims more full blocks than the record holds.
    pub fn sliced_orders(&self, level: usize, f: &LevelFill, slice: Units) -> Result<u32, FillErr> {
        if !self.mask.has(level) {
            return Err(FillErr::DialInvalid);
        }
        let nb = f.blocks_filled;
        // MUTANT m_v32_orders_blocks_only: the freeze slices ignored (one order a filling block whatever its size)
        if nb == 0 || slice == 0 || f.units <= slice || cfg!(feature = "m_v32_orders_blocks_only") {
            return Ok(nb);
        }
        let v = self.lv.get(level).map_or(&[][..], Vec::as_slice);
        let full = usize::try_from(nb - 1).map_err(|_| FillErr::Overflow)?;
        if v.len() < full {
            return Err(FillErr::Overflow);
        }
        let mut n: u64 = 0;
        let mut prev: Units = 0;
        for x in v.iter().take(full) {
            let u = x.c.checked_sub(prev).ok_or(FillErr::Overflow)?;
            prev = x.c;
            n = n.checked_add(crate::legs::freeze_slices(u, slice)).ok_or(FillErr::Overflow)?;
        }
        n = n.checked_add(crate::legs::freeze_slices(f.last.map_or(0, |b| b.units), slice).max(1)).ok_or(FillErr::Overflow)?;
        u32::try_from(n).map_err(|_| FillErr::Overflow)
    }
}
