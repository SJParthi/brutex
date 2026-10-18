//! Tests of the FIB reference stamps (FIB-8; D-6443, FM-F087). Every price is
//! synthetic.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::super::authorities::{
    AuthorityKind, AuthorityVersion, IndexDay, IndexScreen, InterimCalendar, UnscreenedIndices,
    version_digest,
};
use super::super::day::instant;
use super::super::files::{CandidateRecord, Header, Kind, Record as _, ReferenceCell, Scope};
use super::*;

fn day_of(year: u16, month: u8, of_month: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, of_month)
            .unwrap()
            .days_from_epoch(),
    )
}

fn hm(hour: u32, minute: u32) -> u32 {
    (hour * 60 + minute) * 60
}

fn index_minute(day: i64, second: u32, close: i64) -> Candle {
    Candle {
        ts_micros: instant(day, second),
        open: close - 5,
        high: close + 20,
        low: close - 20,
        close,
        volume: 0,
        open_interest: i64::MIN,
    }
}

/// A full index session, the close rising by one paisa a minute from `base`.
fn session(day: i64, base: i64) -> Vec<Candle> {
    (0..375)
        .map(|n| index_minute(day, hm(9, 15) + n * 60, base + i64::from(n)))
        .collect()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-fib8-ref-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_index(root: &Path, symbol: &str, (year, month): (u16, u8), bars: &[Candle]) {
    let key = InstrumentKey::index(Exchange::Nse, symbol).unwrap();
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        YearMonth::new(year, month).unwrap(),
        FileKind::Bars,
    )
    .unwrap();
    let id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    let rows: Vec<Bar> = bars
        .iter()
        .map(|bar| Bar {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        })
        .collect();
    BarFile::open_or_create(root, path, id)
        .unwrap()
        .append(&rows)
        .unwrap();
}

/// PREC-9's verdict, synthetic: one day junk.
struct Junk(AuthorityVersion, i64);

impl IndexScreen for Junk {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn screen(&self, _: &str, day: i64) -> IndexDay {
        if day == self.1 {
            IndexDay::Junk
        } else {
            IndexDay::Clean
        }
    }
}

fn junk(day: i64) -> Junk {
    Junk(
        AuthorityVersion {
            kind: AuthorityKind::IndexScreen,
            word: "synthetic",
            label: "a synthetic PREC-9 census",
            digest: version_digest(AuthorityKind::IndexScreen, "synthetic", "t", b""),
            interim: false,
        },
        day,
    )
}

#[test]
fn the_index_list_is_the_swept_pair_every_stored_index_vix_and_gift() {
    let store = Scratch::new("list");
    assert_eq!(
        reference_indices(&store.0).unwrap(),
        ["NIFTY", "BANKNIFTY", "INDIAVIX", "GIFTNIFTY"]
    );
    let monday = day_of(2025, 3, 3);
    for symbol in ["NIFTYIT", "BANKNIFTY", "NIFTY", "INDIAVIX", "FINNIFTY"] {
        write_index(&store.0, symbol, (2025, 3), &session(monday, 1_000_000));
    }
    let root = index_root(&store.0).unwrap();
    std::fs::write(root.join("a-stray-file"), b"not an index").unwrap();
    assert_eq!(
        reference_indices(&store.0).unwrap(),
        [
            "NIFTY",
            "BANKNIFTY",
            "FINNIFTY",
            "NIFTYIT",
            "INDIAVIX",
            "GIFTNIFTY"
        ]
    );
}

