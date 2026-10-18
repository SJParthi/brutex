//! FIB-2 PART A: the own-past histograms, the bank, the observed tick, the gap
//! day, the touch step and the checkpoint bodies (D-6320 to D-6328). Every
//! test here is named in `docs/04-invariants.md` beside the invariant it
//! proves. Synthetic values only: no price here can be mistaken for a real one.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::panic,
    clippy::similar_names,
    reason = "a test that cannot panic cannot fail"
)]

use indicators::fibflag::hist::{
    BANK_MAGIC, BASE_BINS, BankShape, Basis, CODEC_V1, CODEC_V2, CellKey, DayFeed, DecisionTick,
    Dims, GapDay, HistBank, HistRefusal, Measure, OWN_PAST_MAGIC, ObservedTick, OwnPast, Scope,
    Silence, TICK_MAGIC, TickCensus, TickComparison, UNSPLIT_CELL_BYTES, decision_tick,
    plan_width_passes, silence_in_trade, step_bp, touches,
};
use indicators::fibflag::types::BaseKind;
use vocab::fib_cube::{Ratio, Search};

/// A deterministic generator (splitmix64).
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> i64 {
        i64::try_from(self.next() % n).unwrap()
    }
}

/// A histogram with one committed day holding `values`.
fn committed(values: &[i64]) -> OwnPast {
    let mut past = OwnPast::empty();
    for value in values {
        past.observe(*value).unwrap();
    }
    past.commit_day().unwrap();
    past
}

/// The checkpoint body of a histogram.
fn body(past: &OwnPast) -> Vec<u8> {
    let mut out = Vec::new();
    past.encode(&mut out).unwrap();
    out
}

/// One canonical varint, written independently of the module.
fn varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let low = u8::try_from(value & 0x7F).unwrap();
        value >>= 7;
        if value == 0 {
            out.push(low);
            return;
        }
        out.push(low | 0x80);
    }
}

// ------------------------------------------------------------ OwnPast --

#[test]
fn permille_is_monotone_in_the_value() {
    let mut mix = Mix(7);
    let mut past = OwnPast::empty();
    for _ in 0..5 {
        for _ in 0..2_000 {
            past.observe(mix.below(1 << 30)).unwrap();
        }
        past.commit_day().unwrap();
    }
    let mut probes: Vec<i64> = (0..5_000).map(|_| mix.below(1 << 31)).collect();
    probes.extend([0, 1, i64::MAX]);
    probes.sort_unstable();
    let mut last = (0_u16, Ratio::whole(0));
    for value in probes {
        let permille = past.permille(value).unwrap();
        let rank = past.mid_rank(value).unwrap();
        assert!(permille >= last.0, "{value}");
        assert_ne!(rank.cmp_exact(last.1), core::cmp::Ordering::Less, "{value}");
        assert!(permille <= 1_000);
        last = (permille, rank);
    }
}

#[test]
fn same_day_observations_are_invisible_until_commit() {
    let mut past = committed(&[10, 20, 30]);
    let before = (past.mid_rank(25).unwrap(), past.median().unwrap());
    for value in [25, 25, 25, 1_000, 0] {
        past.observe(value).unwrap();
        assert_eq!(
            (past.mid_rank(25).unwrap(), past.median().unwrap()),
            before,
            "a value staged today changes no answer today"
        );
    }
    assert_eq!(past.staged_values(), 5);
    assert_eq!(past.total(), 3);
    past.commit_day().unwrap();
    assert_eq!(past.total(), 8);
    assert_ne!(past.mid_rank(25).unwrap(), before.0);
    assert_eq!(past.committed_days(), 2);
    assert_eq!(past.max(), Some(1_000));
}

#[test]
fn discard_leaves_the_bytes_identical() {
    let mut past = committed(&[3, 300, 30_000]);
    past.observe_censored();
    past.commit_day().unwrap();
    let before = past.clone();
    let bytes = body(&past);
    for value in [7, 7, 700, 70_000_000] {
        past.observe(value).unwrap();
    }
    past.observe_censored();
    assert_eq!(
        past.encode(&mut Vec::new()),
        Err(HistRefusal::StagedNotEmpty)
    );
    past.discard_day();
    assert_eq!(past, before);
    assert_eq!(body(&past), bytes);
    // A day that staged nothing and is discarded leaves it too.
    past.discard_day();
    assert_eq!(past, before);
}

#[test]
fn no_own_past_before_the_first_committed_day_and_an_answer_after_it() {
    let mut past = OwnPast::empty();
    assert_eq!(past.mid_rank(5), Err(HistRefusal::NoOwnPast));
    assert_eq!(past.permille(5), Err(HistRefusal::NoOwnPast));
    assert_eq!(past.median(), Err(HistRefusal::NoOwnPast));
    assert_eq!(past.value_at_rank(1), Err(HistRefusal::NoOwnPast));
    assert_eq!(past.quantile(1, 2), Err(HistRefusal::NoOwnPast));
    past.observe(5).unwrap();
    assert_eq!(
        past.mid_rank(5),
        Err(HistRefusal::NoOwnPast),
        "staged is not past"
    );
    past.commit_day().unwrap();
    assert_eq!(past.mid_rank(5), Ok(Ratio::mid_rank(0, 1, 1).unwrap()));
    assert_eq!(past.committed_days(), 1);
}

/// FM-F015: a cell with committed days and no observations answers
/// `no_observations` — never a default bucket — and a consumer counts each
/// refused candidate, so candidates = rows + refusals.
#[test]
fn cell_with_committed_days_and_no_observations_answers_none() {
    let mut past = OwnPast::empty();
    past.commit_day().unwrap();
    past.commit_day().unwrap();
    assert_eq!(past.committed_days(), 2);
    assert_eq!(past.mid_rank(1), Err(HistRefusal::NoObservations));
    assert_eq!(past.permille(1), Err(HistRefusal::NoObservations));
    assert_eq!(past.median(), Err(HistRefusal::NoObservations));
    assert_eq!(past.censored_median(), Err(HistRefusal::NoObservations));
    assert_eq!(past.value_at_rank(1), Err(HistRefusal::NoObservations));
    assert_eq!(past.max(), None);
    // A StopBp-empty candidate is refused and counted.
    let (mut rows, mut refused) = (0, 0);
    let mut stop_bp = OwnPast::empty();
    stop_bp.commit_day().unwrap();
    for (day, stops) in [vec![12, 40], vec![25], vec![9, 9, 30]].iter().enumerate() {
        for stop in stops {
            match stop_bp.mid_rank(*stop) {
                Ok(_) => rows += 1,
                Err(HistRefusal::NoObservations) => refused += 1,
                Err(other) => panic!("{other}"),
            }
            stop_bp.observe(*stop).unwrap();
        }
        stop_bp.commit_day().unwrap();
        assert!(day < 3);
    }
    assert_eq!((rows, refused), (4, 2), "the first day's two are refused");
    assert_eq!(rows + refused, 6, "candidates = rows + refusals");
}

