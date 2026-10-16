//! Mutant-killing tests for the public items of the GDFL import's readers,
//! the fold, the scope ledger and the scrub (cloud mutation round,
//! 2026-10-16; `reports/cloud-mut-pull-20261016.md`).
//!
//! Each test names the behaviour one or more surviving `cargo mutants`
//! mutants broke and asserts exactly that behaviour, so the mutant fails it.
//! Private items are tested beside their code, in the `cloud_mutant_tests`
//! module at the end of each source file.
//!
//! # Every value here is invented
//!
//! The repository is public and the vendor's licence is UNVERIFIED: no row,
//! price or file of any vendor appears here. Tickers are real index and
//! share names only where the code under test admits nothing else (the swept
//! surface); every price, quantity and stamp is made up.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use brutex_core::instrument::{Contract, Exchange, Expiry, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey};
use store::path::{Timeframe, YearMonth};

/// A census entry of `contract` with one bar.
fn entry_of(contract: Option<Contract>) -> Entry {
    Entry {
        key: EntryKey {
            contract,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").unwrap(),
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2031, 3).unwrap(),
        },
        rows: 1,
        first_ts_micros: 1,
        last_ts_micros: 1,
    }
}

// ═══ scrub ════════════════════════════════════════════════════════════════

/// `scrub::contract_of` hands back the entry's own contract, and `None` for
/// spot (kills `-> None` and, where it compiles, `-> Some(Default)`).
#[test]
fn scrub_contract_of_is_the_entrys_own_contract() {
    let contract = Contract::of(Kind::Option {
        expiry: Expiry::new(2031, 3, 27).unwrap(),
        strike: Paisa::from_raw(1_234_500),
        side: OptionSide::Put,
    })
    .unwrap();
    assert_eq!(
        pull::scrub::contract_of(&entry_of(Some(contract))),
        Some(contract)
    );
    assert_eq!(pull::scrub::contract_of(&entry_of(None)), None);
}

// ═══ shared fixtures ══════════════════════════════════════════════════════