#[test]
fn the_named_reference_loader_reads_an_index_month() {
    let store = Scratch::new("loader");
    let monday = day_of(2025, 3, 3);
    assert_eq!(load_index_month(&store.0, "NIFTYIT", (2025, 3)), Ok(None));
    let bars = session(monday, 3_000_000);
    write_index(&store.0, "NIFTYIT", (2025, 3), &bars);
    assert_eq!(
        load_index_month(&store.0, "NIFTYIT", (2025, 3)),
        Ok(Some(bars))
    );
    let refused = load_index_month(&store.0, "NOT AN INDEX", (2025, 3)).unwrap_err();
    assert!(refused.starts_with("index_key_refused"), "{refused}");
    let bad_month = load_index_month(&store.0, "NIFTYIT", (1900, 1)).unwrap_err();
    assert!(bad_month.starts_with("1900-01"), "{bad_month}");
    // A month file that is garbage is refused, never read as absent.
    let key = InstrumentKey::index(Exchange::Nse, "NIFTYMID").unwrap();
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 3).unwrap(),
        FileKind::Bars,
    )
    .unwrap()
    .to_path_buf(&store.0);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"synthetic garbage, not a month").unwrap();
    assert!(load_index_month(&store.0, "NIFTYMID", (2025, 3)).is_err());
}

fn sources(monday: i64, tuesday: i64) -> Vec<(String, IndexSource)> {
    let mut nifty = session(monday, 2_000_000);
    nifty.extend(session(tuesday, 2_100_000));
    vec![
        ("NIFTY".to_owned(), IndexSource::Minutes(nifty)),
        ("BANKNIFTY".to_owned(), IndexSource::Missing),
        (VIX_REFERENCE_SYMBOL.to_owned(), IndexSource::Vix),
        (GIFT_NIFTY.to_owned(), IndexSource::Unsourced),
    ]
}

/// FM-F087: a trade's (day, minute) key joins the table by one multiply-add
/// (timed by `crates/cli/benches/fib.rs`); the move is
/// against the previous tradable day's last minute close; a minute with no
/// bar is absent, never a fill; GIFT NIFTY is absent by name.
#[test]
fn a_trade_key_joins_the_reference_table() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let previous: HashMap<String, i64> = [("NIFTY".to_owned(), 1_999_000)].into_iter().collect();
    let table = build_reference(
        (2025, 3),
        &sources(monday, tuesday),
        &previous,
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        None,
    );
    assert_eq!(table.days, 31);
    assert_eq!(table.indices.len(), 4);
    assert_eq!(
        table.screen_label,
        "index day unscreened (PREC-9 not merged)"
    );
    let nifty = table.index_slot("NIFTY").unwrap();
    let open = instant(monday, hm(9, 15));
    assert_eq!(
        table.stamp(open, nifty),
        Some(ReferenceCell::Exact {
            close: 2_000_000,
            previous_close: 1_999_000
        })
    );
    assert_eq!(
        table.stamp(instant(tuesday, hm(10, 0)), nifty),
        Some(ReferenceCell::Exact {
            close: 2_100_045,
            previous_close: 2_000_374
        }),
        "Tuesday measures against Monday's last minute"
    );
    let bank = table.index_slot("BANKNIFTY").unwrap();
    assert_eq!(
        table.stamp(open, bank),
        Some(ReferenceCell::Absent(AbsentReason::NoMinute.byte()))
    );
    let vix = table.index_slot("INDIAVIX").unwrap();
    assert_eq!(
        table.stamp(open, vix),
        Some(ReferenceCell::Absent(AbsentReason::NoMinute.byte())),
        "no VIX month: absent, never a zero"
    );
    let gift = table.index_slot("GIFTNIFTY").unwrap();
    assert_eq!(
        table.stamp(open, gift),
        Some(ReferenceCell::Absent(AbsentReason::SourceUnverified.byte()))
    );
    assert_eq!(table.carried.get("NIFTY"), Some(&2_100_374));
    assert_eq!(table.carried.get("BANKNIFTY"), None);
    assert_eq!(table.holes[nifty], 31 * 375 - 2 * 375);
    assert_eq!(table.holes[gift], 31 * 375);
    // Outside the month, the session or the list: no stamp.
    assert_eq!(
        table.stamp(instant(day_of(2025, 4, 1), hm(9, 15)), nifty),
        None
    );
    assert_eq!(table.stamp(instant(monday, hm(9, 14)), nifty), None);
    assert_eq!(table.stamp(instant(monday, hm(15, 30)), nifty), None);
    assert_eq!(table.stamp(open, 4), None);
    assert_eq!(table.index_slot("SENSEX"), None);
}