/// 0, 1, 2^40 and `i64::MAX` each land in a bin of the constant table: no cap,
/// no overflow bin, and `i64::MAX` ranks like any other value (17:15 IST
/// item 1's golden).
#[test]
fn zero_one_two_to_the_forty_and_i64_max_rank_like_any_other_value() {
    let values = [0, 1, 1 << 40, i64::MAX];
    let past = committed(&values);
    let ranks: Vec<(u64, u64)> = values
        .iter()
        .map(|value| {
            let rank = past.mid_rank(*value).unwrap();
            (rank.num(), rank.den())
        })
        .collect();
    assert_eq!(ranks, [(1, 8), (3, 8), (5, 8), (7, 8)], "golden");
    assert_eq!(past.max(), Some(i64::MAX));
    assert_eq!(
        past.value_at_rank(4).unwrap(),
        15 << 59,
        "the top bin's edge"
    );
    assert_eq!(past.value_at_rank(3).unwrap(), 1 << 40);
    assert_eq!(past.leaves(), BASE_BINS);
    assert_eq!(past.refused_full(), 0);
    let mut negative = OwnPast::empty();
    assert_eq!(
        negative.observe(-1),
        Err(HistRefusal::NegativeValue { value: -1 })
    );
    assert_eq!(negative.refused_negative(), 1);
    assert_eq!(
        negative.mid_rank(-1),
        Err(HistRefusal::NegativeValue { value: -1 })
    );
    assert_eq!(
        negative.leaf_bounds(-1),
        Err(HistRefusal::NegativeValue { value: -1 })
    );
    negative.commit_day().unwrap();
    assert_eq!(negative.mid_rank(0), Err(HistRefusal::NoObservations));
}

#[test]
fn a_value_exactly_on_a_bin_edge_opens_its_bin() {
    // 16..=17 share a bin of width 2; 18 opens the next one.
    let past = committed(&[16, 17, 18]);
    assert_eq!(past.leaf_bounds(16).unwrap(), (16, 2));
    assert_eq!(past.leaf_bounds(17).unwrap(), (16, 2));
    assert_eq!(past.leaf_bounds(18).unwrap(), (18, 2));
    assert_eq!(past.leaf_bounds(0).unwrap(), (0, 1));
    assert_eq!(past.leaf_bounds(7).unwrap(), (7, 1));
    assert_eq!(past.leaf_bounds(1_000).unwrap(), (960, 64));
    let below = past.mid_rank(17).unwrap();
    let on = past.mid_rank(18).unwrap();
    assert_eq!((below.num(), below.den()), (2, 6), "16 and 17 share a bin");
    assert_eq!((on.num(), on.den()), (5, 6), "18 is the next bin's edge");
    assert_eq!(past.value_at_rank(2).unwrap(), 16);
    assert_eq!(past.value_at_rank(3).unwrap(), 18);
}

#[test]
fn the_median_query_is_the_lower_middle_bin_s_edge() {
    let past = committed(&[1, 2, 3, 4]);
    assert_eq!(past.median().unwrap(), 2, "the lower middle of four");
    let odd = committed(&[5, 1, 3]);
    assert_eq!(odd.median().unwrap(), 3);
    let wide = committed(&[100, 101, 1_000_000]);
    assert_eq!(wide.median().unwrap(), 96, "the bin's lower edge");
    assert_eq!(wide.quantile(1, 2).unwrap(), wide.median().unwrap());
    assert_eq!(wide.quantile(1, 1).unwrap(), 983_040);
    assert_eq!(
        wide.quantile(0, 2),
        Err(HistRefusal::QuantileInvalid { num: 0, den: 2 })
    );
    assert_eq!(
        wide.quantile(3, 2),
        Err(HistRefusal::QuantileInvalid { num: 3, den: 2 })
    );
    assert_eq!(
        wide.value_at_rank(0),
        Err(HistRefusal::RankOutOfRange { rank: 0, total: 3 })
    );
    assert_eq!(
        wide.value_at_rank(4),
        Err(HistRefusal::RankOutOfRange { rank: 4, total: 3 })
    );
}

/// FM-F012: `quantile(p)` is the lower edge of the leaf holding the
/// nearest-rank value, found by one bisection whose probes are bounded by the
/// constant table.
#[test]
fn quantile_is_lower_edge_and_bounded_probes() {
    let mut mix = Mix(12);
    let mut values: Vec<i64> = (0..10_000).map(|_| mix.below(1 << 48)).collect();
    let past = committed(&values);
    values.sort_unstable();
    let bound = u64::from(BASE_BINS.next_power_of_two().trailing_zeros()) + 1;
    for (num, den) in [(1, 1_000), (1, 4), (1, 2), (3, 4), (999, 1_000), (1, 1)] {
        let rank = (num * 10_000_u64).div_ceil(den);
        let exact = values[usize::try_from(rank - 1).unwrap()];
        let before = past.work();
        let got = past.quantile(num, den).unwrap();
        let probes = past.work() - before;
        let (edge, width) = past.leaf_bounds(exact).unwrap();
        assert_eq!(got, edge, "{num}/{den}");
        assert!(got <= exact && exact - got < i64::try_from(width).unwrap());
        assert!(probes <= bound, "{probes} probes > {bound}");
    }
    assert_eq!(bound, 10);
}

/// FM-F020: a total past `u32::MAX / 1000` keeps an exact permille (the
/// numerator is wide), matching a brute-force count.
#[test]
fn a_total_of_4_294_968_matches_brute_force() {
    let mut past = OwnPast::empty();
    let mut values = Vec::with_capacity(4_294_968);
    for n in 0..4_294_968_i64 {
        let value = n % 1_000;
        values.push(value);
        past.observe(value).unwrap();
    }
    past.commit_day().unwrap();
    assert_eq!(past.total(), 4_294_968);
    for probe in [0, 1, 7, 499, 500, 998, 999] {
        let below = values.iter().filter(|v| **v < probe).count() as u128;
        let at = values.iter().filter(|v| **v == probe).count() as u128;
        let brute = (2 * below + at) * 1_000 / (2 * 4_294_968);
        // Values below 16 have a bin each; above, a bin may hold several.
        let (edge, width) = past.leaf_bounds(probe).unwrap();
        let in_bin = values
            .iter()
            .filter(|v| **v >= edge && **v < edge + i64::try_from(width).unwrap())
            .count() as u128;
        let below_bin = values.iter().filter(|v| **v < edge).count() as u128;
        let binned = (2 * below_bin + in_bin) * 1_000 / (2 * 4_294_968);
        assert_eq!(u128::from(past.permille(probe).unwrap()), binned);
        if width == 1 {
            assert_eq!(binned, brute, "an exact bin agrees with brute force");
        }
    }
}