/// A scratch directory unique to `tag` and this process, empty.
fn scratch(tag: &str) -> std::path::PathBuf {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "brutex-cloud-mut-{tag}-{}-{serial}",
        std::process::id()
    ));
    let _absent = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The one telemetry sink this binary installs (`telemetry::install` is a
/// process singleton), at `Trace`, and its log directory.
fn sink() -> &'static (std::path::PathBuf, &'static telemetry::Sink) {
    static SINK: std::sync::OnceLock<(std::path::PathBuf, &'static telemetry::Sink)> =
        std::sync::OnceLock::new();
    SINK.get_or_init(|| {
        let logs = scratch("logs");
        let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
        let sink = telemetry::install(&config).expect("the only install in this binary");
        (logs, sink)
    })
}

/// Every log line written so far that holds `marker`, synced first.
fn logged(marker: &str) -> Vec<String> {
    let (logs, sink) = sink();
    sink.sync().unwrap();
    std::fs::read_to_string(telemetry::current_path(logs))
        .unwrap()
        .lines()
        .filter(|line| line.contains(marker))
        .map(str::to_owned)
        .collect()
}

fn day(y: u16, m: u8, d: u8) -> pull::session::Day {
    pull::session::Day::new(y, m, d).unwrap()
}

// ═══ gdfl_cm::auction ═════════════════════════════════════════════════════

mod auction_sidecar {
    use super::{day, logged, scratch, sink};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::gdfl_cm::auction::{self, Print, RECORD_LEN, Record};

    fn print(sod: u32, price: i64, quantity: u64) -> Print {
        Print {
            sod,
            price,
            quantity,
            rows: 1,
            line: 2,
        }
    }

    /// A record is exactly 32 bytes with a zero reserved word: a slice one
    /// byte longer or shorter is no record, whatever its reserved word says.
    /// Kills the length-and-reserved test's `&&` read as `||`.
    #[test]
    fn a_record_is_exactly_thirty_two_bytes() {
        let record = Record {
            t_micros: -5,
            price_paisa: 12_345,
            quantity: 6,
            rows: 7,
        };
        let bytes = record.to_bytes();
        assert_eq!(Record::from_bytes(&bytes), Some(record));
        let mut long = bytes.to_vec();
        long.push(0);
        assert_eq!(Record::from_bytes(&long), None, "33 bytes");
        assert_eq!(
            Record::from_bytes(&bytes[..RECORD_LEN - 1]),
            None,
            "31 bytes"
        );
        assert_eq!(Record::from_bytes(&[]), None, "zero bytes");
        let mut reserved = bytes;
        reserved[31] = 1;
        assert_eq!(Record::from_bytes(&reserved), None, "a reserved bit set");
    }

    /// A print stamped at exactly IST midnight belongs to its own day, never
    /// to the day before: re-offering the earlier day after it is still the
    /// same day's records and writes nothing. Kills the day window's `<`
    /// read as `<=`.
    #[test]
    fn a_print_at_ist_midnight_is_its_own_days_and_not_the_day_befores() {
        let root = scratch("AucMidnight");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZMID").unwrap();
        let (d1, d2) = (day(2031, 3, 4), day(2031, 3, 5));
        let late = [print(15 * 3_600 + 28 * 60 + 7, 50_000, 3)];
        let midnight = [print(0, 50_100, 4)];
        assert_eq!(auction::append(&root, &key, d1, &late).unwrap().written, 1);
        assert_eq!(
            auction::append(&root, &key, d2, &midnight).unwrap().written,
            1
        );
        let again = auction::append(&root, &key, d1, &late).unwrap();
        assert_eq!((again.offered, again.written), (1, 0));
        let records =
            auction::parse(&std::fs::read(auction::path(&root, &key, d1).unwrap()).unwrap())
                .unwrap();
        assert_eq!(
            records,
            vec![Record::of(d1, &late[0]), Record::of(d2, &midnight[0])]
        );
        // 00:00:00 IST of d2 is 18:30:00 UTC of d1.
        assert_eq!(
            records[1].t_micros,
            (i64::from(d1.days_from_epoch()) * 86_400 + 18 * 3_600 + 30 * 60) * 1_000_000
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Every filed day is one `info` line naming the share (an `&` kept as
    /// written), the day, the prints offered and written, and the sidecar;
    /// the idempotent re-offer is a line too, with `written` 0. Kills
    /// `note_filed` read as `()`.
    #[test]
    fn every_filed_day_says_what_it_filed() {
        sink();
        let root = scratch("AucLog");
        let key = InstrumentKey::cash(Exchange::Nse, "Z&Z").unwrap();
        let d = day(2031, 3, 6);
        let prints = [
            print(15 * 3_600 + 28 * 60 + 7, 50_000, 3),
            print(15 * 3_600 + 30 * 60, 50_005, 1),
        ];
        assert_eq!(auction::append(&root, &key, d, &prints).unwrap().written, 2);
        assert_eq!(auction::append(&root, &key, d, &prints).unwrap().written, 0);
        let sidecar = auction::path(&root, &key, d).unwrap();
        let lines = logged(&sidecar.display().to_string());
        assert_eq!(lines.len(), 2, "{lines:#?}");
        for (line, written) in lines.iter().zip(["\"written\":2", "\"written\":0"]) {
            for want in [
                "\"auction prints filed\"",
                "\"gdfl.import\"",
                "\"info\"",
                "\"symbol\":\"Z&Z\"",
                "\"day\":\"2031-03-06\"",
                "\"prints\":2",
                written,
            ] {
                assert!(line.contains(want), "{want} in {line}");
            }
        }
        // Nothing offered: nothing filed, nothing said, no file made.
        let empty = auction::append(&root, &key, day(2031, 3, 7), &[]).unwrap();
        assert_eq!((empty.offered, empty.written), (0, 0));
        assert_eq!(logged("2031-03-07").len(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ scrub: a record that fails its checksum, a month born without them ═══

mod scrub_reads {
    use super::{day, scratch};
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use brutex_core::vendor::Vendor;
    use pull::manifest::{Entry, EntryKey};
    use pull::scrub::{self, Finding};
    use store::file::BarFile;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    fn path() -> StorePath<'static> {
        StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2031, 3).unwrap(),
            file: FileKind::Bars,
        })
        .unwrap()
    }

    /// A month whose one record no longer matches its block checksum: the
    /// counts agree, so only reading the record can tell, and the scrub says
    /// `Unreadable` with the path and the store's own words — never
    /// `Agrees`, never a `Bounds` built from a value it could not read.
    #[test]
    fn a_record_that_fails_its_checksum_is_unreadable_not_agreement() {
        let root = scratch("ScrubRot");
        // 09:15 IST on 2031-03-03.
        let ts = i64::from(day(2031, 3, 3).days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
        let mut writer = BarFile::open_or_create(&root, path(), 7).unwrap();
        writer
            .append(&[Bar {
                ts_micros: ts,
                open: 101,
                high: 109,
                low: 99,
                close: 105,
                volume: 3,
                open_interest: OI_NULL,
            }])
            .unwrap();
        drop(writer);
        let entry = Entry {
            key: EntryKey {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Index,
                symbol: Symbol::new("NIFTY").unwrap(),
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2031, 3).unwrap(),
            },
            rows: 1,
            first_ts_micros: ts,
            last_ts_micros: ts,
        };
        assert_eq!(scrub::one(&entry, &root, Vendor::Dhan, 7), Finding::Agrees);
        let file = BarFile::open_existing(&root, path(), 7).unwrap();
        let at = usize::try_from(file.layout().offset_of(0).unwrap()).unwrap();
        let bin = file.path().to_path_buf();
        drop(file);
        let mut bytes = std::fs::read(&bin).unwrap();
        bytes[at + 8] ^= 0x5a;
        std::fs::write(&bin, bytes).unwrap();
        let shown = path().to_path_buf(&root).display().to_string();
        match scrub::one(&entry, &root, Vendor::Dhan, 7) {
            Finding::Unreadable { path, why } => {
                assert_eq!(path, shown);
                assert!(!why.is_empty());
            }
            other => panic!("a rotted record read as {other:?}"),
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A month born without block checksums (a version-2 header, written
    /// here by hand as every build before the writer's fix wrote one) has
    /// nothing to verify against: the block pass says so, by the file's
    /// name, and checks nothing.
    #[test]
    fn a_month_born_without_checksums_is_unverifiable_by_the_block_pass() {
        let root = scratch("ScrubUnsealed");
        let bin = path().to_path_buf(&root);
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        let layout = store::layout::Layout::V2;
        let genesis = store::header::Header::genesis_at(layout, 7, 60, 0);
        let commit = genesis.commit().unwrap();
        let mut bytes = vec![0_u8; usize::try_from(layout.header_len()).unwrap()];
        let at = usize::try_from(commit.offset).unwrap();
        bytes[at..at + commit.bytes.len()].copy_from_slice(&commit.bytes);
        std::fs::write(&bin, bytes).unwrap();
        let file = BarFile::open_existing(&root, path(), 7).unwrap();
        assert!(!file.header().checksums_present(), "the premise");
        let pass = scrub::blocks(&file, 0, u64::MAX);
        assert_eq!((pass.blocks, pass.checked, pass.next), (0, 0, None));
        assert!(pass.bad.is_empty());
        let why = pass.unverifiable.unwrap();
        assert!(
            why.contains("without block checksums") && why.contains(&bin.display().to_string()),
            "{why}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ gdfl_cm::auction: the sidecar's directory and lock refusals ═══════════

mod auction_refusals {
    use super::{day, scratch};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::gdfl_cm::CmRefusal;
    use pull::gdfl_cm::auction::{self, Print};

    const PRINT: Print = Print {
        sod: 15 * 3_600 + 28 * 60 + 7,
        price: 50_000,
        quantity: 3,
        rows: 1,
        line: 2,
    };

    /// A sidecar whose directory cannot be made (a file stands where the
    /// share's directory goes) is refused naming that directory.
    #[test]
    fn a_sidecar_whose_directory_cannot_be_made_is_refused_by_name() {
        let root = scratch("AucNoDir");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZDIR").unwrap();
        let d = day(2031, 3, 4);
        let sidecar = auction::path(&root, &key, d).unwrap();
        let blocker = sidecar.parent().unwrap().parent().unwrap().to_path_buf();
        std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
        std::fs::write(&blocker, b"in the way").unwrap();
        match auction::append(&root, &key, d, &[PRINT]) {
            Err(CmRefusal::AuctionSidecar { why }) => {
                assert!(why.contains(&blocker.display().to_string()), "{why}");
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A sidecar another writer holds is refused as busy, by name, and is
    /// left byte for byte as it was; once the writer lets go the append
    /// goes through.
    #[test]
    fn a_sidecar_another_writer_holds_is_refused_as_busy() {
        let root = scratch("AucBusy");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZBUSY").unwrap();
        let (d1, d2) = (day(2031, 3, 4), day(2031, 3, 5));
        assert_eq!(
            auction::append(&root, &key, d1, &[PRINT]).unwrap().written,
            1
        );
        let sidecar = auction::path(&root, &key, d1).unwrap();
        let before = std::fs::read(&sidecar).unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&sidecar)
            .unwrap();
        let held = store::flock::Flock::try_lock(file, sidecar.clone()).unwrap();
        match auction::append(&root, &key, d2, &[PRINT]) {
            Err(CmRefusal::AuctionSidecar { why }) => {
                assert!(why.contains("is busy or cannot be locked"), "{why}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(std::fs::read(&sidecar).unwrap(), before);
        held.release().unwrap();
        assert_eq!(
            auction::append(&root, &key, d2, &[PRINT]).unwrap().written,
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ cash_session_cache: the two public loaders over a warm cache ══════════

mod cash_loaders {
    use super::{day, scratch};
    use std::collections::HashMap;
    use std::io::Write as _;

    const CSV: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n1363,HINDALCO,EQ,INE038A01020,1\n16921,20MICRONS,EQ,INE144J01027,0\n";

    fn gzip() -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(CSV.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// `prepare` over a window whose every required master is held answers
    /// each, keyed by IST day number, without asking any host; the weekend
    /// inside it owes nothing. Kills `prepare` answering an empty or a
    /// one-entry map.
    #[test]
    fn prepare_over_a_held_window_answers_every_required_day() {
        let root = scratch("CashPrepare");
        let (mon, tue) = (day(2026, 8, 3), day(2026, 8, 4));
        for held in [mon, tue] {
            pull::cash_session_cache::install(&root, held, &gzip()).unwrap();
        }
        let window = pull::session::Window::new(day(2026, 8, 1), tue).unwrap();
        let map = runtime()
            .block_on(pull::cash_session_cache::prepare(&root, window))
            .unwrap();
        assert_eq!(map.len(), 2);
        let flags = map.get(&mon.days_from_epoch()).unwrap();
        assert!(flags.eligibility("HINDALCO", "INE038A01020").unwrap());
        assert!(!flags.eligibility("20MICRONS", "INE144J01027").unwrap());
        assert!(map.contains_key(&tue.days_from_epoch()));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// `prepare_observed` fills the caller's cache with every observed day's
    /// held master, deduplicated, without asking any host. Kills
    /// `prepare_observed` read as `Ok(())`.
    #[test]
    fn prepare_observed_fills_the_cache_from_held_masters() {
        let root = scratch("CashObserved");
        let (mon, tue) = (day(2026, 8, 3), day(2026, 8, 4));
        for held in [mon, tue] {
            pull::cash_session_cache::install(&root, held, &gzip()).unwrap();
        }
        let mut cache = HashMap::new();
        runtime()
            .block_on(pull::cash_session_cache::prepare_observed(
                &root,
                &[tue, mon, tue],
                &mut cache,
            ))
            .unwrap();
        assert_eq!(cache.len(), 2);
        assert!(
            cache
                .get(&tue.days_from_epoch())
                .unwrap()
                .eligibility("HINDALCO", "INE038A01020")
                .unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ fold ═════════════════════════════════════════════════════════════════

/// The anchor refusal names the stamp it could not place and why it was
/// refused rather than saturated.
#[test]
fn fold_an_anchor_overflow_names_its_stamp() {
    let said = pull::fold::FoldError::AnchorOverflow { ts_micros: -42 }.to_string();
    assert!(
        said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
            && said.contains("Refused rather than saturated"),
        "{said}"
    );
}

/// The completeness fold's paths through its public forms: an index day's
/// absent head and tail, a special session's first bucket, duplicate and
/// off-grid minutes, withheld days counted under their own sentence, and the
/// snapshot fold's reservation.
mod fold_paths {
    use super::day;
    use pull::calendar::Runtime;
    use pull::fold::{Bucket, Derived, Tally};
    use pull::session::Day;
    use pull::vendor::Venue;
    use store::format::Bar;

    const MINUTE: i64 = 60_000_000;

    /// IST midnight of `d`, as UTC microseconds.
    fn midnight(d: Day) -> i64 {
        (i64::from(d.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS) * 1_000_000
    }

    /// One invented bar stamped `ts_micros`.
    fn bar(ts_micros: i64) -> Bar {
        Bar {
            ts_micros,
            open: 1_000,
            high: 1_010,
            low: 990,
            close: 1_005,
            volume: 3,
            open_interest: i64::MIN,
        }
    }

    /// One-minute bars at `minutes` past IST midnight on `d`.
    fn at(d: Day, minutes: impl IntoIterator<Item = i64>) -> Vec<Bar> {
        minutes
            .into_iter()
            .map(|m| bar(midnight(d) + m * MINUTE))
            .collect()
    }

    /// `f` on a thread, failing after ten seconds instead of hanging: a
    /// mutant that zeroes a bucket width loops forever counting buckets.
    fn within<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (send, answer) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(f()));
        answer
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("answered within ten seconds")
    }

    /// The detailed form's sentences, one per withheld bucket, range or day.
    fn detailed(bars: &[Bar], secs: u32, venue: Venue) -> Vec<String> {
        let bars = bars.to_vec();
        within(move || {
            pull::fold::complete_minutes_with_calendar(
                &bars,
                Bucket::of_secs(secs).unwrap(),
                venue,
                None,
                Runtime::default(),
            )
            .unwrap()
            .1
        })
    }

    /// The derive's counted form.
    fn derived(bars: &[Bar], secs: u32, venue: Venue) -> Derived {
        let bars = bars.to_vec();
        within(move || {
            pull::fold::derive_minutes(
                &bars,
                Bucket::of_secs(secs).unwrap(),
                venue,
                None,
                Runtime::default(),
            )
            .unwrap()
        })
    }

    /// An index owes every minute of its session, so a day that starts late
    /// names its absent head from the session's first bucket, and a day that
    /// stops early names its absent tail once the next day begins; each is
    /// counted withheld by the buckets it owes. A share's minute with no
    /// candle is no trade, so the same holes are no bar and no sentence.
    #[test]
    fn an_index_days_absent_head_and_tail_are_named_and_a_shares_are_not() {
        let first = day(2025, 7, 1);
        // 09:25..=14:59: the first day's first two five-minute buckets and its
        // last six hold no minute. The second day is whole.
        let mut bars = at(first, 565..900);
        bars.extend(at(day(2025, 7, 2), 555..930));
        let m = midnight(first);
        assert_eq!(
            detailed(&bars, 300, Venue::NseIndex),
            [
                format!(
                    "bucket range [{}, {}): absent: observed 0, scheduled 10; withheld; historical gap refill requires a versioned store repair",
                    m + 555 * MINUTE,
                    m + 565 * MINUTE
                ),
                format!(
                    "bucket range [{}, {}): absent: observed 0, scheduled 30; observed-day tail withheld; historical gap refill requires a versioned store repair",
                    m + 900 * MINUTE,
                    m + 930 * MINUTE
                ),
            ]
        );
        assert_eq!(
            derived(&bars, 300, Venue::NseIndex).tally,
            Tally {
                days: 2,
                buckets: 67 + 75,
                withheld: 2 + 6,
                partial: 0
            }
        );
        let share = derived(&bars, 300, Venue::NseCash);
        assert!(share.diagnostics.is_empty(), "{:?}", share.diagnostics);
        assert_eq!(
            share.tally,
            Tally {
                days: 2,
                buckets: 142,
                withheld: 0,
                partial: 0
            }
        );
    }

    /// A special session's absent first bucket is named from that bucket's
    /// start on the regular 09:15 grid: the 2024-11-01 Muhurat opened at
    /// 18:00, so its first hour bucket starts at 17:15 and owes the fifteen
    /// minutes 18:00..=18:14, not a range starting at the window's open.
    #[test]
    fn a_muhurats_absent_first_bucket_is_named_from_its_grid_start() {
        let evening = day(2024, 11, 1);
        let m = midnight(evening);
        assert_eq!(
            detailed(&at(evening, 1_095..1_140), 3_600, Venue::NseIndex),
            [format!(
                "bucket range [{}, {}): absent: observed 0, scheduled 15; withheld; historical gap refill requires a versioned store repair",
                m + 1_035 * MINUTE,
                m + 1_095 * MINUTE
            )]
        );
    }

    /// A bucket whose row count equals the minutes it owes is still withheld
    /// when one row repeats a stamp or sits off the minute grid: the count
    /// matches, the coverage does not. For an index and a share alike, since
    /// an invalid row is no more a trade than a missing one.
    #[test]
    fn a_duplicate_or_off_grid_minute_never_completes_a_bucket() {
        let d = day(2025, 7, 1);
        let m = midnight(d);
        let mut bars = at(d, 555..930);
        // 09:24 missing and 09:23 twice: five rows where five are owed.
        bars[9] = bars[8];
        // 09:30 stamped half a minute late: five rows, one off the grid.
        bars[15].ts_micros += 30_000_000;
        let notes = detailed(&bars, 300, Venue::NseIndex);
        assert_eq!(notes.len(), 2, "{notes:?}");
        for (note, start) in notes.iter().zip([560, 570]) {
            assert!(
                note.starts_with(&format!(
                    "bucket {}: incomplete or invalid minute coverage: observed 5, scheduled 5,",
                    m + start * MINUTE
                )),
                "{note}"
            );
        }
        for venue in [Venue::NseIndex, Venue::NseCash] {
            assert_eq!(
                derived(&bars, 300, venue).tally,
                Tally {
                    days: 1,
                    buckets: 73,
                    withheld: 2,
                    partial: 0
                },
                "{venue:?}"
            );
        }
    }

    /// Every bucket of a withheld day is counted under that day's own
    /// sentence, and only there: minute bars on a closed Sunday are one store
    /// defect, and two cash days from 2026-08-03 with no dated eligibility are
    /// one counted line of 150 buckets — never a coverage fault per bucket.
    #[test]
    fn a_withheld_days_buckets_are_counted_under_its_own_sentence() {
        let sunday = day(2025, 7, 6);
        let tuesday = day(2026, 8, 4);
        let mut bars = at(sunday, 555..560);
        bars.extend(at(tuesday, 555..930));
        bars.extend(at(day(2026, 8, 5), 555..930));
        let share = derived(&bars, 300, Venue::NseCash);
        assert!(share.bars.is_empty());
        assert_eq!(
            share.tally,
            Tally {
                days: 0,
                buckets: 0,
                withheld: 1 + 75 + 75,
                partial: 0
            }
        );
        assert_eq!(
            share.diagnostics,
            [
                format!(
                    "day {}: minute bars on a day the calendar measures as closed; store defect; derived buckets withheld",
                    sunday.days_from_epoch()
                ),
                format!(
                    "days whose session hours cannot attest the stored grid: 150 bucket(s) withheld on 2 day(s), 2026-08-04..2026-08-05, over 2 note(s); the first: day {}: cash session UNVERIFIED on 2026-08-04: dated eligibility required; derived buckets withheld",
                    tuesday.days_from_epoch()
                ),
            ]
        );
    }

    /// The snapshot fold reserves exactly what it can return: the buckets
    /// its span covers, capped at one bar per snapshot, so a dense run ends
    /// with no slack and a sparse one never reserves its empty span.
    #[test]
    fn a_fold_reserves_exactly_the_bars_it_can_return() {
        let open = midnight(day(2025, 7, 1)) + 555 * MINUTE;
        let dense: Vec<Bar> = (0..3).map(|i| bar(open + i * MINUTE)).collect();
        let out = pull::fold::fold(&dense, Bucket::MINUTE).unwrap();
        assert_eq!((out.len(), out.capacity()), (3, 3));
        let sparse = [bar(open), bar(open + 100 * MINUTE)];
        let out = pull::fold::fold(&sparse, Bucket::MINUTE).unwrap();
        assert_eq!((out.len(), out.capacity()), (2, 2));
    }

    /// A one-second bar is stamped at its second's start, never at its first
    /// traded row's sub-second stamp, and a quote-only row (no trade) still
    /// gives its second the open interest it carried.
    #[test]
    fn a_seconds_bar_is_stamped_at_its_seconds_start() {
        use pull::fold::Placed;
        let open = midnight(day(2025, 7, 1)) + 555 * MINUTE;
        let rows = [
            Placed {
                bar: bar(open + 250_000),
                traded: true,
            },
            Placed {
                bar: Bar {
                    open_interest: 40,
                    ..bar(open + 1_750_000)
                },
                traded: false,
            },
            Placed {
                bar: bar(open + 1_900_000),
                traded: true,
            },
        ];
        let out = pull::fold::fold_seconds_with_last_oi(&rows).unwrap();
        assert_eq!(
            out.iter()
                .map(|b| (b.ts_micros, b.open_interest))
                .collect::<Vec<_>>(),
            [(open, i64::MIN), (open + 1_000_000, 40)]
        );
    }
}

// ═══ gdfl_tickstore: a socket where a day file belongs ═════════════════════

/// A socket at a day file's name is refused as not a regular file, by
/// name, never opened and never taken for an absent day.
#[test]
fn tickstore_a_socket_at_a_day_name_is_refused_by_name() {
    use pull::gdfl_cm::{CmKind, CmRefusal, CmSource as _};
    let root = scratch("TickSocket");
    let store = pull::gdfl_tickstore::TickStore::new(&root);
    let d = day(2024, 4, 1);
    let path = store.day_path(CmKind::Indices, d);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    match store.day(CmKind::Indices, d) {
        Err(CmRefusal::TickStoreMalformed { what }) => {
            assert!(what.contains("a socket or a device"), "{what}");
        }
        other => panic!("{other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ═══ request minutes: the ledger names the run the sink holds ══════════════

/// A gap line names the run the installed sink holds, the key `/logs?run=`
/// groups a pull's events by, so the ledger and the log name one run.
/// Kills `run_ordinal` answering 0.
#[test]
fn request_minutes_a_gap_line_names_the_sinks_run() {
    let (_, sink) = sink();
    sink.set_run(4_242);
    let d = day(2025, 7, 1);
    let open = i64::from(d.days_from_epoch()) * 86_400 - 19_800 + 555 * 60;
    let rows = (1..375)
        .map(|minute| pull::fetch::RawRow {
            timestamp: open + minute * 60,
            open: 2_550_000,
            high: 2_550_040,
            low: 2_549_960,
            close: 2_550_010,
            volume: 400,
            open_interest: None,
        })
        .collect();
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: pull::session::Window::new(d, d).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::TrueData,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    };
    let store = scratch("GapRun");
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        "NIFTY",
        "Test",
        &store,
        plan,
    );
    assert!(
        done.failures.iter().any(|f| f
            .why
            .starts_with("request minute coverage gap on 2025-07-01")),
        "{:?}",
        done.failures
    );
    let ledger = std::fs::read_to_string(store.join("audit").join("truedata.gaps")).unwrap();
    let gap = ledger
        .lines()
        .find(|line| line.starts_with("gap "))
        .unwrap();
    assert!(
        gap.starts_with("gap NSE-INDEX-NIFTY 2025-07-01 v=1 rung=1min owed=375 held=374 missing=1")
            && gap.ends_with("why=vendor_empty run=4242"),
        "{ledger}"
    );
    // And each derived rung's line, written by the same run.
    let derived: Vec<&str> = ledger
        .lines()
        .filter(|line| line.starts_with("derive "))
        .collect();
    assert_eq!(derived.len(), 7, "{ledger}");
    assert!(
        derived.iter().all(|line| line.ends_with(" run=4242")),
        "{ledger}"
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ═══ a stored zip built here, for the archive and options readers ═════════

/// A plain stored zip of `members`, in order (APPNOTE 4.3, no ZIP64).
fn stored_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    stored_zip_with(members, &[])
}

/// [`stored_zip`], every local header carrying `extra` as its extra field
/// (the central records carry none, as APPNOTE 4.4.11 allows).
fn stored_zip_with(members: &[(&str, &[u8])], extra: &[u8]) -> Vec<u8> {
    fn le16(out: &mut Vec<u8>, v: usize) {
        out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
    }
    fn le32(out: &mut Vec<u8>, v: u64) {
        out.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in members {
        let offset = u64::try_from(out.len()).unwrap();
        let crc = u64::from(pull::gdfl_archive::crc32(data));
        let len = u64::try_from(data.len()).unwrap();
        le32(&mut out, 0x0403_4b50);
        for v in [20, 0, 0] {
            le16(&mut out, v);
        }
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, len);
        le32(&mut out, len);
        le16(&mut out, name.len());
        le16(&mut out, extra.len());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(extra);
        out.extend_from_slice(data);
        le32(&mut central, 0x0201_4b50);
        for v in [20, 20, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, 0);
        le32(&mut central, crc);
        le32(&mut central, len);
        le32(&mut central, len);
        le16(&mut central, name.len());
        for v in [0, 0, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = u64::try_from(out.len()).unwrap();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    for v in [0, 0, members.len(), members.len()] {
        le16(&mut out, v);
    }
    le32(&mut out, u64::try_from(central.len()).unwrap());
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

mod zip_readers {
    use super::{day, stored_zip};
    use pull::gdfl_archive::{Archive, stored_span, zip_entries};
    use pull::gdfl_cm::{CmKind, CmRefusal, CmScope, CmSource as _};
    use std::sync::Arc;

    /// An archive given a scope hands that scope to the caller and to every
    /// day listing it builds.
    #[test]
    fn an_archive_given_a_scope_lists_its_days_under_it() {
        let d = day(2024, 4, 1);
        let inner = stored_zip(&[("GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv", b"x")]);
        let outer = stored_zip(&[(
            "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip",
            &inner,
        )]);
        let len = u64::try_from(outer.len()).unwrap();
        let scope = Arc::new(CmScope::stored(std::iter::empty::<&str>()).unwrap());
        let archive = Archive::from_source(outer, len)
            .unwrap()
            .with_scope(Arc::clone(&scope));
        assert!(Arc::ptr_eq(&archive.scope(), &scope));
        let listing = archive.day(CmKind::Indices, d).unwrap().unwrap();
        assert!(listing.scope().is_stored());
        assert_eq!(listing.entries().len(), 1);
    }

    /// A stored member whose local header has lost its signature has no
    /// span: refused by name, never read from wherever the record points.
    #[test]
    fn a_stored_member_without_its_local_signature_has_no_span() {
        let mut zip = stored_zip(&[("APR_2024/GFDLNFO_TICK_01042024.zip", b"inner bytes")]);
        let len = u64::try_from(zip.len()).unwrap();
        let entries = zip_entries(&zip, 0, len).unwrap();
        let (at, n) = stored_span(&zip, &entries[0]).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&zip[usize::try_from(at).unwrap()..][..11], b"inner bytes");
        zip[0] ^= 0xff;
        assert_eq!(
            stored_span(&zip, &entries[0]),
            Err(CmRefusal::ArchiveMalformed {
                what: "a day zip's local header has no signature"
            })
        );
    }

    /// A stored member whose local header carries an extra field the central
    /// record does not: its span starts past the name AND the extra field.
    /// Kills the extra length added as subtracted.
    #[test]
    fn a_stored_members_span_skips_its_local_extra_field() {
        let name = "APR_2024/GFDLNFO_TICK_01042024.zip";
        let zip = super::stored_zip_with(&[(name, b"Inner")], b"EXTRA123");
        let len = u64::try_from(zip.len()).unwrap();
        let entries = zip_entries(&zip, 0, len).unwrap();
        let (at, n) = stored_span(&zip, &entries[0]).unwrap();
        assert_eq!((at, n), (30 + u64::try_from(name.len()).unwrap() + 8, 5));
        assert_eq!(&zip[usize::try_from(at).unwrap()..][..5], b"Inner");
    }

    /// An options year zip naming one day zip twice refuses that day; one
    /// that does not name the day answers no listing.
    #[test]
    fn an_options_year_zip_naming_a_day_twice_refuses_it_and_one_without_it_has_none() {
        let d = day(2024, 4, 1);
        let inner = stored_zip(&[(
            "GFDLNFO_TICK_01042024\\Options\\NIFTY04APR2422000CE.NFO.csv",
            b"x",
        )]);
        let name = "APR_2024/GFDLNFO_TICK_01042024.zip";
        let twice = stored_zip(&[(name, &inner), (name, &inner)]);
        let len = u64::try_from(twice.len()).unwrap();
        assert_eq!(
            pull::gdfl_nfo::listing_in(&Arc::new(twice), len, d).err(),
            Some(CmRefusal::ArchiveDuplicateDay { day: d })
        );
        let other = stored_zip(&[("APR_2024/GFDLNFO_TICK_02042024.zip", &inner)]);
        let len = u64::try_from(other.len()).unwrap();
        assert!(
            pull::gdfl_nfo::listing_in(&Arc::new(other), len, d)
                .unwrap()
                .is_none()
        );
        let once = stored_zip(&[(name, &inner)]);
        let len = u64::try_from(once.len()).unwrap();
        let listing = pull::gdfl_nfo::listing_in(&Arc::new(once), len, d)
            .unwrap()
            .unwrap();
        assert_eq!(listing.entries().len(), 1);
        assert_eq!(
            listing.member_bytes(),
            Some(u64::try_from(inner.len()).unwrap())
        );
    }
}

// ═══ gdfl_cm: the scope's edges, eligibility, and the trait's default ══════

mod cm_scope_and_auction {
    use super::{day, scratch};
    use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, Segment};
    use brutex_core::symbol::Symbol;
    use pull::gdfl_cm::{
        Auction, CmKind, CmRefusal, CmScope, CmSource, DayListing, ListedFile, RowFault,
    };
    use std::io::Write as _;
    use std::sync::Arc;

    /// The stored scope skips an option underlying that is an index (the
    /// index import files it through its alias row) and keeps a share; it
    /// files trade-for-trade series, the swept scope does not.
    #[test]
    fn the_stored_scope_skips_an_index_underlying_and_files_series() {
        let scope = CmScope::stored(["NIFTY", "ZZSHARE"]).unwrap();
        assert!(scope.shares().iter().any(|s| &**s == "ZZSHARE"));
        assert!(!scope.shares().iter().any(|s| &**s == "NIFTY"));
        assert!(scope.files_series());
        assert!(!CmScope::swept().files_series());
    }

    /// A futures key has no capital-market stem under the stored scope.
    #[test]
    fn a_futures_key_has_no_stem_under_the_stored_scope() {
        let scope = CmScope::stored(std::iter::empty::<&str>()).unwrap();
        let future = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("RELIANCE").unwrap(),
            kind: Kind::Future {
                expiry: Expiry::new(2031, 3, 27).unwrap(),
            },
        };
        assert_eq!(scope.stem_for(&future, day(2031, 3, 3)), None);
    }

    /// Each row fault's log word.
    #[test]
    fn every_row_fault_has_its_word() {
        assert_eq!(RowFault::ZeroLtp.code(), "zero_ltp");
        assert_eq!(RowFault::OffGrid.code(), "off_grid");
        assert_eq!(RowFault::Malformed.code(), "malformed");
    }

    fn isin(symbol: &str) -> &'static str {
        let at = brutex_core::universe::FNO_INDEX.position(symbol).unwrap();
        brutex_core::universe::FNO_UNDERLYINGS_ISIN[at]
    }

    fn master(root: &std::path::Path, d: pull::session::Day) {
        let csv = format!(
            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{},1\n1594,INFY,EQ,{},0\n",
            isin("RELIANCE"),
            isin("INFY")
        );
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(csv.as_bytes()).unwrap();
        pull::cash_session_cache::install(root, d, &gz.finish().unwrap()).unwrap();
    }

    /// A stocks day from 2026-08-03 answers each share's closing-auction
    /// rule from its receipted master: flag 1 eligible, flag 0 not, a share
    /// the master does not list unknown in the master's words, a share the
    /// universe records no ISIN for unknown by name; an index listing is
    /// never asked.
    #[test]
    fn a_receipted_master_answers_each_shares_auction_rule() {
        let root = scratch("CmMaster");
        let d = day(2026, 8, 4);
        master(&root, d);
        let scope = Arc::new(CmScope::swept().with_masters(root.clone()));
        let listing = DayListing::<()>::scoped(CmKind::Stocks, d, Arc::clone(&scope));
        let cash = |s: &str| InstrumentKey::cash(Exchange::Nse, s).unwrap();
        assert_eq!(listing.auction_for(&cash("RELIANCE")), Auction::Eligible);
        assert_eq!(listing.auction_for(&cash("INFY")), Auction::NotEligible);
        assert!(
            matches!(listing.auction_for(&cash("TCS")), Auction::Unknown(why) if !why.is_empty())
        );
        assert!(matches!(
            listing.auction_for(&cash("ZZNOISIN")),
            Auction::Unknown(why) if why.contains("records no ISIN")
        ));
        let index = DayListing::<()>::scoped(CmKind::Indices, d, scope);
        assert_eq!(
            index.auction_for(&InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()),
            Auction::NotAsked
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Two listings are equal only when their eligibility sources are: two
    /// reads of one receipted master are one; a master and no master, or
    /// two different reasons for none, are not.
    #[test]
    fn listings_compare_by_their_eligibility_source() {
        let root = scratch("CmMasterEq");
        let d = day(2026, 8, 4);
        master(&root, d);
        let with = Arc::new(CmScope::swept().with_masters(root.clone()));
        let none = Arc::new(CmScope::swept());
        let elsewhere = Arc::new(CmScope::swept().with_masters(root.join("Absent")));
        let listing =
            |scope: &Arc<CmScope>| DayListing::<()>::scoped(CmKind::Stocks, d, Arc::clone(scope));
        assert_eq!(listing(&with), listing(&with), "one master, read twice");
        assert_eq!(listing(&none), listing(&none), "one reason for none");
        assert_ne!(listing(&with), listing(&none));
        assert_ne!(listing(&none), listing(&with));
        assert_ne!(listing(&none), listing(&elsewhere), "two different reasons");
        // One scope, read before its master is installed and after: the
        // listings differ in nothing but where eligibility comes from.
        let later = scratch("CmMasterLater");
        let scope = Arc::new(CmScope::swept().with_masters(later.clone()));
        let before = listing(&scope);
        master(&later, d);
        let after = listing(&scope);
        assert_ne!(before, after, "no master, then a master");
        assert_ne!(after, before);
        assert_eq!(after, listing(&scope));
        // An index day asks no eligibility: two listings of it are one.
        assert_eq!(
            DayListing::<()>::new(CmKind::Indices, d),
            DayListing::<()>::new(CmKind::Indices, d)
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(later).unwrap();
    }

    /// A stocks day before 2026-08-03 asks no closing-auction eligibility,
    /// even of a scope that names no masters: the rule did not exist yet, so
    /// no share is unknown for want of a master.
    #[test]
    fn a_stocks_day_before_the_auction_change_asks_no_eligibility() {
        let listing = DayListing::<()>::new(CmKind::Stocks, day(2026, 7, 31));
        assert_eq!(
            listing.auction_for(&InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap()),
            Auction::NotAsked
        );
    }

    /// An F&O underlying that is an index has an empty ISIN in the F&O list,
    /// so a stocks day's auction rule for it is unknown for want of an ISIN:
    /// the master is never asked with an empty one.
    #[test]
    fn an_index_underlying_has_no_isin_to_ask_the_master_with() {
        assert_eq!(isin("FINNIFTY"), "");
        let root = scratch("CmMasterIndex");
        let d = day(2026, 8, 4);
        master(&root, d);
        let scope = Arc::new(CmScope::swept().with_masters(root.clone()));
        let listing = DayListing::<()>::scoped(CmKind::Stocks, d, scope);
        assert!(matches!(
            listing.auction_for(&InstrumentKey::cash(Exchange::Nse, "FINNIFTY").unwrap()),
            Auction::Unknown(why) if why.contains("core::universe records no ISIN")
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A day listing states its day zip's size only when its source read
    /// one: none for a fresh listing, exactly the bytes given otherwise.
    #[test]
    fn a_day_listing_states_its_zips_size_only_when_given_one() {
        let fresh = DayListing::<()>::new(CmKind::Indices, day(2024, 4, 2));
        assert_eq!(fresh.member_bytes(), None);
        assert_eq!(
            fresh.with_member_bytes(524_289).member_bytes(),
            Some(524_289)
        );
    }

    /// A source that does not say which scope it lists under lists under
    /// the swept surface: the trait's default.
    #[test]
    fn a_source_that_names_no_scope_lists_under_the_swept_surface() {
        struct Bare;
        impl CmSource for Bare {
            type Locator = ();
            fn day(
                &self,
                _kind: CmKind,
                _day: pull::session::Day,
            ) -> Result<Option<DayListing<()>>, CmRefusal> {
                Ok(None)
            }
            fn fetch(
                &self,
                _listing: &DayListing<()>,
                _file: &ListedFile<()>,
            ) -> Result<Vec<u8>, CmRefusal> {
                Err(CmRefusal::NotSwept)
            }
        }
        let scope = Bare.scope();
        assert!(Arc::ptr_eq(&scope, &CmScope::swept_shared()));
        assert!(!scope.is_stored());
        assert!(
            Bare.day(CmKind::Indices, day(2024, 4, 1))
                .unwrap()
                .is_none()
        );
    }
}

// ═══ extreme inputs, end to end through a stored archive ═══════════════════

mod cm_extremes {
    use super::{day, scratch, stored_zip};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::gdfl_archive::Archive;
    use pull::gdfl_cm::{CmRefusal, CmScope, HEADER_OPEN_INTEREST, read_day};
    use std::io::Write as _;
    use std::sync::Arc;

    /// An outer archive holding one stocks day zip of `files` for `d`.
    fn stocks_day(d: pull::session::Day, files: &[(&str, &[u8])]) -> Archive<Vec<u8>> {
        let folder = format!(
            "GFDLCM_STOCK_TICK_{:02}{:02}{:04}",
            d.day(),
            d.month(),
            d.year()
        );
        let named: Vec<(String, &[u8])> = files
            .iter()
            .map(|(name, bytes)| (format!("{folder}/{name}"), *bytes))
            .collect();
        let members: Vec<(&str, &[u8])> = named.iter().map(|(n, b)| (n.as_str(), *b)).collect();
        let inner = stored_zip(&members);
        let month = [
            "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
        ][usize::from(d.month()) - 1];
        let outer_name = format!("STOCKS/{}/{month}_{}/{folder}.zip", d.year(), d.year());
        let outer = stored_zip(&[(outer_name.as_str(), &inner)]);
        let len = u64::try_from(outer.len()).unwrap();
        Archive::from_source(outer, len).unwrap()
    }

    fn row(stem: &str, d: &str, time: &str, ltp: &str, bid: &str, ask: &str, ltq: u32) -> String {
        format!("{stem},{d},{time},{ltp},{bid},10,{ask},10,{ltq},0")
    }

    /// A share named with `&` (`M&M`) is read through its own stem. Every
    /// row is kept in file order: a pre-open row, an in-session trade, the
    /// same trade stated twice, a LATE row stamped before the one ahead of
    /// it, a quote-only row (LTQ 0), a post-close row, and a last row with no
    /// newline. The largest stamp is the running maximum, which the late
    /// row does not raise; a quote-only row is counted, never usable alone.
    #[test]
    fn duplicate_late_and_quote_only_rows_are_kept_in_file_order() {
        let d = day(2024, 4, 1);
        let date = "01/04/2024";
        let stem = "M&M.NSE";
        let rows = [
            row(stem, date, "09:07:01", "1850.50", "1850.00", "1851.00", 5),
            row(stem, date, "10:00:00", "1852.00", "1851.95", "1852.05", 7),
            row(stem, date, "10:00:00", "1852.00", "1851.95", "1852.05", 7),
            row(stem, date, "09:59:58", "1851.75", "1851.70", "1851.80", 3),
            row(stem, date, "11:00:00", "1853.00", "1852.95", "1853.05", 0),
            row(stem, date, "15:45:00", "1854.00", "0", "0", 2),
        ];
        let text = format!("{HEADER_OPEN_INTEREST}\r\n{}", rows.join("\r\n"));
        let archive = stocks_day(d, &[("M&M.NSE.csv", text.as_bytes())]);
        let key = InstrumentKey::cash(Exchange::Nse, "M&M").unwrap();
        let file = read_day(&archive, &key, d).unwrap().unwrap().file;
        let stamps: Vec<u32> = file.rows.iter().map(|r| r.sod).collect();
        assert_eq!(stamps, [32_821, 36_000, 36_000, 35_998, 39_600, 56_700]);
        assert_eq!(
            file.rows.iter().map(|r| r.line).collect::<Vec<_>>(),
            [2, 3, 4, 5, 6, 7]
        );
        assert_eq!(
            (file.rows_pre, file.rows_in_session, file.rows_post),
            (1, 4, 1)
        );
        assert_eq!(file.ltq_zero_rows, 1);
        assert_eq!(file.max_sod, 56_700);
        assert!(!file.final_newline);
        assert_eq!(file.refused.total(), 0);
    }

    /// A zero-length file, a header with no row, a file cut inside its last
    /// row, and a file of one quote-only row each refuse by name.
    #[test]
    fn empty_header_only_cut_and_quote_only_files_refuse_by_name() {
        let d = day(2024, 4, 1);
        let key = InstrumentKey::cash(Exchange::Nse, "M&M").unwrap();
        let header = format!("{HEADER_OPEN_INTEREST}\r\n");
        let cut = format!("{header}M&M.NSE,01/04/2024,10:0");
        let quote_only = format!(
            "{header}{}\r\n",
            row(
                "M&M.NSE",
                "01/04/2024",
                "10:00:00",
                "1852.00",
                "1851.95",
                "1852.05",
                0
            )
        );
        for (bytes, want) in [
            (Vec::new(), CmRefusal::HeaderUnknown),
            (header.clone().into_bytes(), CmRefusal::NoSessionRows),
            (cut.into_bytes(), CmRefusal::MalformedRow { line: 2 }),
            (quote_only.into_bytes(), CmRefusal::NoSessionRows),
        ] {
            let archive = stocks_day(d, &[("M&M.NSE.csv", &bytes)]);
            assert_eq!(read_day(&archive, &key, d), Err(want.clone()), "{want:?}");
        }
    }

    /// From 2026-08-03 an eligible share's closing-auction print (bid and
    /// ask zero, LTQ positive, in the auction band) is diverted out of the
    /// rows the fold sees, the same print stated twice merged with its
    /// count; with no security master the same file is refused by name.
    #[test]
    fn an_eligible_shares_closing_auction_print_is_diverted_from_its_rows() {
        let d = day(2026, 8, 4);
        let date = "04/08/2026";
        let stem = "RELIANCE.NSE";
        let rows = [
            row(stem, date, "15:00:00", "1400.00", "1399.95", "1400.05", 9),
            row(stem, date, "15:28:07", "1401.50", "0", "0", 400),
            row(stem, date, "15:28:07", "1401.50", "0", "0", 400),
            row(stem, date, "15:40:00", "1401.50", "1401.45", "1401.55", 1),
        ];
        let text = format!("{HEADER_OPEN_INTEREST}\r\n{}\r\n", rows.join("\r\n"));
        let masters = scratch("CmAuctionMasters");
        let at = brutex_core::universe::FNO_INDEX
            .position("RELIANCE")
            .unwrap();
        let isin = brutex_core::universe::FNO_UNDERLYINGS_ISIN[at];
        let csv = format!(
            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{isin},1\n"
        );
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(csv.as_bytes()).unwrap();
        pull::cash_session_cache::install(&masters, d, &gz.finish().unwrap()).unwrap();
        let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
        let scope = Arc::new(CmScope::swept().with_masters(masters.clone()));
        let archive = stocks_day(d, &[("RELIANCE.NSE.csv", text.as_bytes())]).with_scope(scope);
        let file = read_day(&archive, &key, d).unwrap().unwrap().file;
        assert_eq!(
            file.rows.iter().map(|r| r.sod).collect::<Vec<_>>(),
            [54_000, 56_400]
        );
        assert_eq!(file.auction.len(), 1);
        let print = file.auction[0];
        assert_eq!(
            (
                print.sod,
                print.price,
                print.quantity,
                print.rows,
                print.line
            ),
            (55_687, 140_150, 400, 2, 3)
        );
        let blind = stocks_day(d, &[("RELIANCE.NSE.csv", text.as_bytes())]);
        assert!(matches!(
            read_day(&blind, &key, d),
            Err(CmRefusal::AuctionEligibilityUnknown { .. })
        ));
        std::fs::remove_dir_all(masters).unwrap();
    }
}

/// A directory where a day file belongs is unavailable as a directory (the
/// host's own refusal), not a malformed file and not an absent day.
#[test]
fn tickstore_a_directory_at_a_day_name_is_unavailable_as_a_directory() {
    use pull::gdfl_cm::{CmKind, CmRefusal, CmSource as _};
    let root = scratch("TickDirectory");
    let store = pull::gdfl_tickstore::TickStore::new(&root);
    let d = day(2024, 4, 2);
    std::fs::create_dir_all(store.day_path(CmKind::Indices, d)).unwrap();
    assert_eq!(
        store.day(CmKind::Indices, d).err(),
        Some(CmRefusal::TickStoreUnavailable {
            kind: std::io::ErrorKind::IsADirectory
        })
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ═══ gdfl_nfo: the day zip's size ═════════════════════════════════════════

/// An options day's listing states the size of the day zip it was read
/// from only when its source read one: none for a fresh listing (the tick
/// store's, which holds no zip), and exactly the bytes it was given
/// otherwise.
#[test]
fn nfo_a_listing_states_its_day_zips_size_only_when_given_one() {
    let fresh: pull::gdfl_nfo::NfoDay<u64> = pull::gdfl_nfo::NfoDay::new(day(2024, 4, 2), 0);
    assert_eq!(fresh.member_bytes(), None);
    assert_eq!(
        fresh.with_member_bytes(7_340_033).member_bytes(),
        Some(7_340_033)
    );
}

/// Before the day-form switch an index weekly's dated reading must expire
/// in the three-month cycle, counted in calendar months across a year end:
/// a dated expiry three months on is read dated, and one fifteen months on
/// (December of the next year, an earlier calendar month than the trade's)
/// is no dated reading, so the name reads monthly. Invented strikes.
#[test]
fn nfo_a_pre_switch_index_weekly_is_dated_only_inside_the_three_month_cycle() {
    use pull::gdfl_nfo::{TickerForm, decode_ticker};
    let on = day(2018, 9, 10);
    let near = decode_ticker("BANKNIFTY13DEC1826000CE", on).unwrap();
    assert_eq!(
        (near.form, near.contract.as_str()),
        (TickerForm::Day, "2018-12-13-2600000-CE")
    );
    let far = decode_ticker("BANKNIFTY18DEC19500CE", on).unwrap();
    assert_eq!(
        (far.form, far.contract.as_str()),
        (TickerForm::Monthly, "2018-12-27-1950000-CE")
    );
}
