#![cfg(test)]
//! Generated finite storage fixtures; these are never presented as market research.
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath};
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn with_warmed_store<R>(run: impl FnOnce(&std::path::Path) -> R) -> R {
    let fixture = Fixture::warmed();
    run(&fixture.root)
}

struct Fixture {
    root: PathBuf,
    /// The swept instrument every file and request names. NIFTY unless a
    /// test needs a cash equity, which D-0694's statement is about.
    symbol: &'static str,
}
impl Fixture {
    fn new() -> Self {
        Self::for_symbol("NIFTY")
    }
    fn for_symbol(symbol: &'static str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-audited-input-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("scratch");
        let fixture = Self { root, symbol };
        for (month, day) in [(4, 30), (5, 2)] {
            let rows = generated_session(month, day);
            assert!(!rows.is_empty());
            fixture.write(month, Timeframe::MINUTE_1, &rows);
            fixture.write(month, Timeframe::DAY_1, &rows[..1]);
            if month == 5 {
                fixture.write(
                    month,
                    Timeframe::MINUTE_5,
                    &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
                );
            }
        }
        fixture
    }
    fn warmed() -> Self {
        Self::warmed_for("NIFTY")
    }
    fn warmed_for(symbol: &'static str) -> Self {
        let fixture = Self::for_symbol(symbol);
        for day in 5..=13 {
            let rows = generated_session(5, day);
            if rows.is_empty() {
                continue;
            }
            fixture.write(5, Timeframe::MINUTE_1, &rows);
            fixture.write(5, Timeframe::DAY_1, &rows[..1]);
            fixture.write(
                5,
                Timeframe::MINUTE_5,
                &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
            );
        }
        fixture
    }
    fn path(&self, month: u8, timeframe: Timeframe) -> PathBuf {
        let key = stored::swept_index(self.symbol).expect("key");
        StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path")
        .to_path_buf(&self.root)
    }
    fn write(&self, month: u8, timeframe: Timeframe, rows: &[Bar]) {
        let mut file = self.open(month, timeframe);
        file.append(rows).expect("generated fixture rows");
    }
    /// The writer of `month` at `timeframe`, which creates the file when it
    /// is absent. Dropped unwritten, it leaves a month that holds no bar.
    fn open(&self, month: u8, timeframe: Timeframe) -> BarFile {
        let key = stored::swept_index(self.symbol).expect("key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path");
        let hash = brutex_core::universe::fnv1a(self.symbol).to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        BarFile::open_or_create(&self.root, path, symbol).expect("writer")
    }
    fn request<'a>(&'a self, rung: &'a str) -> Request<'a> {
        Request {
            store_root: &self.root,
            vendor: Vendor::Zerodha,
            underlying: self.symbol,
            rung,
            year: 2025,
            month: 5,
            receipt_root: &self.root,
            max_bytes: 1_048_576,
            max_records: 10_000,
        }
    }

    fn audit(&self, rung: &str) -> Result<String, String> {
        crate::audit_stored_kernel(crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.symbol,
            rung,
            year: 2025,
            month: 5,
            min_hits: u64::MAX,
            commit: "generated-stored-audit-fixture",
        })
    }

    fn audit_range(&self, rung: &str, to: (u16, u8)) -> Result<String, String> {
        crate::audit_range_kernel(crate::StoredRangeAuditRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.symbol,
            rung,
            from: (2025, 5),
            to,
            min_hits: u64::MAX,
            attempt: Some(7),
            commit: "generated-stored-range-audit-fixture",
        })
    }

    fn screen(&self, rung: &str, support_ppm: u64) -> Result<String, String> {
        crate::screen_range_kernel(crate::StoredScreenRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.symbol,
            rung,
            span: ((2025, 5), (2025, 5)),
            support_ppm,
            policy: crate::Policy {
                rules: crate::Rules::BASELINE,
                lens: runner::rank::Lens::Detectability,
                validate: false,
            },
            attempt: Some(13),
            commit: "generated-stored-screen-fixture",
        })
    }

    /// The ordinary `sweep-stored` door over May 2025. D-0694.
    fn sweep(&self, rung: &str) -> Result<String, String> {
        crate::sweep_stored_kernel(self.month_request(rung))
    }

    fn month_request<'a>(&self, rung: &'a str) -> crate::StoredSweepRequest<'a> {
        crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.symbol,
            rung,
            year: 2025,
            month: 5,
            min_hits: u64::MAX,
            commit: "generated-stored-sweep-fixture",
        }
    }

    /// The `auto-stored` threshold search over May 2025. D-0694.
    fn auto(&self, rung: &str) -> Result<String, String> {
        crate::auto_stored_kernel(
            &self.root,
            Vendor::Zerodha,
            self.symbol,
            rung,
            ((2025, 5), (2025, 5)),
            "generated-stored-auto-fixture",
        )
    }

    fn omit_owned_minutes(&self, holed_days: &[u8]) {
        self.rewrite_owned_minutes(|day, rows| {
            if holed_days.contains(&day) {
                rows.remove(150);
            }
        });
    }

    fn rewrite_owned_minutes(&self, change: impl Fn(u8, &mut Vec<Bar>)) {
        let path = self.path(5, Timeframe::MINUTE_1);
        fs::remove_file(&path).expect("replace owned generated minute file");
        fs::remove_file(path.with_extension("crc")).expect("replace its owned proof");
        for day in [2, 5, 6, 7, 8, 9, 12, 13] {
            let mut rows = generated_session(5, day);
            change(day, &mut rows);
            self.write(5, Timeframe::MINUTE_1, &rows);
        }
    }
}

/// Moves record `index`'s stamp by `delta_micros` in place, resealing its
/// checksum block and, at either end, the header slot, exactly as the writer
/// would have.
///
/// **A FILE FROM BEFORE D-0915.** `BarFile::append` now refuses an off-grid
/// stamp (`StoreError::OffGrid`), so a sealed month holding one can only be a
/// file written before that refusal existed. Those files still exist and this
/// reader's own malformed-span refusal is what still faces them, so the state
/// is laid by hand rather than abandoned.
fn forge_pre_admission_stamp(path: &std::path::Path, index: u64, delta_micros: i64) {
    use store::format::{HEADER_LEN, Row};
    let mut bytes = fs::read(path).expect("the sealed month");
    let region = usize::try_from(HEADER_LEN).expect("region");
    let header = store::header::Header::read_region(&bytes[..region], bytes.len() as u64)
        .expect("a committed header");
    let layout = store::layout::Layout::for_version(header.format_version).expect("layout");
    let at = usize::try_from(layout.offset_of(index).expect("offset")).expect("offset");
    let mut bar = Bar::read_from(&bytes[at..at + Bar::LEN]).expect("a record");
    bar.ts_micros += delta_micros;
    bytes[at..at + Bar::LEN].copy_from_slice(&bar.image());
    let block = index / layout.records_per_block();
    let (start, end) = layout
        .covered_byte_range(block, header.n_valid)
        .expect("covered range");
    let span = &bytes[usize::try_from(start).expect("s")..usize::try_from(end).expect("e")];
    let sum = store::block::seal(layout, header.n_valid, block, span).expect("seal");
    let crc_path = path.with_extension("crc");
    let mut crc = fs::read(&crc_path).expect("the sidecar");
    let entry = usize::try_from(block * 4).expect("entry");
    crc[entry..entry + 4].copy_from_slice(&sum.to_le_bytes());
    let mut resealed = header;
    if index == 0 {
        resealed.first_ts_micros = bar.ts_micros;
    }
    if index + 1 == header.n_valid {
        resealed.last_ts_micros = bar.ts_micros;
    }
    let commit = resealed.commit().expect("a header image");
    let slot = usize::try_from(commit.offset).expect("slot");
    bytes[slot..slot + commit.bytes.len()].copy_from_slice(&commit.bytes);
    fs::write(path, &bytes).expect("rewrite the month");
    fs::write(&crc_path, &crc).expect("rewrite the sidecar");
}

fn generated_session(month: u8, date: u8) -> Vec<Bar> {
    let civil = pull::session::Day::new(2025, month, date).expect("date");
    let day = i64::from(civil.days_from_epoch());
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .map(|minute| Bar {
            ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                - indicators::IST_OFFSET_MICROS,
            open: 100_000,
            high: 110_000,
            low: 90_000,
            close: 101_000,
            volume: 100,
            open_interest: i64::MIN,
        })
        .collect()
}