/// FM-F020: a bin is refused by name at `u32::MAX`, never wrapped; the
/// refusal is counted.
#[test]
fn a_full_bin_is_refused_by_name_and_counted() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(OWN_PAST_MAGIC);
    bytes.extend_from_slice(&CODEC_V1.to_le_bytes());
    bytes.extend_from_slice(&505_u32.to_le_bytes());
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&u64::from(u32::MAX - 1).to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    varint(&mut bytes, 5);
    bytes.extend_from_slice(&[0; 16]);
    varint(&mut bytes, 1);
    varint(&mut bytes, 19);
    varint(&mut bytes, u64::from(u32::MAX - 1));
    let mut past = OwnPast::decode(&bytes).unwrap();
    assert_eq!(past.total(), u64::from(u32::MAX - 1));
    past.observe(5).unwrap();
    assert_eq!(past.observe(5), Err(HistRefusal::CountFull));
    assert_eq!(past.observe(4), Ok(()), "another bin has room");
    assert_eq!(past.refused_full(), 1);
    past.commit_day().unwrap();
    assert_eq!(past.total(), u64::from(u32::MAX) + 1);
    assert_eq!(past.observe(5), Err(HistRefusal::CountFull));
    assert_eq!(past.refused_full(), 2);
    let full = body(&past);
    assert_eq!(OwnPast::decode(&full).unwrap(), past, "a full bin decodes");
}

#[test]
fn the_committed_day_count_is_refused_at_its_top() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(OWN_PAST_MAGIC);
    bytes.extend_from_slice(&CODEC_V1.to_le_bytes());
    bytes.extend_from_slice(&505_u32.to_le_bytes());
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&[0; 16]);
    varint(&mut bytes, 0);
    bytes.extend_from_slice(&[0; 16]);
    varint(&mut bytes, 0);
    let mut past = OwnPast::decode(&bytes).unwrap();
    past.observe(3).unwrap();
    let before = past.clone();
    assert_eq!(past.commit_day(), Err(HistRefusal::DaysFull));
    assert_eq!(past, before, "nothing merged");
    let mut bank = HistBank::new(shape());
    bank.cell_mut(&CellKey::stock(Measure::AbsMoveBp))
        .map(|cell| *cell = OwnPast::decode(&bytes).unwrap())
        .unwrap();
    bank.observe(&CellKey::stock(Measure::MaxSecondGap), 9)
        .unwrap();
    let before = bank.clone();
    assert_eq!(bank.commit_day(), Err(HistRefusal::DaysFull));
    assert_eq!(bank, before, "no cell committed");
}

#[test]
fn the_censored_median_is_labelled_when_censored_reach_the_resolved() {
    let mut durations = OwnPast::empty();
    for value in [300, 600, 900] {
        durations.observe(value).unwrap();
    }
    durations.observe_censored();
    durations.observe_censored();
    durations.commit_day().unwrap();
    let median = durations.censored_median().unwrap();
    assert_eq!(median.value, 576, "600's bin edge");
    assert!(!median.censored, "two censored against three resolved");
    durations.observe_censored();
    assert!(!durations.censored_median().unwrap().censored, "staged");
    durations.commit_day().unwrap();
    assert_eq!(durations.censored(), 3);
    assert!(durations.censored_median().unwrap().censored);
}

// ---------------------------------------------------------------- splits --

#[test]
fn a_split_divides_a_bin_by_one_more_mantissa_bit_and_rebuilds_from_raw() {
    let raw = [1_000, 1_010, 1_020, 970, 960, 5];
    let mut past = committed(&raw);
    assert_eq!(past.leaf_bounds(1_000).unwrap(), (960, 64));
    let before = past.mid_rank(1_000).unwrap();
    assert_eq!(past.mid_rank(1_020).unwrap(), before, "one bin, one stamp");
    past.split(1_000, &raw).unwrap();
    assert_eq!(past.leaves(), BASE_BINS + 1);
    assert_eq!(past.leaf_bounds(1_000).unwrap(), (992, 32));
    assert_eq!(past.leaf_bounds(970).unwrap(), (960, 32));
    assert_ne!(past.mid_rank(970).unwrap(), past.mid_rank(1_000).unwrap());
    assert_eq!(past.total(), 6);
    assert_eq!(past.committed_days(), 1);
    assert_eq!(past.max(), Some(1_020));
    assert_eq!(past.value_at_rank(3).unwrap(), 960);
    assert_eq!(past.value_at_rank(4).unwrap(), 992);
    assert_eq!(past.median().unwrap(), 960);
    // Split again: the bin's two children become four.
    past.split(1_000, &raw).unwrap();
    assert_eq!(past.leaves(), BASE_BINS + 3);
    assert_eq!(past.leaf_bounds(1_020).unwrap(), (1_008, 16));
    assert_eq!(past.value_at_rank(6).unwrap(), 1_008);
    assert!(past.bytes() > UNSPLIT_CELL_BYTES);
    // A later day observes into the split leaves.
    past.observe(1_021).unwrap();
    past.commit_day().unwrap();
    assert_eq!(past.max(), Some(1_021));
    let body = body(&past);
    assert_eq!(&body[8..12], &CODEC_V2.to_le_bytes(), "v2: a depth table");
    assert_eq!(OwnPast::decode(&body).unwrap(), past);
}

#[test]
fn a_split_is_refused_by_name_and_changes_nothing() {
    let raw = [1_000, 1_010, 5];
    let mut past = committed(&raw);
    let before = past.clone();
    assert_eq!(
        past.split(1_000, &[1_000, 5]),
        Err(HistRefusal::SplitRawMismatch)
    );
    assert_eq!(
        past.split(1_000, &[1_000, 1_011, 5]),
        Err(HistRefusal::SplitRawMismatch),
        "same counts, another maximum"
    );
    assert_eq!(
        past.split(1_000, &[1_000, -1, 5]),
        Err(HistRefusal::NegativeValue { value: -1 })
    );
    assert_eq!(
        past.split(-3, &raw),
        Err(HistRefusal::NegativeValue { value: -3 })
    );
    assert_eq!(
        past.split(5, &raw),
        Err(HistRefusal::SplitOneValueWide { value: 5 })
    );
    assert_eq!(
        past.split(0, &raw),
        Err(HistRefusal::SplitOneValueWide { value: 0 })
    );
    assert_eq!(past, before);
    // A bin of width 2 splits once, then is one value wide.
    let mut narrow = committed(&[16, 17]);
    narrow.split(16, &[16, 17]).unwrap();
    assert_eq!(narrow.leaf_bounds(17).unwrap(), (17, 1));
    assert_eq!(
        narrow.split(16, &[16, 17]),
        Err(HistRefusal::SplitOneValueWide { value: 16 })
    );
    narrow.observe(16).unwrap();
    assert_eq!(
        narrow.split(17, &[16, 17]),
        Err(HistRefusal::StagedNotEmpty)
    );
    narrow.discard_day();
    narrow.observe_censored();
    assert_eq!(
        narrow.split(17, &[16, 17]),
        Err(HistRefusal::StagedNotEmpty)
    );
}

// ------------------------------------------------------------- the codec --