/// A junk-flagged day stamps absent (the swept indices included); an
/// unflagged −8% day is stamped as measured.
#[test]
fn a_junk_day_stamps_absent_and_an_unflagged_fall_is_measured() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut nifty = session(monday, 2_000_000);
    nifty.extend(session(tuesday, 1_840_000));
    let sources = vec![("NIFTY".to_owned(), IndexSource::Minutes(nifty))];
    let none = HashMap::new();
    let measured = build_reference(
        (2025, 3),
        &sources,
        &none,
        &junk(i64::MIN),
        &InterimCalendar::interim(),
        None,
    );
    let fall = measured.stamp(instant(tuesday, hm(9, 15)), 0).unwrap();
    assert_eq!(
        fall,
        ReferenceCell::Exact {
            close: 1_840_000,
            previous_close: 2_000_374
        }
    );
    assert_eq!(
        move_bp(fall),
        Some(-802),
        "an 8% fall is a measured stamp, floored"
    );
    assert_eq!(
        measured.stamp(instant(monday, hm(9, 15)), 0),
        Some(ReferenceCell::NoPrevious { close: 2_000_000 })
    );
    let blanked = build_reference(
        (2025, 3),
        &sources,
        &none,
        &junk(tuesday),
        &InterimCalendar::interim(),
        None,
    );
    assert_eq!(
        blanked.stamp(instant(tuesday, hm(9, 15)), 0),
        Some(ReferenceCell::Absent(AbsentReason::Junk.byte()))
    );
    assert_eq!(
        blanked.carried.get("NIFTY"),
        Some(&2_000_374),
        "a junk day is no previous close"
    );
}

/// A day the calendar does not admit (a Budget Saturday) is stamped but is
/// never the next day's previous close.
#[test]
fn a_session_the_gate_excludes_is_never_a_previous_close() {
    let (friday, saturday, monday) = (day_of(2020, 1, 31), day_of(2020, 2, 1), day_of(2020, 2, 3));
    let previous: HashMap<String, i64> = [("NIFTY".to_owned(), 1_111_111)].into_iter().collect();
    let mut nifty = session(saturday, 1_200_000);
    nifty.extend(session(monday, 1_300_000));
    let table = build_reference(
        (2020, 2),
        &[("NIFTY".to_owned(), IndexSource::Minutes(nifty))],
        &previous,
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        None,
    );
    assert_eq!(
        table.stamp(instant(saturday, hm(9, 15)), 0),
        Some(ReferenceCell::Exact {
            close: 1_200_000,
            previous_close: 1_111_111
        })
    );
    assert_eq!(
        table.stamp(instant(monday, hm(9, 15)), 0),
        Some(ReferenceCell::Exact {
            close: 1_300_000,
            previous_close: 1_111_111
        }),
        "January's last tradable close, not the Budget Saturday's"
    );
    assert!(friday < saturday);
}

#[test]
fn a_candidate_s_stride_is_unchanged_when_an_index_is_added() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut wider = sources(monday, tuesday);
    let before = CandidateRecord::stride();
    let none = HashMap::new();
    let narrow = build_reference(
        (2025, 3),
        &wider,
        &none,
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        None,
    );
    wider.insert(2, ("NIFTYIT".to_owned(), IndexSource::Missing));
    let wide = build_reference(
        (2025, 3),
        &wider,
        &none,
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        None,
    );
    assert_eq!(
        CandidateRecord::stride(),
        before,
        "a trade carries its (day, minute) key only"
    );
    let key = instant(monday, hm(11, 0));
    assert_eq!(
        narrow.stamp(key, narrow.index_slot("NIFTY").unwrap()),
        wide.stamp(key, wide.index_slot("NIFTY").unwrap())
    );
    assert_eq!(wide.indices.len(), narrow.indices.len() + 1);
}