#[test]
fn audited_month_publication_is_idempotent_and_stale_inputs_cannot_publish() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let input = Inputs::load(fixture.request(rung)).expect("audited generated inputs");
        let run = |inputs: &Inputs| {
            crate::stored_month_kernel(
                crate::StoredSweepRequest {
                    root: fixture.root.clone(),
                    vendor: Vendor::Zerodha,
                    underlying: "NIFTY",
                    rung,
                    year: 2025,
                    month: 5,
                    min_hits: u64::MAX,
                    // Explicit generated-test identity, never operator provenance.
                    commit: "generated-audited-month-fixture",
                },
                inputs.data(),
                Some(inputs),
            )
        };
        let first = run(&input).expect("bounded complete generated sweep");
        assert!(first.contains("RESULT RECORDED"));
        assert!(first.contains("six-role input binding"));
        assert!(!first.contains(crate::NOT_RECORDED));
        let path = crate::results::Results::path(&fixture.root);
        let saved = fs::read(&path).expect("published ledger");
        let second = run(&input).expect("exact rerun");
        assert!(second.contains("RESULT ALREADY RECORDED AND VERIFIED"));
        assert_eq!(fs::read(&path).expect("rerun ledger"), saved);
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("cold ledger");
        assert_eq!(ledger.len().expect("count"), 1);
        let row = ledger.read(0).expect("only acknowledged parent");
        assert_eq!(row.min_hits, u64::MAX);
        assert_eq!(row.trades, 0);
        assert_eq!(row.combinations, 0);
        assert_eq!(row.halted, 0);
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        drop(ledger);

        let source = fixture.path(5, Timeframe::MINUTE_1);
        let original = fs::read(&source).expect("source bytes");
        let mut changed = original.clone();
        changed[24] ^= 1;
        fs::write(&source, changed).expect("corrupt the generated source");
        assert!(run(&input).is_err());
        assert_eq!(fs::read(&path).expect("refused ledger"), saved);
        fs::write(&source, original).expect("restore the generated source");
        let reloaded = Inputs::load(fixture.request(rung)).expect("fresh source authentication");
        assert!(
            run(&reloaded)
                .expect("restored rerun")
                .contains("RESULT ALREADY RECORDED")
        );
        assert_eq!(fs::read(&path).expect("restored ledger"), saved);
    }
    crate::knobs::clear_all();
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn stored_screens_scale_support_to_retained_bars_and_disclose_holed_sessions() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    // An explicit finite operator budget also keeps a generated constant-price
    // vocabulary from consuming a machine if new always-true bits are added.
    crate::knobs::set("BRUTEX_CEILING", "256");
    // `swept` is the ledger's `bars`: what the column folded past warm-up, as
    // every door records it since D-1661. Support stays scaled to `retained`.
    for (rung, retained, withheld, swept) in [("1min", 2_625, 374, 1_500), ("5min", 525, 75, 300)] {
        let fixture = Fixture::warmed();
        fixture.omit_owned_minutes(&[5]);
        let source = fixture.path(5, Timeframe::MINUTE_1);
        let source_bytes = fs::read(&source).expect("owned incomplete minute source");
        let report = fixture.screen(rung, 1_000_000).expect("loaded screen");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("screen ledger");
        assert_eq!(ledger.len().expect("parent count"), 1);
        let row = ledger.read(0).expect("screen parent");
        assert_eq!(row.bars, swept, "{rung}");
        assert_eq!(
            row.min_hits, retained,
            "100% support must use the retained sample"
        );
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        assert!(report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
        assert!(report.contains("2025-05-05"), "{report}");
        assert!(
            report.contains(&format!("{withheld} signal bar(s)")),
            "{report}"
        );
        assert_eq!(fs::read(&source).expect("unmodified source"), source_bytes);
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screen_support_and_exact_retries_bind_the_actual_sample() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    // `bars` is the retained sample support scales to; `swept` the ledger's
    // count past warm-up (D-1661).
    for (rung, bars, swept) in [("1min", 3_000, 1_500), ("5min", 600, 300)] {
        let fixture = Fixture::warmed();
        let mut identities = Vec::new();
        for (index, support) in [20_000, 50_000, 1_000_000].into_iter().enumerate() {
            let report = fixture.screen(rung, support).expect("screen request");
            assert!(report.contains("RESULT RECORDED"), "{report}");
            assert!(!report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
            let path = crate::results::Results::path(&fixture.root);
            let saved = fs::read(&path).expect("parent ledger");
            let mut ledger =
                crate::results::Results::open_read(&fixture.root).expect("screen ledger");
            assert_eq!(
                ledger.len().expect("distinct support count"),
                index as u64 + 1
            );
            let row = ledger.read(index as u64).expect("exact screen parent");
            assert_eq!(row.bars, swept, "{rung}");
            assert_eq!(row.min_hits, bars * support / 1_000_000);
            assert_eq!((row.months_asked, row.months_found), (1, 1));
            assert!(!identities.contains(&row.identity));
            identities.push(row.identity);
            drop(ledger);
            let retry = fixture.screen(rung, support).expect("same screen again");
            assert!(retry.contains("RESULT ALREADY RECORDED"), "{retry}");
            assert_eq!(fs::read(&path).expect("retry bytes"), saved);
            let attempt = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
                .expect("read durable attempt")
                .expect("screen attempt exists");
            assert_eq!(attempt.identity, row.identity);
            assert_eq!(
                attempt.completion == crate::sweep_evidence::Completion::Completed,
                row.halted == 0
            );
        }
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screens_refuse_missing_or_corrupt_authorities_before_recording() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (month, timeframe, corrupt) in [
        (5, Timeframe::MINUTE_1, false),
        (4, Timeframe::DAY_1, false),
        (4, Timeframe::MINUTE_1, false),
        (5, Timeframe::MINUTE_5, true),
    ] {
        let fixture = Fixture::warmed();
        let path = fixture.path(month, timeframe);
        let original = fs::read(&path).expect("owned authority");
        let damaged = if corrupt {
            let mut bytes = original;
            bytes[committed_record_byte()] ^= 1;
            fs::write(&path, &bytes).expect("damage owned authority");
            Some(bytes)
        } else {
            fs::remove_file(&path).expect("remove owned required source");
            None
        };
        let refusal = fixture
            .screen("5min", 1_000_000)
            .expect_err("source must refuse");
        assert!(!refusal.is_empty());
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no premature attempt"),
            None
        );
        assert_eq!(
            fs::read(&path).ok(),
            damaged,
            "no source repair or creation"
        );
    }
    crate::knobs::clear_all();
}

/// A byte inside the first committed record, which that record's block checksum covers.
///
/// Not byte 24. Byte 24 is the newest header slot's `n_valid`, and damaging it fails that
/// slot's own checksum, which is exactly what a crash during the slot write leaves. The
/// store reads such a file as the previous commit (D-0688), so it is a torn commit and not a
/// corrupt authority. A committed record byte lies inside every extent the tail-block proof
/// can try, so it is refused as the corruption it is.
fn committed_record_byte() -> usize {
    let first = store::layout::Layout::CURRENT
        .offset_of(0)
        .expect("record 0 has an offset");
    usize::try_from(first).expect("the header region fits a usize") + 8
}

/// A damaged newest header slot is a torn commit, and it reads as the previous one.
///
/// `Fixture::warmed` commits eight sessions, so the 5min month's newest slot holds
/// generation 8 over 600 bars and the other holds generation 7 over 525. Byte 24 is
/// generation 8's `n_valid`. Flipping it fails that slot's checksum, which is byte for byte
/// what a crash during the slot write leaves, and the store's crash table says a reader then
/// sees generation 7. Before D-0688 the tail block, sealed over generation 8's records,
/// refused that read, and this module's refusal tests leaned on the refusal. Now the screen
/// reads 525 bars and records them under their own identity, never the 600-bar one, and the
/// store logs both the fallback and the interrupted append.
#[test]
fn a_damaged_newest_header_slot_screens_as_the_previous_commit() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    let path = fixture.path(5, Timeframe::MINUTE_5);
    let whole = fixture.screen("5min", 1_000_000).expect("the whole month");
    assert!(whole.contains("· 600 bars ·"), "{whole}");

    let mut torn = fs::read(&path).expect("owned authority");
    torn[24] ^= 1;
    fs::write(&path, &torn).expect("tear the newest slot");
    let previous = fixture
        .screen("5min", 1_000_000)
        .expect("the previous commit screens");
    assert!(previous.contains("· 525 bars ·"), "{previous}");
    assert!(previous.contains("RESULT RECORDED"), "{previous}");

    let mut ledger = crate::results::Results::open_read(&fixture.root).expect("ledger");
    assert_eq!(ledger.len().expect("two parents"), 2);
    let (first, second) = (
        ledger.read(0).expect("whole month"),
        ledger.read(1).expect("previous commit"),
    );
    // `bars` is the swept count past warm-up, as every door records it since
    // D-1661: the 600- and 525-bar slices sweep 300 and 225 signal bars.
    assert_eq!((first.bars, second.bars), (300, 225));
    assert_ne!(first.identity, second.identity);
    drop(ledger);
    assert_eq!(
        fs::read(&path).expect("unrepaired"),
        torn,
        "a read repairs nothing"
    );
    crate::knobs::clear_all();
}

#[test]
fn stored_screens_refuse_when_every_signal_session_is_withheld() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    fixture.omit_owned_minutes(&[2, 5, 6, 7, 8, 9, 12, 13]);
    for rung in ["1min", "5min"] {
        let refusal = fixture
            .screen(rung, 20_000)
            .expect_err("no retained session");
        assert_eq!(
            refusal,
            "every signal session has a minute gap; no screenable bars remain"
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no empty attempt"),
            None
        );
    }
    crate::knobs::clear_all();
}

#[test]
fn stored_screens_reject_off_grid_execution_before_creating_an_attempt() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    let path = fixture.path(5, Timeframe::MINUTE_1);
    forge_pre_admission_stamp(&path, 0, 1);
    let before = fs::read(&path).expect("owned off-grid execution stream");
    let checksum = path.with_extension("crc");
    let proof = fs::read(&checksum).expect("matching raw-record checksum");
    for rung in ["1min", "5min"] {
        let refusal = fixture
            .screen(rung, 20_000)
            .expect_err("off-grid execution is not a gap");
        assert!(
            refusal.starts_with("the 1min execution span is malformed:"),
            "{refusal}"
        );
        assert!(refusal.contains("record 0"), "{refusal}");
        assert!(
            refusal.contains("off the exact one-minute grid"),
            "{refusal}"
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no attempted screen"),
            None
        );
        assert_eq!(fs::read(&path).expect("unaltered source"), before);
        assert_eq!(fs::read(&checksum).expect("unaltered proof"), proof);
    }
    crate::knobs::clear_all();
}

#[test]
fn public_screen_admission_preserves_rung_and_build_or_feed_refusals() {
    let policy = crate::Policy {
        rules: crate::Rules::BASELINE,
        lens: runner::rank::Lens::Detectability,
        validate: false,
    };
    for rung in ["", "2min", "1day", "1min"] {
        let plain = crate::screen_range(
            "unknown-generated-feed",
            "NIFTY",
            rung,
            (2025, 5),
            (2025, 5),
            20_000,
            policy,
        );
        let attempted = crate::screen_range_for_attempt(
            "unknown-generated-feed",
            "NIFTY",
            rung,
            ((2025, 5), (2025, 5)),
            20_000,
            policy,
            Some(13),
            &mut crate::ScreenCache::default(),
        );
        assert_eq!(plain, attempted);
        assert!(plain.starts_with("refused: "), "{plain}");
        assert!(!plain.contains(crate::STORED_PROVENANCE), "{plain}");
        assert!(!plain.contains("RESULT RECORDED"), "{plain}");
        if rung == "1min" && crate::commit_stamp().is_none() {
            assert!(plain.contains("no verified commit stamp"), "{plain}");
        }
    }
}

fn generated_public_command_flow(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    crate::knobs::set("BRUTEX_VALIDATE", "0");
    let commands: &[&[&str]] = &[
        &[
            "auto-stored",
            "zerodha",
            "NIFTY",
            "1min",
            "2025",
            "5",
            "2025",
            "5",
        ],
        &[
            "screen", "zerodha", "NIFTY", "1min", "2025", "5", "2025", "5", "1000000", "1", "200",
            "1",
        ],
    ];
    for words in commands {
        let args = words.iter().map(ToString::to_string).collect::<Vec<_>>();
        let mut report = String::new();
        let status = crate::dispatch(&args, &mut report);
        if crate::commit_stamp().is_none() {
            assert_eq!(status, crate::MISUSED, "{report}");
            assert!(report.contains("no verified commit stamp"), "{report}");
            assert!(!report.contains(crate::STORED_PROVENANCE), "{report}");
            assert!(!crate::results::Results::path(root).exists());
            continue;
        }
        assert_eq!(status, crate::OK, "{report}");
        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
        if words[0] == "auto-stored" {
            for text in [
                "BUDGET  256 candidates -- BRUTEX_CEILING",
                "3000 bars over 1 of 1 months",
                "SEARCH",
                "threshold chosen",
            ] {
                assert!(report.contains(text), "missing {text}: {report}");
            }
            assert!(crate::sweep_evidence::latest(root, 1_048_576)?.is_some());
        } else {
            assert!(report.contains("RESULT RECORDED"), "{report}");
            assert!(report.contains(crate::UNVALIDATED), "{report}");
            let mut ledger = crate::results::Results::open_read(root)?;
            assert_eq!(ledger.len()?, 1);
            let row = ledger.read(0)?;
            assert_eq!(
                (row.bars, row.min_hits, row.months_asked, row.months_found),
                (3_000, 3_000, 1, 1)
            );
            assert_eq!(row.halted, 0);
            let attempt =
                crate::sweep_evidence::latest(root, 1_048_576)?.ok_or("screen attempt")?;
            assert_eq!(attempt.identity, row.identity);
            assert_eq!(
                attempt.completion,
                crate::sweep_evidence::Completion::Completed
            );
            let path = crate::results::Results::path(root);
            let saved = fs::read(&path)?;
            let mut retry = String::new();
            assert_eq!(crate::dispatch(&args, &mut retry), crate::OK, "{retry}");
            assert!(retry.contains("RESULT ALREADY RECORDED"), "{retry}");
            assert_eq!(fs::read(path)?, saved);
        }
    }
    crate::knobs::clear_all();
    Ok(())
}