#[test]
fn sparse_snapshot_round_trip_and_golden() {
    let past = committed(&[0, 5, 5, 300]);
    let mut golden = Vec::new();
    golden.extend_from_slice(b"BTXFIBH1");
    golden.extend_from_slice(&[1, 0, 0, 0]); // codec v1
    golden.extend_from_slice(&[0xF9, 0x01, 0, 0]); // 505 bins
    golden.extend_from_slice(&[3, 0, 0, 0]); // 3 sub-bits
    golden.extend_from_slice(&[1, 0, 0, 0]); // 1 committed day
    golden.extend_from_slice(&[4, 0, 0, 0, 0, 0, 0, 0]); // 4 values
    golden.extend_from_slice(&[0; 8]); // 0 censored
    golden.extend_from_slice(&[0xAC, 0x02]); // max 300
    golden.extend_from_slice(&[0; 16]); // no refusal
    golden.extend_from_slice(&[3, 0, 1, 18, 2, 46, 1]); // bins 0, 19, 66
    assert_eq!(body(&past), golden);
    assert_eq!(OwnPast::decode(&golden).unwrap(), past);
    let mut mix = Mix(99);
    for days in 0..6 {
        let mut past = OwnPast::empty();
        for _ in 0..days {
            for _ in 0..500 {
                past.observe(mix.below(1 << 40)).unwrap();
            }
            past.observe_censored();
            past.commit_day().unwrap();
        }
        let _ = past.observe(-1);
        let bytes = body(&past);
        let back = OwnPast::decode(&bytes).unwrap();
        assert_eq!(back, past);
        assert_eq!(body(&back), bytes, "byte-identical resume");
        assert!(bytes.len() < past.bytes() / 2, "sparse");
    }
}

#[test]
fn a_damaged_body_is_refused_by_name() {
    let good = body(&committed(&[0, 5, 5, 300]));
    let refused = |bytes: &[u8]| match OwnPast::decode(bytes) {
        Err(HistRefusal::SnapshotInvalid { what }) => what,
        other => panic!("{other:?}"),
    };
    assert_eq!(refused(&good[..good.len() - 1]), "truncated");
    assert_eq!(refused(&good[..3]), "truncated");
    let mut extra = good.clone();
    extra.push(0);
    assert_eq!(refused(&extra), "trailing");
    let mut magic = good.clone();
    magic[0] = b'X';
    assert_eq!(refused(&magic), "magic");
    let mut version = good.clone();
    version[8] = 3;
    assert_eq!(refused(&version), "version");
    let mut bins = good.clone();
    bins[12] = 0;
    assert_eq!(refused(&bins), "bin_table");
    let mut sub = good.clone();
    sub[16] = 4;
    assert_eq!(refused(&sub), "bin_table");
    let mut total = good.clone();
    total[24] = 5;
    assert_eq!(refused(&total), "total");
    let mut no_day = good.clone();
    no_day[20] = 0;
    assert_eq!(refused(&no_day), "total", "values with no committed day");
    let mut max = good.clone();
    max[40] = 0xAD; // 301: the same bin as 300
    assert_eq!(OwnPast::decode(&max).unwrap().max(), Some(301));
    max[40] = 0xAC;
    max[41] = 0x03; // 428: another bin
    assert_eq!(refused(&max), "max");
    let len = good.len();
    let mut zero = good.clone();
    zero[len - 1] = 0;
    assert_eq!(refused(&zero), "sparse", "a zero written as a value");
    let mut past_end = good[..len - 2].to_vec();
    varint(&mut past_end, 500);
    varint(&mut past_end, 1);
    assert_eq!(refused(&past_end), "sparse", "a position past the table");
    // An empty histogram writes max 0; a body claiming a max with no value
    // is refused.
    let empty = body(&OwnPast::empty());
    let mut ghost = empty.clone();
    ghost[40] = 1;
    assert_eq!(refused(&ghost), "max");
    assert_eq!(OwnPast::decode(&empty).unwrap(), OwnPast::empty());
    // A count past u32::MAX.
    let mut bytes = good[..58].to_vec();
    varint(&mut bytes, 1);
    varint(&mut bytes, 0);
    varint(&mut bytes, u64::from(u32::MAX) + 1);
    assert_eq!(refused(&bytes), "counts");
}

#[test]
fn a_damaged_depth_table_is_refused_by_name() {
    let raw = [1_000, 1_010, 5];
    let mut split = committed(&raw);
    split.split(1_000, &raw).unwrap();
    let good = body(&split);
    let refused = |bytes: &[u8]| match OwnPast::decode(bytes) {
        Err(HistRefusal::SnapshotInvalid { what }) => what,
        other => panic!("{other:?}"),
    };
    // v2 header (20 bytes), then the depth table: one split bin (bin 80, depth 1).
    assert_eq!(&good[20..23], &[1, 80, 1]);
    let mut deep = good.clone();
    deep[22] = 7; // bin 80 holds 960..=1023: at most 6 more bits
    assert_eq!(refused(&deep), "depth");
    let mut small = good.clone();
    small[21] = 3; // bin 3: never reached, one value wide
    assert_eq!(refused(&small), "depth");
    let mut none = good[..20].to_vec();
    none.push(0);
    none.extend_from_slice(&good[23..]);
    assert_eq!(refused(&none), "depth", "v2 with no split bin");
    let mut v1 = good.clone();
    v1[8] = 1;
    assert!(OwnPast::decode(&v1).is_err(), "v1 reads no depth table");
}

#[test]
fn the_tick_and_bank_bodies_round_trip_and_refuse_by_name() {
    let mut tick = ObservedTick::empty();
    for day in 0..4 {
        for price in [500_000, 500_005, 500_015, 500_010] {
            tick.observe(price + day * 5).unwrap();
        }
        tick.commit_day().unwrap();
    }
    let mut bytes = Vec::new();
    tick.encode(&mut bytes).unwrap();
    assert_eq!(&bytes[..8], TICK_MAGIC);
    assert_eq!(ObservedTick::decode(&bytes).unwrap(), tick);
    let refused = |bytes: &[u8]| match ObservedTick::decode(bytes) {
        Err(HistRefusal::SnapshotInvalid { what }) => what,
        other => panic!("{other:?}"),
    };
    let mut version = bytes.clone();
    version[8] = 2;
    assert_eq!(refused(&version), "version");
    let mut days = bytes.clone();
    days[12] = 0;
    assert_eq!(refused(&days), "tick_days", "more gcd days than days");
    let len = bytes.len();
    let mut zero_days = bytes.clone();
    zero_days[len - 1] = 0;
    assert_eq!(refused(&zero_days), "tick_days");
    let mut order = bytes[..len - 3].to_vec();
    varint(&mut order, 2);
    varint(&mut order, 5);
    varint(&mut order, 1);
    varint(&mut order, 5);
    varint(&mut order, 1);
    assert_eq!(refused(&order), "tick_order");
    let mut extra = bytes.clone();
    extra.push(0);
    assert_eq!(refused(&extra), "trailing");
    tick.observe(500_000).unwrap();
    assert_eq!(
        tick.encode(&mut Vec::new()),
        Err(HistRefusal::StagedNotEmpty)
    );

    let mut bank = HistBank::new(shape());
    bank.observe(&CellKey::stock(Measure::AbsMoveBp), 40)
        .unwrap();
    let key = CellKey::at(Measure::StopBp, Search::Short, 1).kind(BaseKind::Rest);
    bank.observe(&key, 77).unwrap();
    bank.commit_day().unwrap();
    let mut bytes = Vec::new();
    bank.encode(&mut bytes).unwrap();
    assert_eq!(&bytes[..8], BANK_MAGIC);
    let back = HistBank::decode(&bytes).unwrap();
    assert_eq!(back, bank);
    assert_eq!(back.cell(&key).unwrap().max(), Some(77));
    let bank_refused = |bytes: &[u8]| match HistBank::decode(bytes) {
        Err(HistRefusal::SnapshotInvalid { what }) => what,
        other => panic!("{other:?}"),
    };
    let mut version = bytes.clone();
    version[8] = 2;
    assert_eq!(bank_refused(&version), "version");
    let mut extra = bytes.clone();
    extra.push(0);
    assert_eq!(bank_refused(&extra), "trailing");
    // The long widths [1, 2, 4]: write 70,000 for the first.
    let mut wide = bytes[..12].to_vec();
    varint(&mut wide, 3);
    varint(&mut wide, 70_000);
    assert_eq!(bank_refused(&wide), "shape");
    let mut unordered = bytes.clone();
    unordered[13] = 3; // widths [3, 2, 4]
    assert_eq!(
        HistBank::decode(&unordered),
        Err(HistRefusal::ShapeInvalid {
            what: "widths_not_increasing"
        })
    );
    let shape_end = 12 + 1 + 3 + 3 + 1 + 2 + 3;
    let mut cells = bytes.clone();
    cells[shape_end] ^= 1;
    assert_eq!(bank_refused(&cells), "cells");
    bank.observe(&key, 1).unwrap();
    assert_eq!(
        bank.encode(&mut Vec::new()),
        Err(HistRefusal::StagedNotEmpty)
    );
}

