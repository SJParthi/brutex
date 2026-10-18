//! Synthetic FIB-6 fixtures, shared by `tests/fib_lattice.rs`,
//! `tests/fib_score.rs` and the lattice bench rows. Every value is invented:
//! the prices are round synthetic paisa no market printed, the days are
//! counted from an arbitrary origin, and nothing here is a market fact.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc,
    clippy::type_complexity,
    reason = "a fixture that cannot be built is a failed test; each including target uses part of it"
)]

use runner::fib::lattice::{Decision, Partition, RowTable};
use runner::fib::score::{Calendar, DayKeys};
use runner::fib::types::{ExitReason, SlotOutcome, TradeFlags};
use vocab::fib_cube::{
    Coords, CutList, EntryVariants, Generation, GenerationParts, ROW_COUNT, Ratio, RowId, Search,
    Source, VariantSet,
};

/// A deterministic generator (splitmix64).
pub(crate) struct Seeded(pub(crate) u64);

impl Seeded {
    /// The next 64 random bits.
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (`n > 0`).
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A Zipf-skewed value in `0..n`: value `v` with weight `1 / (v + 1)`,
    /// drawn by inverting the cumulative harmonic weights in integers.
    pub(crate) fn zipf(&mut self, n: u64) -> u64 {
        let scale = 1_u64 << 20;
        let weights: Vec<u64> = (1..=n).map(|v| scale / v).collect();
        let total: u64 = weights.iter().sum();
        let mut pick = self.below(total);
        for (value, weight) in weights.iter().enumerate() {
            if pick < *weight {
                return value as u64;
            }
            pick -= weight;
        }
        n - 1
    }
}

/// A generation of `search` with `measured` values on every measured row,
/// the mover-rank `edges`, `exits` exit variants, one width (one minute) and
/// the three entry styles.
pub(crate) fn generation_with_edges(
    search: Search,
    measured: u8,
    edges: Vec<u16>,
    exits: u16,
) -> Generation {
    let mut cuts = Vec::new();
    for id in RowId::ALL {
        if id.row().source != Source::Measured || id == RowId::MoverRank {
            continue;
        }
        let (timeframe, list) = if id == RowId::TimeOfDay {
            (
                Some(1),
                (1..u64::from(measured))
                    .map(|c| Ratio::whole(c * 60))
                    .collect(),
            )
        } else {
            (None, (1..u64::from(measured)).map(Ratio::whole).collect())
        };
        cuts.push(CutList {
            row: id,
            timeframe,
            base_kind: None,
            cuts: list,
        });
    }
    Generation::new(GenerationParts {
        search,
        version: 1,
        birth_step: 0,
        timeframes: vec![1],
        rank_edges: edges,
        cuts,
        exit_set: VariantSet {
            version: 1,
            digest: [0x33; 32],
            count: exits,
        },
        entry_variants: EntryVariants {
            version: 1,
            digest: [0x44; 32],
            styles: vec![0, 1, 2],
        },
        others: Vec::new(),
    })
    .expect("the fixture is a valid generation")
}

/// [`generation_with_edges`] with `groups` mover-rank groups (edges 1, 2, ...).
pub(crate) fn generation(search: Search, measured: u8, groups: u8, exits: u16) -> Generation {
    let edges = (1..u16::from(groups)).collect();
    generation_with_edges(search, measured, edges, exits)
}

/// The first side value of a search: `long_with_gainer` or `short_with_loser`.
pub(crate) fn first_side(search: Search) -> u8 {
    match search {
        Search::Long => 0,
        Search::Short => 1,
    }
}

/// The other side value of a search.
pub(crate) fn second_side(search: Search) -> u8 {
    match search {
        Search::Long => 3,
        Search::Short => 2,
    }
}

/// The base cell: every row at value 0, the side at the search's first.
pub(crate) fn base_cell(search: Search) -> Coords {
    let mut cell = [0; ROW_COUNT];
    cell[RowId::Side.index()] = first_side(search);
    cell
}

/// A partition of entry style 0, timeframe 0 and exits `0..exits`.
pub(crate) fn partition(search: Search, exits: u8) -> Partition {
    Partition {
        search,
        entry_style: 0,
        timeframe: 0,
        exits: 0..exits,
    }
}

/// One decision.
pub(crate) fn decision(coords: Coords, slot: u32, day: u32, second: u32, id: u64) -> Decision {
    Decision {
        coords,
        slot,
        day,
        second,
        id,
    }
}

/// Pushes `months` (each a list of decisions with their outcome indices, any
/// order) into a fresh table, each month sorted into (slot, id) order first.
pub(crate) fn table_of(
    generation: &Generation,
    partition: Partition,
    months: u32,
    data: Vec<(u32, Vec<(Decision, Vec<u32>)>)>,
) -> RowTable {
    let mut table = RowTable::new(*generation.layout(), partition, months).expect("a table");
    for (month, mut decisions) in data {
        decisions.sort_by_key(|(d, _)| (d.slot, d.id));
        let outcomes: Vec<u32> = decisions.iter().flat_map(|(_, o)| o.clone()).collect();
        let decisions: Vec<Decision> = decisions.into_iter().map(|(d, _)| d).collect();
        table
            .push_month(month, &decisions, &outcomes)
            .expect("a valid month");
    }
    table
}

/// A priced slot of a long or short trade: `qty` shares, entry and exit
/// values in paisa, risk per share in paisa, and the seconds.
pub(crate) fn priced(
    qty: u64,
    entry_value: i128,
    exit_value: i128,
    risk: i64,
    entry_second: u32,
    exit_second: u32,
) -> SlotOutcome {
    SlotOutcome {
        filled_qty: qty,
        entry_value_paisa: entry_value,
        exit_value_paisa: exit_value,
        risk_per_share_paisa: risk,
        first_entry_second: entry_second,
        last_entry_second: entry_second,
        exit_second,
        exit_reason: ExitReason::Forced,
        mfe_to_exit_paisa: risk,
        mae_to_exit_paisa: -risk,
        mfe_to_horizon_paisa: risk,
        mae_to_horizon_paisa: -risk,
        flags: TradeFlags::NONE,
    }
}

/// A calendar of `count` civil days from `first`, every day tradable, with
/// synthetic period keys: a week of 7 days, a fortnight of 15, a month of 30,
/// a quarter of 90, a half-year of 180, a year of 360 (invented: real keys
/// come from the calendar authority).
pub(crate) fn calendar(first: u32, count: u32) -> Calendar {
    let days = (0..count)
        .map(|d| {
            Some(DayKeys {
                tradable_index: d,
                week: d / 7,
                fortnight: d / 15,
                month: d / 30,
                quarter: d / 90,
                half_year: d / 180,
                year: d / 360,
            })
        })
        .collect();
    Calendar::new(first, days).expect("a calendar")
}