#[test]
fn public_generated_probe_and_screen_agree_with_durable_results()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_TEST_PUBLIC_SCREEN_PIPELINE";
    if std::env::var_os(CHILD).is_some() {
        return generated_public_command_flow(&crate::store_root()?);
    }
    let fixture = Fixture::warmed();
    let mut original = Vec::new();
    for (month, timeframe) in [
        (4, Timeframe::MINUTE_1),
        (4, Timeframe::DAY_1),
        (5, Timeframe::MINUTE_1),
        (5, Timeframe::MINUTE_5),
        (5, Timeframe::DAY_1),
    ] {
        let path = fixture.path(month, timeframe);
        original.push((path.clone(), fs::read(&path)?));
        let checksum = path.with_extension("crc");
        original.push((checksum.clone(), fs::read(checksum)?));
    }
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "audited_stored::tests::public_generated_probe_and_screen_agree_with_durable_results",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "generated")
        .env("BRUTEX_STORE", &fixture.root)
        .env("BRUTEX_LOG_DIR", fixture.root.join("logs"))
        .output()?;
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    for (path, bytes) in original {
        assert_eq!(fs::read(path)?, bytes);
    }
    Ok(())
}

#[test]
fn monthly_audits_publish_empty_extinction_and_exact_retry_identity_at_both_resolutions() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let report = fixture.audit(rung).expect("actual stored audit");
        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
        assert!(report.contains("generated-stored-audit-fixture"));
        assert!(report.contains("RESULT RECORDED"), "{report}");
        assert!(!report.contains(crate::NOT_RECORDED), "{report}");
        assert!(!report.contains("NOTHING MEASURED"), "{report}");
        let path = crate::results::Results::path(&fixture.root);
        let original = fs::read(&path).expect("actual parent ledger");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("cold ledger");
        assert_eq!(ledger.len().expect("parent count"), 1);
        let row = ledger.read(0).expect("saved audit parent");
        assert!(row.bars > 0);
        assert_eq!((row.trades, row.combinations, row.halted), (0, 0, 0));
        assert_eq!(row.min_hits, u64::MAX);
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        drop(ledger);
        let first = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read first audit evidence")
            .expect("durable audit attempt");
        assert_eq!(first.identity, row.identity);
        assert_eq!(
            first.completion,
            crate::sweep_evidence::Completion::Completed
        );

        let retry = fixture.audit(rung).expect("exact audit retry");
        assert!(
            retry.contains("RESULT ALREADY RECORDED AND VERIFIED"),
            "{retry}"
        );
        assert_eq!(fs::read(&path).expect("retry ledger"), original);
        let second = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read retry evidence")
            .expect("durable retry");
        assert!(second.attempt > first.attempt);
        assert_eq!(second.identity, first.identity);
        assert_eq!(
            second.completion,
            crate::sweep_evidence::Completion::Completed
        );

        let source = fixture.path(5, Timeframe::MINUTE_1);
        let saved = fs::read(&source).expect("original minutes");
        let mut corrupt = saved.clone();
        corrupt[committed_record_byte()] ^= 1;
        fs::write(&source, corrupt).expect("corrupt owned minute authority");
        assert!(fixture.audit(rung).is_err());
        assert_eq!(fs::read(&path).expect("refused ledger"), original);
        fs::write(&source, saved).expect("restore owned authority");
        assert!(
            fixture
                .audit(rung)
                .expect("restored audit")
                .contains("RESULT ALREADY RECORDED")
        );
        assert_eq!(fs::read(&path).expect("restored ledger"), original);
    }
    crate::knobs::clear_all();
}

#[test]
fn monthly_audit_missing_execution_or_prior_context_refuses_before_an_attempt() {
    for (month, timeframe) in [(5, Timeframe::MINUTE_1), (4, Timeframe::DAY_1)] {
        let fixture = Fixture::warmed();
        fs::remove_file(fixture.path(month, timeframe)).expect("remove owned required context");
        let refusal = fixture
            .audit("5min")
            .expect_err("required context is absent");
        assert!(!refusal.is_empty());
        if timeframe == Timeframe::MINUTE_1 {
            assert!(refusal.contains("1min execution series"), "{refusal}");
            assert!(refusal.contains("no coarse fallback"), "{refusal}");
        }
        assert!(fixture.audit_range("5min", (2025, 5)).is_err());
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no attempt ledger"),
            None
        );
    }
}

#[test]
fn range_audits_record_exact_requested_and_found_months_without_inventing_missing_bars() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (rung, end) in [
        ("1min", (2025, 5)),
        ("5min", (2025, 5)),
        ("5min", (2025, 6)),
    ] {
        let fixture = Fixture::warmed();
        if end.1 == 6 {
            let refusal = fixture
                .audit_range(rung, end)
                .expect_err("required June context is absent");
            assert!(refusal.contains("1day reference stream is incomplete: missing 2025-06"));
            assert!(!crate::results::Results::path(&fixture.root).exists());
            assert_eq!(
                crate::sweep_evidence::latest(&fixture.root, 1_048_576)
                    .expect("no premature attempt"),
                None
            );
            // Supply the mandatory reference and execution authority while
            // leaving June's five-minute signal file absent.
            let context = generated_session(6, 2);
            assert!(!context.is_empty());
            fixture.write(6, Timeframe::MINUTE_1, &context);
            fixture.write(6, Timeframe::DAY_1, &context[..1]);
        }
        let report = fixture
            .audit_range(rung, end)
            .expect("actual stored range audit");
        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        assert!(!report.contains(crate::NOT_RECORDED), "{report}");
        assert!(!report.contains("NOTHING MEASURED"), "{report}");
        assert_eq!(
            report.contains("MONTHS MISSING FROM THIS SPAN (1): 2025-06"),
            end.1 == 6
        );
        let mut ledger =
            crate::results::Results::open_read(&fixture.root).expect("cold range ledger");
        assert_eq!(ledger.len().expect("one parent"), 1);
        let row = ledger.read(0).expect("recorded range");
        assert_eq!((row.from_year, row.from_month), (2025, 5));
        assert_eq!((row.to_year, row.to_month), end);
        assert_eq!(
            (row.months_asked, row.months_found),
            (u32::from(end.1) - 4, 1)
        );
        assert_eq!(crate::results::read_field(&row.timeframe), rung);
        assert!(row.bars > 0);
        assert_eq!((row.trades, row.combinations, row.halted), (0, 0, 0));
        drop(ledger);
        let first = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read range attempt")
            .expect("range evidence");
        assert_eq!(first.identity, row.identity);
        assert_eq!(
            first.completion,
            crate::sweep_evidence::Completion::Completed
        );
        let path = crate::results::Results::path(&fixture.root);
        let saved = fs::read(&path).expect("saved parent bytes");
        let retry = fixture.audit_range(rung, end).expect("range retry");
        assert!(
            retry.contains("RESULT ALREADY RECORDED AND VERIFIED"),
            "{retry}"
        );
        assert_eq!(fs::read(path).expect("retry parent bytes"), saved);
        let second = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("read retry")
            .expect("range retry evidence");
        assert!(second.attempt > first.attempt);
        assert_eq!(second.identity, first.identity);
        assert_eq!(
            second.completion,
            crate::sweep_evidence::Completion::Completed
        );
    }
    crate::knobs::clear_all();
}

#[test]
fn native_and_coarse_use_exact_audited_rows_and_the_same_calendar_converters() {
    let fixture = Fixture::new();
    for (rung, guards, records) in [("1min", 4, 752), ("5min", 5, 827)] {
        let input = Inputs::load(fixture.request(rung)).expect("strict inputs");
        assert_eq!(input.guards.len(), guards);
        assert_eq!(input.records, records);
        let ordinary = stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", rung, 2025, 5)
            .expect("ordinary decoded fixture");
        assert_eq!(input.data.loaded.bars, ordinary.bars);
        let daily = stored::load_daily_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 5)),
            &ordinary.bars,
        )
        .expect("same daily converter");
        let minute = stored::load_exact_minute_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            ((2025, 5), (2025, 5)),
            &ordinary.bars,
        )
        .expect("same minute converter");
        assert_eq!(input.data.daily.bars, daily.bars);
        assert_eq!(input.data.exact_minute.bars, minute.bars);
        assert_eq!(input.data.execution_bars.is_some(), rung == "5min");
        assert_ne!(input.bind_digest([1; 32]), [1; 32]);
        assert_ne!(input.bind_digest([1; 32]), input.bind_digest([2; 32]));
        assert_eq!(input.roles[1], input.roles[5]);
        assert_eq!(input.roles[0] == input.roles[1], rung == "1min");
        assert!(input.note().contains("six-role input binding"));
        input.require_current().expect("all live guards");
    }
}

#[test]
fn every_context_source_and_the_saved_role_binding_remain_required_after_loading() {
    for (month, timeframe) in [
        (5, Timeframe::MINUTE_5),
        (5, Timeframe::MINUTE_1),
        (4, Timeframe::MINUTE_1),
        (4, Timeframe::DAY_1),
        (5, Timeframe::DAY_1),
    ] {
        let fixture = Fixture::new();
        let input = Inputs::load(fixture.request("5min")).expect("strict inputs");
        let path = fixture.path(month, timeframe);
        let bytes = fs::read(&path).expect("exact source");
        fs::remove_file(&path).expect("replace scratch source");
        fs::write(&path, bytes).expect("same bytes new inode");
        assert!(input.require_current().is_err());
    }
    let fixture = Fixture::new();
    let input = Inputs::load(fixture.request("1min")).expect("strict inputs");
    let path = fixture.root.join("audited-inputs-v1").join(format!(
        "{}.bin",
        crate::identity_hex(&input.binding_identity)
    ));
    let mut bytes = fs::read(&path).expect("binding");
    assert_eq!(bytes.len(), 512);
    assert_eq!(&bytes[..8], b"BRHIN001");
    bytes[24] ^= 1;
    fs::write(path, bytes).expect("corrupt scratch relationship");
    assert!(input.require_current().is_err());
}