// --------------------------------------------------------- ObservedTick --

#[test]
fn the_tick_of_a_five_paisa_and_a_one_paisa_series() {
    let mut five = ObservedTick::empty();
    assert_eq!(five.tick(), Err(HistRefusal::NoTick), "no committed day");
    for price in [123_450, 123_455, 123_470, 123_465] {
        five.observe(price).unwrap();
    }
    assert_eq!(five.tick(), Err(HistRefusal::NoTick), "staged is not past");
    five.commit_day().unwrap();
    assert_eq!(five.tick(), Ok(5));
    let mut one = ObservedTick::empty();
    for price in [77_701, 77_703, 77_708] {
        one.observe(price).unwrap();
    }
    one.commit_day().unwrap();
    assert_eq!(one.tick(), Ok(1));
    // A day of one distinct price has no gcd: it counts as a day, not a tick.
    let mut flat = ObservedTick::empty();
    for _ in 0..3 {
        flat.observe(900_000).unwrap();
    }
    flat.commit_day().unwrap();
    assert_eq!(flat.tick(), Err(HistRefusal::NoTick));
    assert_eq!(flat.committed_days(), 1);
    assert_eq!(
        flat.observe(0),
        Err(HistRefusal::PriceNotPositive { price: 0 })
    );
    assert_eq!(flat.refused_prices(), 1);
}

/// FM-F014: one odd-print day leaves the tick at 5 (a running gcd would
/// answer 1 forever).
#[test]
fn one_odd_print_day_leaves_the_tick_at_five() {
    let mut tick = ObservedTick::empty();
    for day in 0..5 {
        tick.observe(200_000).unwrap();
        tick.observe(200_010).unwrap();
        if day == 2 {
            tick.observe(200_013).unwrap(); // the odd print
        }
        tick.observe(200_005).unwrap();
        tick.commit_day().unwrap();
    }
    assert_eq!(tick.tick(), Ok(5));
    assert_eq!(tick.odd_tick_days(), 1);
    // Ties go to the larger gcd.
    let mut tied = ObservedTick::empty();
    for step in [5, 10, 10, 5] {
        tied.observe(300_000).unwrap();
        tied.observe(300_000 + step).unwrap();
        tied.commit_day().unwrap();
    }
    assert_eq!(tied.tick(), Ok(10));
    assert_eq!(tied.odd_tick_days(), 2);
}

/// FM-F014: 5-paisa days, then a basis break, then 50-paisa days give 50.
#[test]
fn five_paisa_days_a_break_then_fifty_paisa_days_give_fifty() {
    let mut tick = ObservedTick::empty();
    for _ in 0..10 {
        for price in [400_000, 400_005, 400_020] {
            tick.observe(price).unwrap();
        }
        tick.commit_day().unwrap();
    }
    assert_eq!(tick.tick(), Ok(5));
    tick.observe(400_000).unwrap();
    assert_eq!(tick.basis_break(), Err(HistRefusal::StagedNotEmpty));
    tick.discard_day();
    tick.basis_break().unwrap();
    assert_eq!(tick.tick(), Err(HistRefusal::NoTick), "the map restarts");
    assert_eq!((tick.breaks(), tick.committed_days()), (1, 0));
    for _ in 0..2 {
        for price in [40_000, 40_050, 40_150] {
            tick.observe(price).unwrap();
        }
        tick.commit_day().unwrap();
    }
    assert_eq!(tick.tick(), Ok(50));
    assert_eq!(tick.odd_tick_days(), 0);
}

/// FM-F014: an adjusted span labels the decision tick unobservable and uses
/// the GDFL raw tick converted exactly by the factor pair.
#[test]
fn an_adjusted_span_uses_the_converted_raw_tick() {
    let mut decision = ObservedTick::empty();
    let mut raw = ObservedTick::empty();
    for price in [610_000, 610_007] {
        decision.observe(price).unwrap();
    }
    for price in [305_000, 305_005, 305_015] {
        raw.observe(price).unwrap();
    }
    decision.commit_day().unwrap();
    raw.commit_day().unwrap();
    assert_eq!(
        decision_tick(&decision, Basis::Unadjusted, &raw),
        Ok(DecisionTick::Observed { tick: 7 })
    );
    let adjusted = Basis::Adjusted {
        zerodha_close: 300_000,
        gdfl_close: 600_000,
    };
    assert_eq!(
        decision_tick(&decision, adjusted, &raw),
        Ok(DecisionTick::Unobservable { num: 5, den: 2 }),
        "5 paisa on the raw basis is 2.5 on the decision basis"
    );
    let zero = Basis::Adjusted {
        zerodha_close: 0,
        gdfl_close: 600_000,
    };
    assert_eq!(
        decision_tick(&decision, zero, &raw),
        Err(HistRefusal::PriceNotPositive { price: 0 })
    );
    let negative = Basis::Adjusted {
        zerodha_close: 300_000,
        gdfl_close: -1,
    };
    assert_eq!(
        decision_tick(&decision, negative, &raw),
        Err(HistRefusal::PriceNotPositive { price: -1 })
    );
    assert_eq!(
        decision_tick(&decision, adjusted, &ObservedTick::empty()),
        Err(HistRefusal::NoTick)
    );
    assert_eq!(
        decision_tick(&ObservedTick::empty(), Basis::Unadjusted, &raw),
        Err(HistRefusal::NoTick)
    );
}