#[test]
fn the_reference_file_round_trips_and_joins_in_o1() {
    let store = Scratch::new("file");
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let previous: HashMap<String, i64> = [("NIFTY".to_owned(), 1_999_000)].into_iter().collect();
    let table = build_reference(
        (2025, 3),
        &sources(monday, tuesday),
        &previous,
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        None,
    );
    let path = store.0.join("reference-2025-03.fib");
    write_reference(&path, [1; 32], [2; 32], &table).unwrap();
    let mut file = ReferenceFile::open(&path).unwrap();
    assert_eq!(file.indices, table.indices);
    for (day, second) in [
        (monday, hm(9, 15)),
        (tuesday, hm(15, 29)),
        (day_of(2025, 3, 31), hm(12, 0)),
    ] {
        for slot in 0..table.indices.len() {
            let key = instant(day, second);
            assert_eq!(
                file.stamp(key, slot).unwrap(),
                table.stamp(key, slot).unwrap()
            );
        }
    }
    let outside = file.stamp(instant(monday, hm(9, 0)), 0).unwrap_err();
    assert_eq!(outside.word, "outside_table");
    // Not a reference file, or a profile that does not match its records.
    let closes = store.0.join("closes.fib");
    files::write_records(
        &closes,
        [1; 32],
        [2; 32],
        Scope::StockMonth {
            series: super::super::authorities::SeriesId(1),
            year: 2025,
            month: 3,
        },
        Vec::new(),
        &[files::CloseRecord::NoPrevious { day: 1 }],
    )
    .unwrap();
    assert_eq!(ReferenceFile::open(&closes).unwrap_err().word, "wrong_kind");
    let rank = store.0.join("rank.fib");
    files::write_file(
        &rank,
        &Header {
            kind: Kind::Rank,
            record_version: 1,
            stride: 8,
            schema_digest: [0; 32],
            run_identity: [0; 32],
            scope: Scope::Month {
                year: 2025,
                month: 3,
            },
            profile: Vec::new(),
        },
        std::iter::empty(),
    )
    .unwrap();
    assert_eq!(ReferenceFile::open(&rank).unwrap_err().word, "wrong_kind");
    let mismatched = store.0.join("mismatched.fib");
    let mut lie = profile(&table.indices);
    lie.push(0);
    files::write_file(
        &mismatched,
        &Header {
            kind: Kind::Reference,
            record_version: 1,
            stride: ReferenceCell::stride(),
            schema_digest: [0; 32],
            run_identity: [0; 32],
            scope: Scope::Month {
                year: 2025,
                month: 3,
            },
            profile: lie,
        },
        std::iter::empty(),
    )
    .unwrap();
    assert_eq!(
        ReferenceFile::open(&mismatched).unwrap_err().word,
        "bad_profile"
    );
    let short = store.0.join("short.fib");
    files::write_file(
        &short,
        &Header {
            kind: Kind::Reference,
            record_version: 1,
            stride: ReferenceCell::stride(),
            schema_digest: [0; 32],
            run_identity: [0; 32],
            scope: Scope::Month {
                year: 2025,
                month: 3,
            },
            profile: vec![9, 0, 0, 0],
        },
        std::iter::empty(),
    )
    .unwrap();
    assert_eq!(ReferenceFile::open(&short).unwrap_err().word, "bad_profile");
}