#[test]
fn strict_input_caps_and_missing_prior_context_refuse_without_fallback() {
    let fixture = Fixture::new();
    for cap in [0, 1, 374, 751] {
        let mut request = fixture.request("1min");
        request.max_records = cap;
        assert!(Inputs::load(request).is_err());
    }
    let mut request = fixture.request("1min");
    request.max_records = 752;
    Inputs::load(request).expect("exact raw-source cap");
    let mut request = fixture.request("1min");
    request.max_bytes = 1;
    assert!(Inputs::load(request).is_err());
    fs::remove_file(fixture.path(4, Timeframe::DAY_1)).expect("remove required prior daily source");
    assert!(Inputs::load(fixture.request("1min")).is_err());
}

/// Every directory under `root`, and every file's bytes.
fn tree(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("readable fixture directory") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                out.insert(path.clone(), None);
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("readable fixture file");
                out.insert(path, Some(bytes));
            }
        }
    }
    out
}

/// What each recorded verb must answer with a usable budget in its environment.
fn budgeted_recorded_runs_refuse(root: &std::path::Path) -> Result<(), String> {
    let named = crate::SCREEN_BUDGET_NOT_RECORDABLE;
    assert_eq!(crate::recorded_budget_refusal(), Err(named.to_owned()));
    // THE THREE STORED KERNELS: refused as admission errors, before the source.
    let audit = crate::audit_stored_kernel(crate::StoredSweepRequest {
        root: root.to_path_buf(),
        vendor: Vendor::Zerodha,
        underlying: "NIFTY",
        rung: "5min",
        year: 2025,
        month: 5,
        min_hits: u64::MAX,
        commit: "generated-stored-audit-fixture",
    });
    assert_eq!(audit, Err(named.to_owned()));
    let range = crate::audit_range_kernel(crate::StoredRangeAuditRequest {
        root: root.to_path_buf(),
        vendor: Vendor::Zerodha,
        underlying: "NIFTY",
        rung: "5min",
        from: (2025, 5),
        to: (2025, 5),
        min_hits: u64::MAX,
        attempt: Some(7),
        commit: "generated-stored-range-audit-fixture",
    });
    assert_eq!(range, Err(named.to_owned()));
    let screen = crate::screen_range_kernel(crate::StoredScreenRequest {
        root: root.to_path_buf(),
        vendor: Vendor::Zerodha,
        underlying: "NIFTY",
        rung: "5min",
        span: ((2025, 5), (2025, 5)),
        support_ppm: 20_000,
        policy: crate::Policy {
            rules: crate::Rules::BASELINE,
            lens: runner::rank::Lens::Detectability,
            validate: false,
        },
        attempt: Some(13),
        commit: "generated-stored-screen-fixture",
    });
    assert_eq!(screen, Err(named.to_owned()));
    // A RANGE RUNG, whose support derivation would write before `audit_range`.
    // `BRUTEX_STORE` names this fixture, so without the refusal it would.
    let rung = crate::one_rung("zerodha", "NIFTY", "5min", (2025, 5), (2025, 5), None, None);
    assert_eq!(rung.outcome.err().as_deref(), Some(named));
    let table = crate::range_over("zerodha", "NIFTY", &["5min"], (2025, 5), (2025, 5), None);
    assert!(table.starts_with("refused: "), "{table}");
    assert!(table.contains(named), "{table}");
    budgeted_audit_transaction_refuses_only_a_recording(root)
}

/// The shared audit transaction under a usable budget: refused with a recording
/// target, not refused without one, and the generated-bar verb still runs.
fn budgeted_audit_transaction_refuses_only_a_recording(
    root: &std::path::Path,
) -> Result<(), String> {
    let named = crate::SCREEN_BUDGET_NOT_RECORDABLE;
    let key = stored::swept_index("NIFTY")?;
    let id = crate::identity(&runner::identity::Run {
        mask: vocab::ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &key,
        timeframe: "5min",
        params: runner::identity::Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: [7; 32],
        commit: "generated-budget-fixture",
        feed: "zerodha",
    });
    let options = |recording| crate::AuditOptions {
        prepared_column: None,
        replay: None,
        execution: None,
        native_minute_execution: true,
        recording,
        rules: crate::Rules::BASELINE,
        lens: runner::rank::Lens::Detectability,
        ceiling: Some(64),
        validate: false,
        cost: runner::audit::CostScope::IndexSpot,
    };
    let recorded = crate::audit_bars(
        &Err("EVALUATOR_WAS_REACHED"),
        Vec::new(),
        "fixture",
        1,
        Some(&id),
        options(Some(crate::Recording {
            root,
            feed: "zerodha",
            underlying: "NIFTY",
            timeframe: "5min",
            from: (2025, 5),
            to: (2025, 5),
            attempt: None,
            months_asked: 1,
            months_found: 1,
        })),
    );
    assert_eq!(recorded, format!("refused: {named}\n"));
    // AN UNRECORDED AUDIT IS NOT REFUSED: it passes the guard and reaches its
    // evaluator, and the generated-bar verb still runs under the budget.
    let unrecorded = crate::audit_bars(
        &Err("EVALUATOR_WAS_REACHED"),
        Vec::new(),
        "fixture",
        1,
        None,
        options(None),
    );
    assert!(unrecorded.contains("EVALUATOR_WAS_REACHED"), "{unrecorded}");
    let generated = crate::audit_run(1, u64::MAX);
    assert!(generated.starts_with(crate::PROVENANCE), "{generated}");
    assert!(!generated.contains(named), "{generated}");
    assert!(!generated.contains("KNOB REFUSED"), "{generated}");
    Ok(())
}

/// Every run that RECORDS refuses a usable screen budget by name before it reads
/// its source or writes anything: the three stored kernels, a range rung and
/// the shared audit transaction. The unrecorded generated-bar audit still runs,
/// and the same range audit with a stated screen cap records. D-0685.
///
/// The budget is stated in a CHILD process's environment, not in the process
/// knob store, so no concurrently running recorded test can observe it.
///
/// BEFORE IT READS, NOT ONLY BEFORE IT WRITES. The child's store was whole, so
/// a kernel that read its months and only then refused the budget answered
/// with the same sentence and wrote nothing, and passed (found by a review,
/// D-0696). Every month file of the child's store is now bytes no reader
/// accepts, so a kernel that reads before it refuses answers with the store's
/// refusal instead, and the child's exact comparison with the budget's
/// sentence fails.
#[test]
fn recorded_runs_refuse_a_screen_budget_before_reading_or_writing()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_TEST_RECORDED_SCREEN_BUDGET";
    if std::env::var_os(CHILD).is_some() {
        return Ok(budgeted_recorded_runs_refuse(&crate::store_root()?)?);
    }
    let fixture = Fixture::warmed();
    let unreadable = Fixture::warmed();
    let mut damaged = 0_usize;
    for (path, bytes) in tree(&unreadable.root) {
        if bytes.is_some() && path.extension().is_some_and(|ext| ext == "bin") {
            fs::write(&path, b"NOT A BAR FILE: any reader refuses these bytes")?;
            damaged += 1;
        }
    }
    assert!(damaged >= 5, "premise: every month file was damaged");
    assert!(
        stored::load(&unreadable.root, Vendor::Zerodha, "NIFTY", "5min", 2025, 5).is_err(),
        "premise: a read of the child's store is refused"
    );
    let before = tree(&unreadable.root);
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "audited_stored::tests::recorded_runs_refuse_a_screen_budget_before_reading_or_writing",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "budgeted")
        .env("BRUTEX_STORE", &unreadable.root)
        .env("BRUTEX_SCREEN_BUDGET_MS", "5000")
        .output()?;
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    assert_eq!(
        tree(&unreadable.root),
        before,
        "a refused budget wrote under the store"
    );

    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_SCREEN_CAP", "7");
    let capped = fixture.audit_range("5min", (2025, 5));
    crate::knobs::clear_all();
    let capped = capped.expect("a stated screen cap is recordable");
    assert!(capped.contains("RESULT RECORDED"), "{capped}");
    assert!(
        !capped.contains(crate::SCREEN_BUDGET_NOT_RECORDABLE),
        "{capped}"
    );
    Ok(())
}

/// The index header's opening words, as every index audit prints them.
const INDEX_HEADER: &str = "\nAUDIT\n  INDEX SPOT run. There is no brokerage";
/// The cash-equity header's opening line. D-0681.
const EQUITY_HEADER: &str =
    "\nAUDIT\n  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.\n";

/// Generated sessions whose prices MOVE, filed under any swept instrument.
///
/// [`Fixture`] writes one constant bar shape under NIFTY. That is right for the
/// provenance tests above and can never select a trade, and the audit's charge
/// statement is written only on a page that ranks and trades. These fixtures
/// carry `runner::synthetic`'s drifting, wobbling bars under the instrument a
/// test names -- a share or an index -- so a kernel's header can be read off
/// its real output.
struct Traded {
    root: PathBuf,
    underlying: &'static str,
}

impl Traded {
    fn new(underlying: &'static str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-audited-traded-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("scratch");
        let fixture = Self { root, underlying };
        for (index, (month, day)) in
            (0_i64..).zip([(4, 30), (5, 2), (5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])
        {
            let rows = moving_session(month, day, index);
            assert!(
                !rows.is_empty(),
                "2025-{month:02}-{day:02} must be an open session"
            );
            fixture.write(month, Timeframe::MINUTE_1, &rows);
            fixture.write(month, Timeframe::DAY_1, &[aggregate(&rows)]);
            if month == 5 {
                fixture.write(
                    month,
                    Timeframe::MINUTE_5,
                    &rows.chunks(5).map(aggregate).collect::<Vec<_>>(),
                );
            }
        }
        fixture
    }

    fn write(&self, month: u8, timeframe: Timeframe, rows: &[Bar]) {
        let key = stored::swept_index(self.underlying).expect("a swept fixture key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2025, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path");
        let hash = brutex_core::universe::fnv1a(key.underlying.as_str()).to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = BarFile::open_or_create(&self.root, path, symbol).expect("writer");
        file.append(rows).expect("generated fixture rows");
    }

    fn audit(&self, rung: &str, min_hits: u64) -> Result<String, String> {
        crate::audit_stored_kernel(crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.underlying,
            rung,
            year: 2025,
            month: 5,
            min_hits,
            commit: "generated-traded-audit-fixture",
        })
    }