#[test]
fn the_tick_census_compares_with_a_reference_for_the_report_only() {
    let mut census = TickCensus::zero();
    let series = |step: i64| {
        let mut tick = ObservedTick::empty();
        tick.observe(100_000).unwrap();
        tick.observe(100_000 + step).unwrap();
        tick.commit_day().unwrap();
        tick
    };
    assert_eq!(census.record(&series(5), 5), TickComparison::Equal);
    assert_eq!(census.record(&series(1), 5), TickComparison::Finer);
    assert_eq!(census.record(&series(10), 5), TickComparison::Coarser);
    assert_eq!(census.record(&series(10), 5), TickComparison::Coarser);
    assert_eq!(
        census.record(&ObservedTick::empty(), 5),
        TickComparison::NoTick
    );
    assert_eq!(
        census,
        TickCensus {
            equal: 1,
            finer: 1,
            coarser: 2,
            no_tick: 1
        }
    );
}

// ------------------------------------------------------- MaxSecondGap --

#[test]
fn max_second_gap_only_grows_at_commits() {
    let mut gaps = OwnPast::empty();
    let mut day = GapDay::empty();
    let mut maxima = Vec::new();
    for (start, holes) in [(0_i64, [3, 4]), (100_000, [9, 2]), (200_000, [5, 5])] {
        let mut second = start;
        day.second(second).unwrap();
        for hole in holes {
            second += hole;
            day.second(second).unwrap();
            assert_eq!(gaps.max(), maxima.last().copied(), "not before commit");
        }
        gaps.observe(day.end_day(DayFeed::Clean).unwrap()).unwrap();
        gaps.commit_day().unwrap();
        maxima.push(gaps.max().unwrap());
    }
    assert_eq!(maxima, [4, 9, 9], "never shrinks");
}

/// FM-F018: a day with a 40-minute hole does not raise the threshold, an
/// overnight gap is never observed, and the first day gives Unknown.
#[test]
fn a_hole_day_and_an_overnight_gap_never_raise_the_threshold() {
    let mut gaps = OwnPast::empty();
    let mut day = GapDay::empty();
    assert_eq!(silence_in_trade(&gaps, 600), Silence::Unknown, "first day");
    for second in [1_000, 1_001, 1_003] {
        day.second(second).unwrap();
    }
    gaps.observe(day.end_day(DayFeed::Clean).unwrap()).unwrap();
    assert_eq!(silence_in_trade(&gaps, 600), Silence::Unknown, "staged");
    gaps.commit_day().unwrap();
    assert_eq!(silence_in_trade(&gaps, 3), Silence::Silent);
    assert_eq!(silence_in_trade(&gaps, 2), Silence::Quiet);
    // The next day starts 86,400 seconds later: the overnight gap is not one.
    for second in [87_400, 87_401] {
        day.second(second).unwrap();
    }
    // A 40-minute hole on an execution_feed_hole day.
    day.second(87_401 + 2_400).unwrap();
    assert_eq!(day.end_day(DayFeed::ExecutionFeedHole), None);
    gaps.commit_day().unwrap();
    assert_eq!(gaps.max(), Some(2), "the hole day was never observed");
    // A halt is a phase break: the gap across it is never measured.
    day.second(200_000).unwrap();
    day.phase_break();
    day.second(203_000).unwrap();
    day.second(203_004).unwrap();
    assert_eq!(day.end_day(DayFeed::Clean), Some(4));
    // A day of one second has no gap.
    day.second(300_000).unwrap();
    assert_eq!(day.end_day(DayFeed::Clean), None);
    // Out of order, across a phase break too, is refused and counted.
    day.second(400_000).unwrap();
    day.phase_break();
    assert_eq!(
        day.second(399_999),
        Err(HistRefusal::SecondOutOfOrder {
            second: 399_999,
            last: 400_000
        })
    );
    assert_eq!(
        day.second(400_000).map_err(|e| e.name()),
        Err("second_out_of_order")
    );
    assert_eq!(day.refused(), 2);
    assert_eq!(day.end_day(DayFeed::Clean), None);
    assert_eq!(day.refused(), 2, "the count survives the day");
    // The second, labelled reading is the median.
    let mut empty = OwnPast::empty();
    empty.commit_day().unwrap();
    assert_eq!(
        silence_in_trade(&empty, 1),
        Silence::Unknown,
        "no clean day"
    );
    assert_eq!(gaps.median(), Ok(2));
}

// ------------------------------------------------------------- touches --

/// FM-F017: the step is in basis points of the previous extreme, so a history
/// built at price P and a base at 10P give the same touches; the short
/// tolerance comes from low steps.
#[test]
fn a_history_at_p_and_a_base_at_ten_p_give_the_same_touch_count() {
    let mut long_steps = OwnPast::empty();
    let mut short_steps = OwnPast::empty();
    let highs = [100_000, 100_030, 100_010, 100_050, 100_020, 100_040];
    let lows = [99_000, 98_700, 98_900, 98_100, 98_800, 98_500];
    for pair in highs.windows(2) {
        long_steps
            .observe(step_bp(pair[0], pair[1]).unwrap())
            .unwrap();
    }
    for pair in lows.windows(2) {
        short_steps
            .observe(step_bp(pair[0], pair[1]).unwrap())
            .unwrap();
    }
    long_steps.commit_day().unwrap();
    short_steps.commit_day().unwrap();
    let long_tol = long_steps.median().unwrap();
    let short_tol = short_steps.median().unwrap();
    assert_eq!(long_tol, 2);
    assert_eq!(short_tol, 30, "the short side reads lows, not highs");
    let count =
        |edge: i64, tol: i64, xs: &[i64]| xs.iter().filter(|x| touches(**x, edge, tol)).count();
    let at_p = [100_000, 100_015, 100_020, 100_021, 99_980, 99_979];
    let at_ten_p: Vec<i64> = at_p.iter().map(|x| x * 10).collect();
    assert_eq!(count(100_000, long_tol, &at_p), 4);
    assert_eq!(
        count(1_000_000, long_tol, &at_ten_p),
        4,
        "not rescaled, same"
    );
    assert!(touches(100_020, 100_000, 2), "on the line touches");
    assert!(!touches(100_021, 100_000, 2));
    assert_eq!(
        step_bp(0, 5),
        Err(HistRefusal::PriceNotPositive { price: 0 })
    );
    assert_eq!(step_bp(100_000, 99_999), Ok(0), "rounded toward zero");
    assert_eq!(step_bp(1, i64::MAX), Ok(i64::MAX), "saturates, never wraps");
}

// ------------------------------------------------------------- the bank --

fn shape() -> BankShape {
    BankShape::new(
        Dims {
            widths: vec![1, 2, 4],
            styles: 2,
            variants: 3,
            levels: 2,
        },
        Dims {
            widths: vec![1, 3],
            styles: 1,
            variants: 2,
            levels: 3,
        },
    )
    .unwrap()
}