/// INDIA VIX through `VixReferenceMonth`: exact or absent, never a fill.
#[test]
fn india_vix_is_stamped_exact_or_absent() {
    let store = Scratch::new("vix");
    let monday = day_of(2025, 3, 3);
    let mut vix = session(monday, 1_500);
    vix.retain(|bar| bar.ts_micros != instant(monday, hm(10, 0)));
    write_index(&store.0, "INDIAVIX", (2025, 3), &vix);
    let month =
        VixReferenceMonth::open(&store.0, Vendor::Zerodha, YearMonth::new(2025, 3).unwrap())
            .unwrap();
    let table = build_reference(
        (2025, 3),
        &[(VIX_REFERENCE_SYMBOL.to_owned(), IndexSource::Vix)],
        &HashMap::new(),
        &UnscreenedIndices::interim(),
        &InterimCalendar::interim(),
        Some(&month),
    );
    assert_eq!(
        table.stamp(instant(monday, hm(9, 15)), 0),
        Some(ReferenceCell::NoPrevious { close: 1_500 })
    );
    assert_eq!(
        table.stamp(instant(monday, hm(10, 0)), 0),
        Some(ReferenceCell::Absent(AbsentReason::NoMinute.byte()))
    );
    assert_eq!(
        table.carried.get(VIX_REFERENCE_SYMBOL),
        Some(&(1_500 + 374))
    );
}

#[test]
fn the_reasons_slots_and_moves_are_exact() {
    for (at, reason) in AbsentReason::ALL.iter().enumerate() {
        assert_eq!(usize::from(reason.byte()), at);
        assert_eq!(AbsentReason::of_byte(reason.byte()), Some(*reason));
    }
    assert_eq!(AbsentReason::of_byte(9), None);
    let names: Vec<&str> = AbsentReason::ALL
        .iter()
        .map(|reason| reason.name())
        .collect();
    assert_eq!(
        names,
        [
            "no_minute",
            "index_day_foreign",
            "index_day_rescaled",
            "index_day_junk",
            "source UNVERIFIED"
        ]
    );
    assert_eq!(AbsentReason::of_screen(IndexDay::Unscreened), None);
    assert_eq!(AbsentReason::of_screen(IndexDay::Clean), None);
    assert_eq!(
        AbsentReason::of_screen(IndexDay::Foreign),
        Some(AbsentReason::Foreign)
    );
    assert_eq!(
        AbsentReason::of_screen(IndexDay::Rescaled),
        Some(AbsentReason::Rescaled)
    );
    assert_eq!(
        AbsentReason::of_screen(IndexDay::Junk),
        Some(AbsentReason::Junk)
    );
    let monday = day_of(2025, 3, 3);
    assert_eq!(minute_slot(instant(monday, hm(9, 15))), Some(0));
    assert_eq!(minute_slot(instant(monday, hm(15, 29))), Some(374));
    assert_eq!(minute_slot(instant(monday, hm(15, 30))), None);
    assert_eq!(minute_slot(instant(monday, hm(9, 14))), None);
    assert_eq!(minutes_per_day(), 375);
    assert_eq!(
        move_bp(ReferenceCell::Exact {
            close: 101,
            previous_close: 100
        }),
        Some(100)
    );
    assert_eq!(
        move_bp(ReferenceCell::Exact {
            close: 99,
            previous_close: 100
        }),
        Some(-100)
    );
    assert_eq!(
        move_bp(ReferenceCell::Exact {
            close: 5,
            previous_close: 0
        }),
        None
    );
    assert_eq!(move_bp(ReferenceCell::NoPrevious { close: 5 }), None);
    assert_eq!(move_bp(ReferenceCell::Absent(0)), None);
}

/// NO TYPED NUMBERS: the shipping part of this file holds no numeric
/// literal but 0 and 1.
#[test]
fn reference_rs_types_no_number() {
    let source = include_str!("reference.rs");
    let typed = vocab::fib_cube::untyped_numbers(source);
    assert!(typed.is_empty(), "{typed:?}");
}

#[test]
fn the_named_indices_are_the_lists() {
    for name in SWEPT_INDICES.iter().chain([&VIX_REFERENCE_SYMBOL]) {
        assert!(is_named_index(name), "{name}");
    }
    assert!(!is_named_index("NIFTYIT"));
    assert!(!is_named_index(GIFT_NIFTY));
}