    fn range(&self, rung: &str, min_hits: u64) -> Result<String, String> {
        crate::audit_range_kernel(crate::StoredRangeAuditRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.underlying,
            rung,
            from: (2025, 5),
            to: (2025, 5),
            min_hits,
            attempt: None,
            commit: "generated-traded-range-fixture",
        })
    }

    fn screen(&self, rung: &str, support_ppm: u64) -> Result<String, String> {
        crate::screen_range_kernel(crate::StoredScreenRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.underlying,
            rung,
            span: ((2025, 5), (2025, 5)),
            support_ppm,
            policy: crate::Policy {
                rules: crate::Rules::BASELINE,
                lens: runner::rank::Lens::Detectability,
                validate: false,
            },
            attempt: None,
            commit: "generated-traded-screen-fixture",
        })
    }

    /// The `sweep-stored` verb's own door over May 2025: `sweep_stored_kernel`,
    /// which withholds holed sessions and hands `stored_month_kernel` no
    /// checksum guard.
    fn sweep_stored(&self, rung: &str, min_hits: u64) -> Result<String, String> {
        crate::sweep_stored_kernel(crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: self.underlying,
            rung,
            year: 2025,
            month: 5,
            min_hits,
            commit: "generated-traded-sweep-fixture",
        })
    }

    /// The `sweep-audited-stored` verb's door over May 2025: the two calls
    /// `sweep_audited_stored` makes, `Inputs::load` and then
    /// `stored_month_kernel` with that checksum guard.
    fn sweep(&self, rung: &str, min_hits: u64) -> Result<String, String> {
        let input = Inputs::load(Request {
            store_root: &self.root,
            vendor: Vendor::Zerodha,
            underlying: self.underlying,
            rung,
            year: 2025,
            month: 5,
            receipt_root: &self.root,
            max_bytes: 16_777_216,
            max_records: 100_000,
        })?;
        crate::stored_month_kernel(
            crate::StoredSweepRequest {
                root: self.root.clone(),
                vendor: Vendor::Zerodha,
                underlying: self.underlying,
                rung,
                year: 2025,
                month: 5,
                min_hits,
                commit: "generated-traded-sweep-fixture",
            },
            input.data(),
            Some(&input),
        )
    }
}

impl Drop for Traded {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// One coarser bar from consecutive finer ones: first open and stamp, extreme
/// high and low, last close, summed volume -- what a vendor's own rung prints.
fn aggregate(rows: &[Bar]) -> Bar {
    let first = rows.first().expect("a nonempty generated aggregation");
    Bar {
        ts_micros: first.ts_micros,
        open: first.open,
        high: rows.iter().map(|row| row.high).max().expect("high"),
        low: rows.iter().map(|row| row.low).min().expect("low"),
        close: rows.last().expect("close").close,
        volume: rows.iter().map(|row| row.volume).sum(),
        open_interest: i64::MIN,
    }
}

/// One open session of `runner::synthetic` bars on its real IST minute grid.
fn moving_session(month: u8, date: u8, index: i64) -> Vec<Bar> {
    let civil = pull::session::Day::new(2025, month, date).expect("date");
    let day = i64::from(civil.days_from_epoch());
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .enumerate()
        .map(|(minute_of_session, minute)| {
            let bar = runner::synthetic::bar(index, minute_of_session);
            Bar {
                ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                    - indicators::IST_OFFSET_MICROS,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                open_interest: i64::MIN,
            }
        })
        .collect()
}

/// Bounded to what a header needs: no validation stack and the smallest exit
/// grid a strict run admits. Neither changes which header is written. The
/// caller holds `knobs::serially` and clears them after.
fn bounded_header_knobs() {
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_VALIDATE", "0");
    crate::knobs::set("BRUTEX_GRID_RUNGS", "2");
}

/// One stored kernel over the same generated sessions filed under a share and
/// under an index: each page carries its own header and never the other's, the
/// share's FINDINGS carry the ranking's own gross statement, and the rung note
/// lifts the whole AUDIT header.
fn kernel_heads_by_kind(verb: &str, run: impl Fn(&Traded) -> Result<String, String>) {
    for (underlying, header, foreign) in [
        ("RELIANCE", EQUITY_HEADER, INDEX_HEADER),
        ("NIFTY", INDEX_HEADER, EQUITY_HEADER),
    ] {
        let fixture = Traded::new(underlying);
        let equity = underlying == "RELIANCE";
        let report = run(&fixture)
            .map_err(|why| format!("{underlying} {verb}: {why}"))
            .expect("a generated traded kernel run completes");
        assert!(
            !crate::carries_refusal(&report),
            "{underlying} {verb}:\n{report}"
        );
        assert!(
            report.contains(header),
            "{underlying} {verb} must carry its own header:\n{report}"
        );
        assert!(
            !report.contains(foreign),
            "{underlying} {verb} carries the other instrument's header:\n{report}"
        );
        let findings = crate::section_note(&report, "FINDINGS").expect("a ranking");
        assert_eq!(
            findings.contains(crate::EQUITY_RANKING_GROSS.trim_end()),
            equity,
            "{underlying} {verb}: the ranking's charge statement:\n{findings}"
        );
        let lifted = crate::validation_note(&report).expect("the rung note");
        assert!(lifted.contains(header.trim_start_matches('\n')), "{lifted}");
        if equity {
            for paragraph in [
                "COST-EXCLUDED RESEARCH, NOT A NET RESULT",
                "No equity result carries Selection V6",
                "GROSS OF THE SPREAD",
            ] {
                assert!(
                    lifted.contains(paragraph),
                    "{verb}: the lift must keep every paragraph of the header, \
                     missing {paragraph:?}:\n{lifted}"
                );
            }
        }
    }
}

/// **`audit-stored` heads a share GROSS OF EVERY CHARGE and an index exactly
/// as it always has, from the key it loaded.** D-0681.
///
/// Every stored-kernel fixture was NIFTY, so replacing any kernel's
/// `stored::audit_cost_scope(&key)?` with a constant `IndexSpot` passed the
/// whole suite -- and a mutation tool does not mutate call-site arguments, so
/// it could not see that either. This and the two tests below run one kernel
/// each on a generated share and a generated index, so a regression names the
/// kernel it is in.
#[test]
fn the_stored_month_audit_heads_a_share_gross_and_an_index_as_before() {
    let _knobs = crate::knobs::serially();
    bounded_header_knobs();
    kernel_heads_by_kind("audit_stored_kernel", |fixture| fixture.audit("1min", 700));
    crate::knobs::clear_all();
}

/// **A stored range audit heads a share GROSS OF EVERY CHARGE.** D-0681.
#[test]
fn the_stored_range_audit_heads_a_share_gross_and_an_index_as_before() {
    let _knobs = crate::knobs::serially();
    bounded_header_knobs();
    kernel_heads_by_kind("audit_range_kernel", |fixture| fixture.range("1min", 700));
    crate::knobs::clear_all();
}

/// **A stored screen heads a share GROSS OF EVERY CHARGE.** D-0681.
#[test]
fn the_stored_screen_heads_a_share_gross_and_an_index_as_before() {
    let _knobs = crate::knobs::serially();
    bounded_header_knobs();
    kernel_heads_by_kind("screen_range_kernel", |fixture| {
        fixture.screen("1min", 311_111)
    });
    crate::knobs::clear_all();
}

/// **`cli top` names a recorded share as a share, gross of every charge, and
/// an index exactly as before.**
///
/// The legend was one literal: a RELIANCE run read "`mean` is the average
/// forward move ... per ONE unit of the index, gross of the statutory charge
/// stack" -- the wrong instrument, and a charge statement naming no charge.
#[test]
fn top_names_a_recorded_share_as_a_share_and_an_index_as_before() {
    let _knobs = crate::knobs::serially();
    bounded_header_knobs();
    for underlying in ["RELIANCE", "NIFTY"] {
        let fixture = Traded::new(underlying);
        let equity = underlying == "RELIANCE";
        let recorded = fixture
            .audit("1min", 700)
            .map_err(|why| format!("{underlying}: {why}"))
            .expect("a recorded generated audit");
        assert!(recorded.contains("RESULT RECORDED"), "premise:\n{recorded}");
        let top = crate::top_at(&fixture.root, None, Some(underlying));
        assert!(top.contains("TOP COMBINATIONS"), "{top}");
        assert_eq!(
            top.contains(&format!("per {}\n", crate::SHARE_MEAN_LEGEND)),
            equity,
            "{underlying}: `cli top` must name a share as a share:\n{top}"
        );
        assert_eq!(
            top.contains("per ONE unit of the index, gross of the statutory charge stack."),
            !equity,
            "{underlying}: an index keeps its legend byte for byte:\n{top}"
        );
        if equity {
            assert!(!top.contains("unit of the index"), "{top}");
        }
    }
    crate::knobs::clear_all();
}

/// **`sweep-stored` on a share says its ranking is gross of every charge.**
///
/// The verb renders FINDINGS and stops -- it has no AUDIT block, so D-0681's
/// header never reached it, and a ranked share table (for example `mean 45
/// paisa, t 103.04, clears`) carried no charge statement at all.
///
/// An index's sweep carries none of the equity text: not the label, not the
/// banner's gross paragraph, not the corporate-actions sentence, anywhere on
/// its page. That is what is asserted of NIFTY. It is not compared byte for
/// byte with an index page from before D-0694, and this doc said "An index's
/// sweep is unchanged" when it asserted less than that (D-0696).
///
/// BOTH DOORS. This drove only `sweep-audited-stored`'s, through
/// `Traded::sweep`, while this doc and AF-31, AF-33 and AF-38 named
/// `sweep-stored`, whose door no test ranked on (found by a review, D-0696).
/// Each page is now asserted from `sweep-stored`'s door, `Traded::sweep_stored`,
/// and from the audited one.
#[test]
fn a_stored_sweep_of_a_share_says_its_ranking_is_gross_of_every_charge() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (underlying, door) in [
        ("RELIANCE", "sweep-stored"),
        ("NIFTY", "sweep-stored"),
        ("RELIANCE", "sweep-audited-stored"),
        ("NIFTY", "sweep-audited-stored"),
    ] {
        let fixture = Traded::new(underlying);
        let page = if door == "sweep-stored" {
            fixture.sweep_stored("1min", 700)
        } else {
            fixture.sweep("1min", 700)
        }
        .map_err(|why| format!("{underlying} through {door}: {why}"))
        .expect("a generated traded sweep completes");
        assert_eq!(
            page.contains("STRICT HISTORICAL INPUT CHECKSUMS V1"),
            door == "sweep-audited-stored",
            "premise: the page is {door}'s:\n{page}"
        );
        let findings = crate::section_note(&page, "FINDINGS").expect("the ranking");
        assert!(
            findings.contains("  rank "),
            "premise: a ranked table:\n{findings}"
        );
        assert_eq!(
            findings.contains(crate::EQUITY_RANKING_GROSS.trim_end()),
            underlying == "RELIANCE",
            "{underlying} through {door}:\n{page}"
        );
        // FINDINGS is where the ranking is labelled, and not the page's only
        // charge statement: since D-0694 a share's page opens with the banner
        // note too. D-0696 said "FINDINGS only".
        let banner = format!(
            "{}{}",
            crate::STORED_PROVENANCE,
            runner::audit::CostScope::CashEquity.report_note()
        );
        assert_eq!(
            page.starts_with(&banner),
            underlying == "RELIANCE",
            "{underlying} through {door}: the banner note heads a share's page and no index's:\n{page}"
        );
        for equity_text in [
            crate::EQUITY_RANKING_GROSS.trim_end(),
            runner::audit::CASH_EQUITY_GROSS,
            runner::audit::CORPORATE_ACTIONS_UNCHECKED,
        ] {
            assert_eq!(
                page.contains(equity_text),
                underlying == "RELIANCE",
                "{underlying} through {door}: a share's page carries {equity_text:?} and an \
                 index's carries it nowhere:\n{page}"
            );
        }
        assert!(!crate::carries_refusal(&page), "{page}");
    }
}