#[test]
fn hist_bank_indexes_every_cell_exactly_once() {
    let shape = shape();
    let keys = shape.keys();
    assert_eq!(keys.len(), shape.cell_count());
    for (at, key) in keys.iter().enumerate() {
        assert_eq!(shape.index(key), Ok(at), "{key:?}");
    }
    // Long: per width 9 x 3 kind cells (8 measures + 12 shapes = 20 x 3 =
    // 60), touches 3 x 2 = 6, width-only 2, styles 9 x 2 = 18, duration
    // 2 x 3 = 6: 92 per width x 3 widths. Short: 60 + 9 + 2 + 9 + 2 = 82 per
    // width x 2. Two per-stock cells.
    assert_eq!(shape.cell_count(), 92 * 3 + 82 * 2 + 2);
    assert_eq!(
        shape.unsplit_bytes(),
        u64::try_from(shape.cell_count() * UNSPLIT_CELL_BYTES).unwrap()
    );
    let bank = HistBank::new(shape.clone());
    assert_eq!(bank.bytes(), shape.unsplit_bytes());
    assert_eq!(bank.shape(), &shape);
    assert_eq!(OwnPast::empty().bytes(), UNSPLIT_CELL_BYTES);
    let scopes: Vec<Scope> = Measure::ALL.iter().map(|m| m.scope()).collect();
    assert_eq!(scopes.len(), 35);
    let names: Vec<&str> = Measure::ALL.iter().map(|m| m.name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "unique names");
    assert_eq!(Measure::CandleHighStep.name(), "candle_high_step_bp");
    assert_eq!(shape.dims(Search::Short).widths, [1, 3]);
}

#[test]
fn a_cell_key_outside_its_scope_is_refused_by_name() {
    let shape = shape();
    let refused = |key: CellKey| shape.index(&key).unwrap_err();
    let stop = CellKey::at(Measure::StopBp, Search::Long, 0);
    assert_eq!(
        refused(stop),
        HistRefusal::ScopeMismatch {
            measure: "stop_bp",
            field: "kind"
        }
    );
    assert_eq!(
        refused(stop.kind(BaseKind::Micro2).style(0)),
        HistRefusal::ScopeMismatch {
            measure: "stop_bp",
            field: "style"
        }
    );
    assert_eq!(
        refused(CellKey::stock(Measure::StopBp)),
        HistRefusal::ScopeMismatch {
            measure: "stop_bp",
            field: "dir"
        }
    );
    assert_eq!(
        refused(CellKey::at(Measure::AbsMoveBp, Search::Long, 0)),
        HistRefusal::ScopeMismatch {
            measure: "abs_move_bp",
            field: "dir"
        }
    );
    assert_eq!(
        refused(CellKey::at(Measure::StopBp, Search::Short, 2).kind(BaseKind::Rest)),
        HistRefusal::CellOutOfRange {
            measure: "stop_bp",
            field: "width",
            value: 2,
            count: 2
        }
    );
    assert_eq!(
        refused(
            CellKey::at(Measure::Touches, Search::Long, 0)
                .kind(BaseKind::Rest)
                .level(2)
        ),
        HistRefusal::CellOutOfRange {
            measure: "touches",
            field: "level",
            value: 2,
            count: 2
        }
    );
    assert_eq!(
        refused(
            CellKey::at(Measure::TradeDurationSecs, Search::Long, 0)
                .style(1)
                .variant(3)
        ),
        HistRefusal::CellOutOfRange {
            measure: "trade_duration_secs",
            field: "variant",
            value: 3,
            count: 3
        }
    );
    let mut bank = HistBank::new(shape.clone());
    assert!(bank.cell(&stop).is_err());
    assert!(bank.cell_mut(&stop).is_err());
    assert!(bank.observe(&stop, 1).is_err());
    assert_eq!(HistRefusal::NoTick.name(), "no_tick");
    assert_eq!(
        HistRefusal::NoOwnPast.to_string(),
        "own past refused (no_own_past): NoOwnPast"
    );
}

#[test]
fn a_bank_shape_that_is_not_well_formed_is_refused() {
    let dims = |widths: Vec<u16>, styles: u16| Dims {
        widths,
        styles,
        variants: 1,
        levels: 1,
    };
    let refused = |long, short| match BankShape::new(long, short) {
        Err(HistRefusal::ShapeInvalid { what }) => what,
        other => panic!("{other:?}"),
    };
    assert_eq!(refused(dims(vec![], 1), dims(vec![1], 1)), "widths_empty");
    assert_eq!(
        refused(dims(vec![1], 1), dims(vec![2, 2], 1)),
        "widths_not_increasing"
    );
    assert_eq!(
        refused(dims(vec![0, 1], 1), dims(vec![1], 1)),
        "widths_not_increasing"
    );
    assert_eq!(
        refused(dims(vec![1], 0), dims(vec![1], 1)),
        "dimension_zero"
    );
    let mut no_variant = dims(vec![1], 1);
    no_variant.variants = 0;
    assert_eq!(refused(dims(vec![1], 1), no_variant), "dimension_zero");
    let mut no_level = dims(vec![1], 1);
    no_level.levels = 0;
    assert_eq!(refused(no_level, dims(vec![1], 1)), "dimension_zero");
}

/// FM-F011: adding an exit variant leaves every existing histogram
/// byte-identical, and a width added later starts empty.
#[test]
fn adding_a_variant_or_a_width_leaves_every_histogram_byte_identical() {
    let mut bank = HistBank::new(shape());
    let mut mix = Mix(5);
    let keys = bank.shape().keys();
    for _ in 0..3 {
        for key in &keys {
            bank.observe(key, mix.below(10_000)).unwrap();
        }
        bank.commit_day().unwrap();
    }
    let old = bank.clone();
    let grown = BankShape::new(
        Dims {
            widths: vec![1, 2, 3, 4],
            styles: 2,
            variants: 4,
            levels: 2,
        },
        Dims {
            widths: vec![1, 3],
            styles: 1,
            variants: 2,
            levels: 3,
        },
    )
    .unwrap();
    let bank = bank.regrow(grown).unwrap();
    for key in old.shape().keys() {
        let mut moved = key;
        if key.dir == Some(Search::Long) && key.width == Some(2) {
            moved.width = Some(3);
        }
        assert_eq!(bank.cell(&moved).unwrap(), old.cell(&key).unwrap());
    }
    let fresh = CellKey::at(Measure::TradeDurationSecs, Search::Long, 0)
        .style(1)
        .variant(3);
    assert_eq!(bank.cell(&fresh).unwrap().committed_days(), 0);
    let new_width = CellKey::at(Measure::StopBp, Search::Long, 2).kind(BaseKind::Micro3);
    assert_eq!(
        bank.cell(&new_width).unwrap().mid_rank(5),
        Err(HistRefusal::NoOwnPast)
    );
    // A shape that drops a width or shrinks a dimension is refused.
    let shrunk = |long: Dims| BankShape::new(long, bank.shape().dims(Search::Short).clone());
    let dropped = shrunk(Dims {
        widths: vec![1, 3, 4],
        styles: 2,
        variants: 4,
        levels: 2,
    })
    .unwrap();
    assert_eq!(
        bank.clone().regrow(dropped),
        Err(HistRefusal::ShapeNotExtension { what: "width" })
    );
    let fewer = shrunk(Dims {
        widths: vec![1, 2, 3, 4],
        styles: 1,
        variants: 4,
        levels: 2,
    })
    .unwrap();
    assert_eq!(
        bank.clone().regrow(fewer),
        Err(HistRefusal::ShapeNotExtension { what: "dimension" })
    );
    for (styles, variants, levels) in [(2, 3, 2), (2, 4, 1)] {
        let shape = shrunk(Dims {
            widths: vec![1, 2, 3, 4],
            styles,
            variants,
            levels,
        })
        .unwrap();
        assert!(bank.clone().regrow(shape).is_err());
    }
    let past_end = shrunk(Dims {
        widths: vec![1, 2, 3],
        styles: 2,
        variants: 4,
        levels: 2,
    })
    .unwrap();
    assert_eq!(
        bank.regrow(past_end),
        Err(HistRefusal::ShapeNotExtension { what: "width" })
    );
}