/// **A ranked stored sweep of a share says its corporate actions are
/// unchecked inside FINDINGS, right after the gross label, and an index's
/// sweep never does.** D-0694.
///
/// `stored_month_kernel` appends the FINDINGS label itself rather than through
/// `ranked_opening`, so the audit-page proof in `equity_statement_tests` does
/// not reach this call site. A lifted FINDINGS block carries no banner, so the
/// sentence must be inside the block to travel with the ranking.
///
/// Asserted from both doors that reach `stored_month_kernel`: `sweep-stored`'s
/// and `sweep-audited-stored`'s. Only the second was driven until a review
/// found it (D-0696).
#[test]
fn a_ranked_stored_sweep_of_a_share_states_corporate_actions_inside_its_findings() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    for (underlying, door) in [
        ("RELIANCE", "sweep-stored"),
        ("NIFTY", "sweep-stored"),
        ("RELIANCE", "sweep-audited-stored"),
        ("NIFTY", "sweep-audited-stored"),
    ] {
        let fixture = Traded::new(underlying);
        let equity = underlying == "RELIANCE";
        let page = if door == "sweep-stored" {
            fixture.sweep_stored("1min", 700)
        } else {
            fixture.sweep("1min", 700)
        }
        .map_err(|why| format!("{underlying} through {door}: {why}"))
        .expect("a generated traded sweep completes");
        let findings = crate::section_note(&page, "FINDINGS").expect("the ranking");
        assert!(
            findings.contains("  rank "),
            "premise: a ranked table:\n{findings}"
        );
        assert_eq!(
            findings.contains(crate::equity_ranking_statement().trim_end()),
            equity,
            "{underlying} through {door}: the gross label and the sentence, together:\n{page}"
        );
        assert_eq!(
            findings.contains(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
            equity,
            "{underlying} through {door}:\n{page}"
        );
        assert!(!crate::carries_refusal(&page), "{page}");
    }
    crate::knobs::clear_all();
}

/// **An extinct share audit prints no ranking and so no charge statement.**
///
/// `min_hits = u64::MAX` is the extinction path: nothing is kept, nothing is
/// ranked, nothing is traded, and neither header is written. F1 adds no
/// arithmetic, so the only edge is that the label must not qualify a table
/// that does not exist.
///
/// D-0694 put one statement ABOVE the report: every stored report over a
/// stock opens with the provenance banner, then gross of every charge, then
/// corporate actions unchecked, because the support counts that went extinct
/// were counted on a share's unadjusted bars. So the page is split at that
/// banner. The banner must be exactly the stock's, and below it -- where the
/// ranking and the audit would be -- neither header, no gross label and no
/// corporate-action sentence may appear.
#[test]
fn an_extinct_share_audit_prints_no_ranking_and_no_charge_statement() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Traded::new("RELIANCE");
    let banner = format!(
        "{}{}",
        crate::STORED_PROVENANCE,
        runner::audit::CostScope::CashEquity.report_note()
    );
    for (verb, report) in [
        ("audit_stored_kernel", fixture.audit("1min", u64::MAX)),
        ("audit_range_kernel", fixture.range("1min", u64::MAX)),
    ] {
        let report = report
            .map_err(|why| format!("{verb}: {why}"))
            .expect("an extinct generated audit completes");
        assert!(
            report.contains("nothing kept — the sweep produced no combination"),
            "{verb}:\n{report}"
        );
        assert!(
            report.contains("This is extinction, not a failure."),
            "{report}"
        );
        let below = report.strip_prefix(banner.as_str());
        assert!(below.is_some(), "{verb}: a share's banner leads:\n{report}");
        let below = below.expect("asserted above");
        for absent in [
            EQUITY_HEADER,
            INDEX_HEADER,
            "GROSS OF EVERY CHARGE",
            runner::audit::CORPORATE_ACTIONS_UNCHECKED,
        ] {
            assert!(
                !below.contains(absent),
                "{verb} printed {absent:?} below its banner:\n{report}"
            );
        }
    }
}

/// **The ordinary stored sweep refuses an empty month as empty, and never
/// blames minute gaps it did not measure.** D-0696.
///
/// `stored::load` returns a month whose file exists and holds no record as a
/// month with no bars. The door then refused it with "every signal session
/// has a minute gap; no sweepable bars remain", though no gap was measured and
/// nothing was withheld (found by a review). An interrupted ingest leaves
/// exactly that file. Here a `5min` April left empty beside damaged execution
/// minutes, and a `1min` June left empty, are each refused by the reason that
/// holds, before execution minutes are loaded or an attempt is opened.
#[test]
fn the_ordinary_stored_sweep_refuses_an_empty_month_as_empty() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::new();
    drop(fixture.open(4, Timeframe::MINUTE_5));
    drop(fixture.open(6, Timeframe::MINUTE_1));
    let minutes = fixture
        .root
        .join("bars/zerodha/NSE/INDEX/NIFTY/1min/2025-04.bin");
    std::fs::write(&minutes, b"unreadable execution minutes").expect("damage April's minutes");
    assert!(
        stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", "1min", 2025, 4).is_err(),
        "premise: loading execution minutes before the empty check would refuse differently"
    );
    for (rung, month) in [("5min", 4), ("1min", 6)] {
        let loaded = stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", rung, 2025, month)
            .expect("premise: the month's file exists");
        assert!(loaded.bars.is_empty(), "premise: it holds no bar");
        let mut request = fixture.month_request(rung);
        request.month = month;
        let refusal = crate::sweep_stored_kernel(request).expect_err("an empty month");
        assert_eq!(
            refusal,
            format!(
                "the {rung} month 2025-{month:02} is stored and holds no bar. Nothing was swept"
            )
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no empty attempt"),
            None
        );
    }
    crate::knobs::clear_all();
}

/// A NIFTY future: a key `swept_index` can never hand a kernel.
fn a_contract() -> brutex_core::instrument::InstrumentKey {
    use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, Segment};
    InstrumentKey {
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("valid"),
        kind: Kind::Future {
            expiry: Expiry::new(2025, 5, 29).expect("a real expiry"),
        },
    }
}

/// One stored kernel handed a contract through the scope seam: refused with
/// exactly the scope's own sentence, and no result recorded. Once the guard
/// drops, the same kernel on the same store is whole again.
fn contract_is_refused_by(verb: &str, run: impl Fn(&Fixture) -> Result<String, String>) {
    let future = a_contract();
    let named = stored::audit_cost_scope(&future).expect_err("a contract has no header");
    let fixture = Fixture::warmed();
    let refused = {
        let _contract = stored::CostScopeFault::install(future);
        run(&fixture)
    };
    assert_eq!(refused, Err(named.clone()), "{verb}");
    assert!(
        !crate::results::Results::path(&fixture.root).exists(),
        "{verb}: a refused contract recorded a result"
    );
    let whole = run(&fixture)
        .map_err(|why| format!("{verb}: {why}"))
        .expect("the seam is gone with its guard");
    assert!(whole.contains("RESULT RECORDED"), "{verb}:\n{whole}");
    assert!(!whole.contains(&named), "{verb}:\n{whole}");
}

/// **A contract reaching `audit-stored` is refused by name before a result is
/// recorded.**
///
/// `swept_index` refuses every contract first, so the `?` each kernel puts on
/// `audit_cost_scope` could not run from any store: a kernel that swallowed
/// the refusal and borrowed a header would have passed. The seam hands the
/// scope decision a NIFTY future in place of the loaded key, on this thread
/// only. This and the two tests below cover one kernel each.
#[test]
fn a_contract_reaching_the_stored_month_audit_is_refused_before_it_is_recorded() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    contract_is_refused_by("audit_stored_kernel", |fixture| fixture.audit("5min"));
}

/// **A contract reaching a stored range audit is refused by name.**
#[test]
fn a_contract_reaching_the_stored_range_audit_is_refused_before_it_is_recorded() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    contract_is_refused_by("audit_range_kernel", |fixture| {
        fixture.audit_range("5min", (2025, 5))
    });
}

/// **A contract reaching a stored screen is refused by name.**
#[test]
fn a_contract_reaching_the_stored_screen_is_refused_before_it_is_recorded() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    contract_is_refused_by("screen_range_kernel", |fixture| {
        fixture.screen("5min", 900_000)
    });
}

/// **The ordinary stored sweep withholds a holed session and names it, as
/// `screen`, `audit-range`, `auto-stored` and `pool` do.** D-0694.
///
/// 2025-05-05 loses one interior minute. Before D-0694 this door kept the day:
/// on `1min` the bar after the hole was folded as the neighbour of the bar
/// before it, and on `5min` the exact-minute overlay refused the whole month.
/// Now the day is out of the swept sample, no bar of it reaches the column,
/// the report names it and the recorded bar count excludes it, and the source
/// file is untouched.
#[test]
fn the_ordinary_stored_sweep_withholds_and_names_a_holed_session() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    let holed = i64::from(
        pull::session::Day::new(2025, 5, 5)
            .expect("date")
            .days_from_epoch(),
    );
    // `swept` is what the column folded past warm-up: the ledger's `bars`
    // (AC-whp-law-2, D-1661). It pinned `retained`, the slice length, which
    // counts warming bars the sweep never folded.
    for (rung, retained, withheld, swept) in [
        ("1min", 2_625_u64, 374_u64, 1_500_u64),
        ("5min", 525, 75, 300),
    ] {
        let fixture = Fixture::warmed();
        fixture.omit_owned_minutes(&[5]);
        let source = fixture.path(5, Timeframe::MINUTE_1);
        let source_bytes = fs::read(&source).expect("owned incomplete minute source");

        let inputs = crate::stored_sweep_inputs(&fixture.month_request(rung)).expect("inputs");
        assert_eq!(inputs.loaded.bars.len() as u64, retained, "{rung}");
        assert!(
            inputs
                .loaded
                .bars
                .iter()
                .all(|bar| indicators::ist_day(bar.ts_micros) != holed),
            "{rung}: no bar of the holed session may reach the column"
        );
        let gaps = inputs
            .minute_gaps
            .as_ref()
            .expect("this door applies the minute-gap rule");
        assert_eq!(gaps.day_numbers(), &[holed]);
        assert_eq!(gaps.signal_bars(), withheld);
        // WHICH SERIES LOST THE DAY. This asserted zero minute bars on both
        // rungs, "the execution minutes stay whole", and on `1min` they do not:
        // there the signal bars ARE the execution bars. AF-19.
        match inputs.execution_bars.as_ref() {
            None => {
                assert_eq!(rung, "1min", "only the minute rung has no separate series");
                assert_eq!(
                    gaps.minute_bars(),
                    withheld,
                    "{rung}: the withheld bars were the execution minutes"
                );
            }
            Some(execution) => {
                assert_eq!(gaps.minute_bars(), 0, "{rung}: no minute bar was removed");
                assert!(
                    execution
                        .bars
                        .iter()
                        .any(|bar| indicators::ist_day(bar.ts_micros) == holed),
                    "{rung}: the separately loaded execution minutes keep the day"
                );
            }
        }

        let report = fixture.sweep(rung).expect("the holed month sweeps");
        assert!(
            report.contains(&format!(
                "MINUTE-GAP SESSIONS WITHHELD: {withheld} signal bar(s); IST dates: 2025-05-05. \
                 The sweep uses the remaining {retained} signal bars."
            )),
            "{report}"
        );
        assert!(report.contains(&format!("· {retained} bars ·")), "{report}");
        assert!(report.contains("RESULT RECORDED"), "{report}");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("sweep ledger");
        assert_eq!(ledger.len().expect("one parent"), 1);
        assert_eq!(ledger.read(0).expect("sweep parent").bars, swept, "{rung}");
        drop(ledger);
        assert_eq!(fs::read(&source).expect("unmodified source"), source_bytes);
    }
    crate::knobs::clear_all();
}

/// **A month with every session holed leaves nothing to sweep, and the
/// ordinary door refuses before it opens an attempt.** D-0694.
#[test]
fn the_ordinary_stored_sweep_refuses_when_every_session_is_withheld() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    fixture.omit_owned_minutes(&[2, 5, 6, 7, 8, 9, 12, 13]);
    for rung in ["1min", "5min"] {
        let refusal = fixture.sweep(rung).expect_err("no retained session");
        assert_eq!(
            refusal,
            "every signal session has a minute gap; no sweepable bars remain"
        );
        assert!(!crate::results::Results::path(&fixture.root).exists());
        assert_eq!(
            crate::sweep_evidence::latest(&fixture.root, 1_048_576).expect("no empty attempt"),
            None
        );
    }
    crate::knobs::clear_all();
}

/// **A month with no hole keeps its bars and every mask, and moves its
/// identity.** D-0694.
///
/// The minute-gap rule withholds nothing here, so the bars, both reference
/// contexts and the anchored column must be byte-identical to the loads this
/// door made before D-0694 (`CLAUDE.md` §3 rule 5). The rule is still a
/// different computation from no rule, so the recorded identity binds its
/// version and differs from the one the same month had before, while the
/// checksum-audited door, which applies no rule, keeps the ladder alone.
#[test]
fn a_gap_free_month_keeps_its_bars_and_masks_and_moves_its_identity() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let span = ((2025, 5), (2025, 5));
        // The loads this door made before D-0694, in the same order.
        let loaded =
            stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", rung, 2025, 5).expect("month");
        let daily =
            stored::load_daily_context(&fixture.root, Vendor::Zerodha, "NIFTY", span, &loaded.bars)
                .expect("daily context");
        let exact = stored::load_exact_minute_context(
            &fixture.root,
            Vendor::Zerodha,
            "NIFTY",
            span,
            &loaded.bars,
        )
        .expect("exact-minute context");

        let inputs = crate::stored_sweep_inputs(&fixture.month_request(rung)).expect("inputs");
        assert!(
            inputs
                .minute_gaps
                .as_ref()
                .is_some_and(crate::minute_gaps::GapExclusion::is_empty),
            "{rung}: a gap-free month withholds nothing"
        );
        assert_eq!(inputs.loaded.bars, loaded.bars, "{rung}: the same bars");
        // WHOLE CONTEXTS, EVERY FIELD. This compared the daily bars and
        // eligibility and the exact-minute bars, and left the references,
        // the month counts, the prior session and the calendar exclusion to
        // the data digest. AF-19.
        assert_eq!(inputs.daily, daily, "{rung}: the same daily context");
        assert_eq!(
            inputs.exact_minute, exact,
            "{rung}: the same exact-minute context"
        );
        let signal = stored::rung_length_micros(rung).expect("a swept rung");
        let availability = stored::vwap_availability(&loaded.key);
        let before =
            crate::stored_anchored_column(&loaded.bars, &daily, &exact, signal, availability)
                .expect("the column before D-0694");
        let after = crate::stored_anchored_column(
            &inputs.loaded.bars,
            &inputs.daily,
            &inputs.exact_minute,
            signal,
            availability,
        )
        .expect("the column after");
        assert_eq!(after, before, "{rung}: every mask identical");

        let report = fixture.sweep(rung).expect("the month sweeps");
        assert!(!report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("sweep ledger");
        let row = ledger.read(0).expect("sweep parent");
        drop(ledger);
        let execution = inputs
            .execution_bars
            .as_ref()
            .map_or(loaded.bars.as_slice(), |minute| minute.bars.as_slice());
        let digest = crate::stored_executed_digest(&loaded.bars, &exact, &daily, execution)
            .expect("the same data term");
        let ladder = crate::ladder_for(u64::MAX).expect("ladder");
        let id = |params: runner::identity::Params| {
            runner::identity::identity(&runner::identity::Run {
                mask: vocab::ConditionMask::default(),
                direction: runner::identity::Direction::Undirected,
                instrument: &loaded.key,
                timeframe: loaded.timeframe,
                params,
                data_digest: digest,
                commit: "generated-stored-sweep-fixture",
                feed: "zerodha",
            })
            .bytes()
        };
        let legacy = runner::identity::Params::of(ladder);
        let gapped = legacy.with_policy(&[u64::from(crate::minute_gaps::MINUTE_GAP_POLICY)]);
        assert_eq!(
            crate::stored_month_params(ladder, Some(&crate::minute_gaps::GapExclusion::none())),
            gapped
        );
        assert_eq!(
            crate::stored_month_params(ladder, None),
            legacy,
            "the checksum-audited door keeps the ladder alone"
        );
        assert_eq!(
            row.identity,
            id(gapped),
            "{rung}: the rule's version is bound"
        );
        assert_ne!(
            row.identity,
            id(legacy),
            "{rung}: the identity this month had before D-0694 must not be reused"
        );
    }
    crate::knobs::clear_all();
}

/// The identity a stored month kernel recorded for `inputs` under `params`,
/// rebuilt term by term from outside the kernel.
fn month_identity(
    inputs: &crate::StoredMonthInputs,
    data_digest: [u8; 32],
    params: runner::identity::Params,
) -> [u8; 32] {
    runner::identity::identity(&runner::identity::Run {
        mask: vocab::ConditionMask::default(),
        direction: runner::identity::Direction::Undirected,
        instrument: &inputs.loaded.key,
        timeframe: inputs.loaded.timeframe,
        params,
        data_digest,
        commit: "generated-stored-sweep-fixture",
        feed: "zerodha",
    })
    .bytes()
}

/// The data term a stored month kernel digests `inputs` into.
fn month_digest(inputs: &crate::StoredMonthInputs) -> [u8; 32] {
    let execution = inputs
        .execution_bars
        .as_ref()
        .map_or(inputs.loaded.bars.as_slice(), |minute| {
            minute.bars.as_slice()
        });
    crate::stored_executed_digest(
        &inputs.loaded.bars,
        &inputs.exact_minute,
        &inputs.daily,
        execution,
    )
    .expect("the data term")
}

/// **The ordinary door binds the minute-gap rule by its VALUE, and the
/// checksum-audited door records the ladder alone, on the row it
/// records.** AF-19.
///
/// Two holes the D-0694 tests left. The gap-free test builds its expected
/// identity from `MINUTE_GAP_POLICY` itself, so renumbering the constant
/// re-keyed every ordinary stored sweep recorded since D-0694 and failed
/// nothing (`CLAUDE.md` §3 rule 8). And the audited door's
/// `Params::of(ladder)` was checked on `stored_month_params`, never on the
/// identity the door writes, so a kernel call site binding the rule for both
/// doors passed. Here the value is a literal, and both doors run over one
/// gap-free month and each recorded row is rebuilt from outside the kernel.
#[test]
fn the_minute_gap_rule_is_bound_by_value_and_the_audited_door_records_the_ladder_alone() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    // Rule 1 (D-0694) withheld interior gaps only. D-1662 withholds every day
    // missing its demanded closing minute too, which is a different rule, so
    // it takes the next number and 1 keeps meaning what it always meant
    // (`CLAUDE.md` §3 rule 8). The literal still refuses a silent renumbering.
    assert_eq!(
        crate::minute_gaps::MINUTE_GAP_POLICY,
        2,
        "renumbering the rule re-keys every ordinary stored sweep recorded since D-1662"
    );
    for rung in ["1min", "5min"] {
        let fixture = Fixture::warmed();
        let ladder = crate::ladder_for(u64::MAX).expect("ladder");
        let legacy = runner::identity::Params::of(ladder);

        let ordinary = crate::stored_sweep_inputs(&fixture.month_request(rung)).expect("inputs");
        let first = fixture.sweep(rung).expect("the ordinary door sweeps");
        assert!(first.contains("RESULT RECORDED"), "{rung}:\n{first}");
        let audited = Inputs::load(fixture.request(rung)).expect("audited generated inputs");
        let second =
            crate::stored_month_kernel(fixture.month_request(rung), audited.data(), Some(&audited))
                .expect("the audited door sweeps");
        assert!(second.contains("RESULT RECORDED"), "{rung}:\n{second}");

        let mut ledger = crate::results::Results::open_read(&fixture.root).expect("ledger");
        assert_eq!(ledger.len().expect("count"), 2, "{rung}: one row per door");
        let (ordinary_row, audited_row) = (
            ledger.read(0).expect("the ordinary row"),
            ledger.read(1).expect("the audited row"),
        );
        drop(ledger);
        assert_eq!(
            ordinary_row.identity,
            month_identity(&ordinary, month_digest(&ordinary), legacy.with_policy(&[2])),
            "{rung}: the ordinary door binds minute-gap rule 2"
        );
        assert_ne!(
            ordinary_row.identity,
            month_identity(&ordinary, month_digest(&ordinary), legacy.with_policy(&[1])),
            "{rung}: rule 1's word must not name D-1662's census"
        );
        let bound = audited.bind_digest(month_digest(audited.data()));
        assert_eq!(
            audited_row.identity,
            month_identity(audited.data(), bound, legacy),
            "{rung}: the checksum-audited door records the ladder alone"
        );
        assert_ne!(
            audited_row.identity,
            month_identity(audited.data(), bound, legacy.with_policy(&[2])),
            "{rung}: the rule did not reach the audited door"
        );
    }
    crate::knobs::clear_all();
}