/// FM-F011: the exit measures' quantiles equal the sorted history's, read
/// from committed days only, and a share with no committed short decision
/// refuses its short components by name.
#[test]
fn exit_measures_quantiles_equal_sorted_history_from_committed_days_only() {
    let mut bank = HistBank::new(shape());
    let long = CellKey::at(Measure::MaeMilliR, Search::Long, 0).style(1);
    let short = CellKey::at(Measure::MaeMilliR, Search::Short, 0).style(0);
    let mut history = Vec::new();
    for day in 0..4_i64 {
        for trade in 0..3 {
            let mae = (day * 3 + trade) % 7;
            history.push(mae);
            bank.observe(&long, mae).unwrap();
        }
        bank.commit_day().unwrap();
    }
    bank.observe(&long, 6).unwrap(); // today's: never read today
    history.sort_unstable();
    for (num, den) in [(1, 4), (1, 2), (3, 4), (1, 1)] {
        let rank = (num * 12_u64).div_ceil(den);
        let exact = history[usize::try_from(rank - 1).unwrap()];
        assert_eq!(bank.cell(&long).unwrap().quantile(num, den), Ok(exact));
    }
    assert_eq!(
        bank.cell(&short).unwrap().quantile(1, 2),
        Err(HistRefusal::NoObservations),
        "no_own_past:exit:mae for the short side"
    );
    bank.discard_day();
    assert_eq!(bank.cell(&long).unwrap().total(), 12);
}

#[test]
fn width_passes_fit_the_budget_or_refuse_one_width_by_name() {
    let shape = shape();
    let cell = u64::try_from(UNSPLIT_CELL_BYTES).unwrap();
    let all = plan_width_passes(&shape, 10, u64::MAX).unwrap();
    assert_eq!(all.widths, [1, 2, 3, 4]);
    assert_eq!(all.pass_of, [0, 0, 0, 0], "everything fits: one pass");
    assert_eq!(all.fixed_bytes, 2 * cell * 10);
    let total = shape.unsplit_bytes() * 10;
    assert_eq!(all.pass_bytes, [total]);
    // Width 1 is in both directions: 92 + 82 cells; 2 and 4 long only (92);
    // 3 short only (82).
    let one = (92 + 82) * cell * 10;
    let budget = all.fixed_bytes + one;
    let passes = plan_width_passes(&shape, 10, budget).unwrap();
    assert_eq!(passes.pass_of[0], 0, "the largest goes first");
    let count = usize::from(*passes.pass_of.iter().max().unwrap()) + 1;
    assert_eq!(passes.pass_bytes.len(), count);
    assert!(count >= 2);
    for bytes in &passes.pass_bytes {
        assert!(*bytes <= budget, "{bytes} > {budget}");
    }
    let mut seen = [false; 4];
    for (at, pass) in passes.pass_of.iter().enumerate() {
        assert!(usize::from(*pass) < count);
        seen[at] = true;
    }
    assert!(seen.iter().all(|s| *s));
    assert_eq!(
        plan_width_passes(&shape, 10, budget - 1),
        Err(HistRefusal::HistbankWidthOverMemory {
            minutes: 1,
            need: one,
            budget: one - 1
        })
    );
    assert_eq!(
        plan_width_passes(&shape, 10, 5),
        Err(HistRefusal::HistbankWidthOverMemory {
            minutes: 1,
            need: one,
            budget: 0
        })
    );
    let narrow = BankShape::new(
        Dims {
            widths: vec![2],
            styles: 1,
            variants: 1,
            levels: 1,
        },
        Dims {
            widths: vec![1],
            styles: 1,
            variants: 1,
            levels: 1,
        },
    )
    .unwrap();
    let both = plan_width_passes(&narrow, 1, u64::MAX).unwrap();
    assert_eq!(both.widths, [1, 2]);
    let mirrored = BankShape::new(
        narrow.dims(Search::Short).clone(),
        narrow.dims(Search::Long).clone(),
    )
    .unwrap();
    assert_eq!(
        plan_width_passes(&mirrored, 1, u64::MAX).unwrap().widths,
        [1, 2]
    );
}

#[test]
fn every_refusal_has_its_name() {
    let names: Vec<&str> = [
        HistRefusal::NoOwnPast,
        HistRefusal::NoObservations,
        HistRefusal::NegativeValue { value: -1 },
        HistRefusal::CountFull,
        HistRefusal::DaysFull,
        HistRefusal::RankOutOfRange { rank: 0, total: 0 },
        HistRefusal::QuantileInvalid { num: 0, den: 0 },
        HistRefusal::StagedNotEmpty,
        HistRefusal::SplitOneValueWide { value: 0 },
        HistRefusal::SplitRawMismatch,
        HistRefusal::LeavesExceedTable,
        HistRefusal::PriceNotPositive { price: 0 },
        HistRefusal::NoTick,
        HistRefusal::SecondOutOfOrder { second: 0, last: 0 },
        HistRefusal::ScopeMismatch {
            measure: "m",
            field: "f",
        },
        HistRefusal::CellOutOfRange {
            measure: "m",
            field: "f",
            value: 0,
            count: 0,
        },
        HistRefusal::ShapeInvalid { what: "w" },
        HistRefusal::ShapeNotExtension { what: "w" },
        HistRefusal::HistbankWidthOverMemory {
            minutes: 0,
            need: 0,
            budget: 0,
        },
        HistRefusal::SnapshotInvalid { what: "w" },
        HistRefusal::Cube(vocab::fib_cube::CubeRefusal::EmptySample),
    ]
    .iter()
    .map(HistRefusal::name)
    .collect();
    assert_eq!(
        names,
        [
            "no_own_past",
            "no_observations",
            "negative_value",
            "count_full",
            "days_full",
            "rank_out_of_range",
            "quantile_invalid",
            "staged_not_empty",
            "split_one_value_wide",
            "split_raw_mismatch",
            "leaves_exceed_table",
            "price_not_positive",
            "no_tick",
            "second_out_of_order",
            "scope_mismatch",
            "cell_out_of_range",
            "shape_invalid",
            "shape_not_extension",
            "histbank_width_over_memory",
            "snapshot_invalid",
            "cube",
        ]
    );
    let error: &dyn std::error::Error = &HistRefusal::CountFull;
    assert!(error.to_string().contains("count_full"));
}