/// **A holed minute day the signal rung holds no bar of withholds nothing,
/// and the report does not name it.** AF-19.
///
/// The ordinary door printed its MINUTE-GAP line whenever the measured day
/// list was not empty, where `screen` prints only when a signal bar was
/// withheld. Here 2025-05-05 loses one minute and the `5min` file holds no
/// bar of that day, so nothing leaves the signal series: the line would have
/// read "0 signal bar(s)". The rule is still applied and still bound.
#[test]
fn a_holed_day_the_signal_rung_does_not_hold_withholds_nothing_and_is_not_named() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    let holed = i64::from(
        pull::session::Day::new(2025, 5, 5)
            .expect("date")
            .days_from_epoch(),
    );
    let fixture = Fixture::warmed();
    fixture.omit_owned_minutes(&[5]);
    let coarse = fixture.path(5, Timeframe::MINUTE_5);
    fs::remove_file(&coarse).expect("replace owned generated coarse file");
    fs::remove_file(coarse.with_extension("crc")).expect("replace its owned proof");
    for day in [2, 6, 7, 8, 9, 12, 13] {
        let rows = generated_session(5, day);
        fixture.write(
            5,
            Timeframe::MINUTE_5,
            &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
        );
    }

    let inputs = crate::stored_sweep_inputs(&fixture.month_request("5min")).expect("inputs");
    let gaps = inputs
        .minute_gaps
        .as_ref()
        .expect("this door applies the minute-gap rule");
    assert_eq!(
        gaps.day_numbers(),
        &[holed],
        "premise: the hole is measured"
    );
    assert_eq!(
        gaps.signal_bars(),
        0,
        "premise: no signal bar of it is held"
    );
    let report = fixture.sweep("5min").expect("the month sweeps");
    assert!(report.contains("RESULT RECORDED"), "{report}");
    assert!(!report.contains("MINUTE-GAP SESSIONS WITHHELD"), "{report}");
    crate::knobs::clear_all();
}

/// **Every stored report over a stock states corporate actions are
/// unchecked, beside its gross-of-every-charge statement, and no report over
/// an index does.** D-0694.
///
/// Drives each stored door end to end over the same generated month, once as
/// RELIANCE's cash series and once as NIFTY: the ordinary and the
/// checksum-audited sweep, the one-month audit, the range audit, the screen
/// and the threshold search. The audits say it in their charge header; the
/// others after the provenance banner.
#[test]
fn every_stored_report_over_a_stock_states_corporate_actions_are_unchecked() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "256");
    for (symbol, stock) in [("RELIANCE", true), ("NIFTY", false)] {
        let fixture = Fixture::warmed_for(symbol);
        let audited = Inputs::load(fixture.request("5min")).expect("audited generated inputs");
        let reports = [
            ("sweep-stored", fixture.sweep("5min")),
            (
                "sweep-audited-stored",
                crate::stored_month_kernel(
                    fixture.month_request("5min"),
                    audited.data(),
                    Some(&audited),
                ),
            ),
            ("audit-stored", fixture.audit("5min")),
            ("audit-range", fixture.audit_range("5min", (2025, 5))),
            ("screen", fixture.screen("5min", 1_000_000)),
            ("auto-stored", fixture.auto("5min")),
        ];
        for (door, report) in reports {
            let report = report.expect(door);
            assert_eq!(
                report.contains(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
                stock,
                "{symbol} {door}:\n{report}"
            );
            assert_eq!(
                report.contains("GROSS OF EVERY CHARGE"),
                stock,
                "{symbol} {door}:\n{report}"
            );
            if stock {
                assert!(
                    report.find("GROSS OF EVERY CHARGE")
                        < report.find(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
                    "{door}: beside and after the charge statement:\n{report}"
                );
            }
        }
    }
    crate::knobs::clear_all();
}

/// **A support descent loads its stored inputs once, and answers byte for byte
/// as a fresh load does.** o1cli-1, D-0997.
///
/// Every step of an `elite` descent re-ran the whole screen kernel, which
/// loaded the signal span, the one-minute execution span and both contexts and
/// rebuilt the anchored column, though none of that depends on the support
/// threshold. Measured before the fix: two steps, two loads. Now two steps over
/// one shared cache cost one load, the pages equal two uncached screens over an
/// identical store, and a cache handed a different question loads afresh.
#[test]
fn a_descent_loads_its_stored_inputs_once() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fresh = Fixture::warmed();
    let cached = Fixture::warmed();
    let supports = [1_000_000, 500_000, 20_000];
    crate::SCREEN_SPAN_LOADS.with(|loads| loads.set(0));
    let expected: Vec<String> = supports
        .iter()
        .map(|support| fresh.screen("5min", *support).expect("fresh step"))
        .collect();
    assert_eq!(crate::SCREEN_SPAN_LOADS.with(std::cell::Cell::get), 3);

    crate::SCREEN_SPAN_LOADS.with(|loads| loads.set(0));
    let mut cache = crate::ScreenCache::default();
    let request = |support_ppm| crate::StoredScreenRequest {
        root: cached.root.clone(),
        vendor: Vendor::Zerodha,
        underlying: cached.symbol,
        rung: "5min",
        span: ((2025, 5), (2025, 5)),
        support_ppm,
        policy: crate::Policy {
            rules: crate::Rules::BASELINE,
            lens: runner::rank::Lens::Detectability,
            validate: false,
        },
        attempt: Some(13),
        commit: "generated-stored-screen-fixture",
    };
    let got: Vec<String> = supports
        .iter()
        .map(|support| {
            crate::screen_range_kernel_cached(request(*support), &mut cache).expect("cached step")
        })
        .collect();
    assert_eq!(crate::SCREEN_SPAN_LOADS.with(std::cell::Cell::get), 1);
    let fresh_root = fresh.root.display().to_string();
    let cached_root = cached.root.display().to_string();
    for (want, page) in expected.iter().zip(&got) {
        assert!(page.contains("RESULT RECORDED"), "{page}");
        assert_eq!(&page.replace(&cached_root, &fresh_root), want);
    }

    // A different question is never answered from the held span.
    let other = crate::StoredScreenRequest {
        rung: "1min",
        ..request(1_000_000)
    };
    let one_minute = crate::screen_range_kernel_cached(other, &mut cache).expect("1min");
    assert_eq!(crate::SCREEN_SPAN_LOADS.with(std::cell::Cell::get), 2);
    assert_ne!(one_minute.replace(&cached_root, &fresh_root), expected[0]);
    crate::knobs::clear_all();
}

/// AC-whp-law-0, D-1661: `sweep-stored zerodha nifty` then `NIFTY` is ONE run.
/// The store reads one file for either case and the identity is built from the
/// canonical key, so the rerun must be accepted as the recorded answer. It was
/// refused as "deterministic fields differ" because the row stored the typed
/// word while the identity used the key.
#[test]
fn a_case_only_rerun_of_a_stored_sweep_is_the_recorded_answer() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    let lower = crate::StoredSweepRequest {
        underlying: "nifty",
        ..fixture.month_request("5min")
    };
    let first = crate::sweep_stored_kernel(lower).expect("lower case sweeps");
    assert!(first.contains("RESULT RECORDED"), "{first}");
    let again = fixture
        .sweep("5min")
        .expect("the canonical word is a rerun");
    assert!(
        again.contains("RESULT ALREADY RECORDED AND VERIFIED"),
        "{again}"
    );
    let mut ledger = crate::results::Results::open_read(&fixture.root).expect("ledger");
    assert_eq!(ledger.len().expect("one parent"), 1);
    assert_eq!(
        crate::results::read_field(&ledger.read(0).expect("parent").underlying),
        "NIFTY"
    );
}

/// W2-cli9-3 and W2-cli8-6, D-1662: three sessions that stop at 15:24 — their
/// 15:25-15:29 minutes missing while every other day holds them — are found by
/// the census before any column is built. The screen, which has no retry loop,
/// sweeps the span instead of refusing it, and the range audit builds its
/// column once instead of once per holed day plus one.
#[test]
fn sessions_missing_their_closing_minutes_are_withheld_up_front() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    let fixture = Fixture::warmed();
    let holed = [6_u8, 8, 12];
    fixture.rewrite_owned_minutes(|day, rows| {
        if holed.contains(&day) {
            let keep = rows.len().saturating_sub(5);
            rows.truncate(keep);
        }
    });
    let days: Vec<i64> = holed
        .iter()
        .map(|day| {
            i64::from(
                pull::session::Day::new(2025, 5, *day)
                    .expect("date")
                    .days_from_epoch(),
            )
        })
        .collect();

    let signal =
        stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", "5min", 2025, 5).expect("signal");
    let minutes =
        stored::load(&fixture.root, Vendor::Zerodha, "NIFTY", "1min", 2025, 5).expect("minutes");
    let signal_length = stored::rung_length_micros("5min").expect("five minutes");
    assert!(
        crate::minute_gaps::days_with_interior_gaps(&minutes.bars).is_empty(),
        "the interior walk cannot see a session that stops early"
    );
    assert_eq!(
        crate::minute_gaps::days_with_minute_holes(&signal.bars, &minutes.bars, signal_length),
        days
    );
    // On `1min` no signal bar demands the missing minutes, so none is withheld.
    assert!(
        crate::minute_gaps::days_with_minute_holes(&minutes.bars, &minutes.bars, 60_000_000)
            .is_empty()
    );
    // A day with signal bars and no minutes at all is flagged too.
    let no_minutes: Vec<_> = minutes
        .bars
        .iter()
        .copied()
        .filter(|bar| indicators::ist_day(bar.ts_micros) != days[0])
        .collect();
    assert!(
        crate::minute_gaps::days_with_minute_holes(&signal.bars, &no_minutes, signal_length)
            .contains(&days[0])
    );

    let screen = fixture.screen("5min", 1);
    assert!(screen.is_ok(), "{screen:?}");

    crate::COLUMN_BUILD_ATTEMPTS.with(|count| count.set(0));
    let range = fixture.audit_range("5min", (2025, 5));
    assert!(range.is_ok(), "{range:?}");
    assert_eq!(
        crate::COLUMN_BUILD_ATTEMPTS.with(std::cell::Cell::get),
        1,
        "every holed day was withheld before the first build"
    );
}
