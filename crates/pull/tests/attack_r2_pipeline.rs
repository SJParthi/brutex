//! ATTACK ROUND 2, PIPELINE: decode -> ingest -> store -> fold -> pricing, end
//! to end, on fixed-seed random months.
//!
//! Round 1 attacked each stage on its own. This file joins them: a seeded
//! generator writes Kite minute candles for random July 2025 sessions with
//! random defects (null prices, impossible OHLC, pre-open and after-close
//! minutes, exact duplicates), the shipped Zerodha decoder reads them, the
//! window is split into random day-aligned chunks and landed chunk by chunk,
//! and the bytes on disk are checked against an independent model of what
//! must be there: the minute file, every derived rung, the receipt's
//! arithmetic, idempotence, chunking independence, and the spot join pricing
//! reads from the stored month.
//!
//! splitmix64 with fixed seeds only, so reruns are byte-identical
//! (`CLAUDE.md` §3 rule 5).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::float_arithmetic
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::fetch::{BarRequest, RawWindow};
use pull::ingest::{Ingested, Plan};
use pull::session::{Day, Window};
use pull::vendor::{Granularity, Listing, TimestampEncoding};
use store::path::Timeframe;

const MINUTE: i64 = 60;
const SESSION_MINUTES: i64 = 375;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-attack-r2-pipeline-{tag}-{}",
            std::process::id()
        ));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn zerodha() -> pull::vendor::HttpSpec {
    let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    spec
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

/// 09:15 IST of `d` as a UTC epoch second.
fn open_utc(d: Day) -> i64 {
    i64::from(d.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60
}

fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::IsoDateTimeOffset,
        scale: pull::http::DECODED_PRICE_SCALE,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

fn ist_text(utc: i64) -> String {
    let local = utc + 19_800;
    let d = Day::from_days(u32::try_from(local.div_euclid(86_400)).unwrap()).unwrap();
    let secs = local.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+0530",
        d.year(),
        d.month(),
        d.day(),
        secs / 3_600,
        (secs / 60) % 60,
        secs % 60
    )
}

fn rupees(paisa: i64) -> String {
    format!("{}.{:02}", paisa / 100, paisa % 100)
}

fn body(candles: &[String]) -> String {
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        candles.join(",")
    )
}

/// What a well-formed minute must become on disk: `(ts_micros, o, h, l, c)`.
type Expected = (i64, i64, i64, i64, i64);

/// The vendor's answer for some sessions, and the model of what must land.
struct Month {
    candles: Vec<String>,
    /// The kept minutes, keyed by stamp in micros.
    kept: BTreeMap<i64, Expected>,
    /// Candles the decoder must skip: null price, impossible OHLC.
    skipped: usize,
    /// Candles outside the session.
    dropped: usize,
    /// Exact repeats that fold into the minute already open.
    folded: usize,
}

fn generate(mix: &mut Mix, days: &[Day]) -> Month {
    let mut out = Month {
        candles: Vec::new(),
        kept: BTreeMap::new(),
        skipped: 0,
        dropped: 0,
        folded: 0,
    };
    let mut level: i64 = 2_400_000 + mix.below(200_000) as i64;
    for &d in days {
        let open = open_utc(d);
        // pre-open minutes the vendor sometimes sends
        for k in (1..=mix.below(3) as i64).rev() {
            out.candles.push(format!(
                "[\"{}\",1.00,1.00,1.00,1.00,0]",
                ist_text(open - k * MINUTE)
            ));
            out.dropped += 1;
        }
        for m in 0..SESSION_MINUTES {
            let stamp = open + m * MINUTE;
            let step = mix.below(2_001) as i64 - 1_000;
            level = (level + step).max(1_000_000);
            let o = level;
            let c = level + mix.below(801) as i64 - 400;
            let h = o.max(c) + mix.below(300) as i64;
            let l = o.min(c) - mix.below(300) as i64;
            let roll = mix.below(1_000);
            if roll < 4 {
                out.candles
                    .push(format!("[\"{}\",null,null,null,null,0]", ist_text(stamp)));
                out.skipped += 1;
                continue;
            }
            if roll < 7 {
                // high below low: impossible, skipped by the decoder
                out.candles.push(format!(
                    "[\"{}\",{},{},{},{},0]",
                    ist_text(stamp),
                    rupees(o),
                    rupees(l - 1),
                    rupees(h + 1),
                    rupees(c)
                ));
                out.skipped += 1;
                continue;
            }
            let text = format!(
                "[\"{}\",{},{},{},{},0]",
                ist_text(stamp),
                rupees(o),
                rupees(h),
                rupees(l),
                rupees(c)
            );
            out.candles.push(text.clone());
            if roll < 15 {
                out.candles.push(text);
                out.folded += 1;
            }
            out.kept
                .insert(stamp * 1_000_000, (stamp * 1_000_000, o, h, l, c));
        }
        // the close and after it
        for k in 0..mix.below(3) as i64 {
            out.candles.push(format!(
                "[\"{}\",1.00,1.00,1.00,1.00,0]",
                ist_text(open + (SESSION_MINUTES + k) * MINUTE)
            ));
            out.dropped += 1;
        }
    }
    out
}

fn bar_file(root: &Path, tf: Timeframe) -> Vec<store::format::Bar> {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: tf,
        month: store::path::YearMonth::new(2025, 7).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let Ok(file) = store::file::BarFile::open_existing(
        root,
        path,
        brutex_core::universe::fnv1a("NIFTY") as u32,
    ) else {
        return Vec::new();
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn bar_images(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) != Some("lock") {
                out.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("bars"), &mut out);
    out.sort();
    out.into_iter()
        .map(|(p, b)| (p.strip_prefix(root).unwrap().to_path_buf(), b))
        .collect()
}

/// Splits the window's days into contiguous chunks at random cut points.
fn chunks(mix: &mut Mix, days: &[Day]) -> Vec<Vec<Day>> {
    let mut out: Vec<Vec<Day>> = vec![Vec::new()];
    for (i, &d) in days.iter().enumerate() {
        if i > 0 && mix.below(2) == 0 {
            out.push(Vec::new());
        }
        out.last_mut().unwrap().push(d);
    }
    out
}

/// Lands `month` chunk by chunk, each chunk's candles decoded on their own and
/// filed against the window they were asked for.
fn land_in_chunks(root: &Path, month: &Month, cut: &[Vec<Day>]) -> Ingested {
    let mut total = Ingested::default();
    for chunk in cut {
        let from = chunk[0];
        let to = *chunk.last().unwrap();
        // the candles of these days only, in vendor order
        let mine: Vec<String> = month
            .candles
            .iter()
            .filter(|c| {
                let stamp = &c[2..21];
                let d = day(
                    stamp[0..4].parse().unwrap(),
                    stamp[5..7].parse().unwrap(),
                    stamp[8..10].parse().unwrap(),
                );
                d >= from && d <= to
            })
            .cloned()
            .collect();
        let raw: RawWindow =
            pull::http::decode_body(&body(&mine), &zerodha(), Listing::Index).expect("decodes");
        assert_eq!(
            raw.rows.len() + raw.skipped.total(),
            mine.len(),
            "the decoder accounts for every candle it was sent"
        );
        let request = BarRequest {
            instrument_id: String::new(),
            listing: Listing::Index,
            window: Window::new(from, to).expect("a legal window"),
            granularity: Granularity::Minute1,
        };
        total.absorb(pull::ingest::from_window(
            &raw,
            "NIFTY",
            "attack-r2",
            root,
            plan(&request),
        ));
    }
    total
}

/// Folds the model's kept minutes into `width`-second buckets on the 09:15
/// grid, keeping only buckets whose every session minute is present.
fn model_rung(kept: &BTreeMap<i64, Expected>, days: &[Day], width: i64) -> Vec<Expected> {
    let mut out = Vec::new();
    for &session in days {
        let open = open_utc(session);
        let close = open + SESSION_MINUTES * MINUTE;
        let mut start = open;
        while start < close {
            let end = (start + width).min(close);
            let mut bucket: Option<Expected> = None;
            let mut complete = true;
            let mut minute = start;
            while minute < end {
                match kept.get(&(minute * 1_000_000)) {
                    Some(&(_, first, high, low, last)) => {
                        bucket = Some(match bucket {
                            None => (start * 1_000_000, first, high, low, last),
                            Some((stamp, opened, top, bottom, _)) => {
                                (stamp, opened, top.max(high), bottom.min(low), last)
                            }
                        });
                    }
                    None => complete = false,
                }
                minute += MINUTE;
            }
            if complete && let Some(done) = bucket {
                out.push(done);
            }
            start += width;
        }
    }
    out
}

fn as_expected(bars: &[store::format::Bar]) -> Vec<Expected> {
    bars.iter()
        .map(|b| (b.ts_micros, b.open, b.high, b.low, b.close))
        .collect()
}

/// July 2025 sessions: no NSE holiday falls in the month.
fn july_sessions() -> Vec<Day> {
    let mut out = Vec::new();
    for d in 1..=31u8 {
        let x = day(2025, 7, d);
        // 2025-07-05 is a Saturday
        let weekday = (u32::from(d) + 2) % 7; // 0 = Saturday, 1 = Sunday
        if weekday != 0 && weekday != 1 {
            out.push(x);
        }
    }
    out
}

/// **THE WHOLE DATA PATH AGREES WITH AN INDEPENDENT MODEL, ON RANDOM MONTHS.**
///
/// For each seed: random consecutive July sessions, random defects, random
/// chunking. The receipt's arithmetic holds with decoder skips in it; the
/// minute file is exactly the kept minutes; each derived rung is exactly the
/// model's complete buckets; a rerun writes nothing and changes no byte; and a
/// different chunking of the same answer lands the same bar files.
#[test]
fn random_months_land_the_modelled_bytes_through_every_stage() {
    let sessions = july_sessions();
    assert_eq!(sessions.len(), 23, "July 2025 has 23 weekdays");
    let mut mix = Mix(0x00A7_7AC4_0002_3180);
    for case in 0..48 {
        let start = mix.below(sessions.len() as u64 - 3) as usize;
        let len = 1 + mix.below(4) as usize;
        let days: Vec<Day> = sessions[start..(start + len).min(sessions.len())].to_vec();
        let month = generate(&mut mix, &days);
        let a = Scratch::new(&format!("a{case}"));
        let b = Scratch::new(&format!("b{case}"));

        let cut_a = chunks(&mut mix, &days);
        let done = land_in_chunks(&a.0, &month, &cut_a);
        // THE ARITHMETIC, whether or not a derived rung reported a failure.
        assert_eq!(done.rows_read, month.candles.len(), "case {case}: {done:?}");
        assert_eq!(done.decoder_skips.total(), month.skipped, "case {case}");
        assert_eq!(done.census.total() as usize, month.dropped, "case {case}");
        assert_eq!(done.rows_folded, month.folded, "case {case}");
        assert_eq!(done.bars_stored, month.kept.len(), "case {case}");
        assert_eq!(
            done.rows_read,
            done.bars_stored
                + done.rows_folded
                + done.census.total() as usize
                + done.decoder_skips.total(),
            "case {case}"
        );
        // A minute the decoder skipped is a hole the request audit names, and
        // the derived buckets over it are withheld by name. Never a refused
        // bar, and never silence: a month with a skip carries a failure.
        for f in &done.failures {
            assert!(
                f.why.contains("coverage gap")
                    || f.why.contains("withheld")
                    || f.why.contains("derived"),
                "case {case}: an unexpected failure: {}",
                f.why
            );
        }
        if month.skipped == 0 {
            assert!(done.balances(), "case {case}: {:?}", done.failures);
        } else {
            assert!(
                done.failures.iter().any(|f| f.why.contains("coverage gap")),
                "case {case}: {} skipped minute(s) and no gap named: {:?}",
                month.skipped,
                done.failures
            );
        }

        // THE MINUTE FILE IS THE MODEL.
        let minutes = bar_file(&a.0, Timeframe::MINUTE_1);
        let want: Vec<Expected> = month.kept.values().copied().collect();
        assert_eq!(as_expected(&minutes), want, "case {case}: minute file");

        // EVERY DERIVED RUNG IS THE MODEL'S COMPLETE BUCKETS.
        for tf in [
            Timeframe::MINUTE_2,
            Timeframe::MINUTE_3,
            Timeframe::MINUTE_5,
            Timeframe::MINUTE_10,
            Timeframe::MINUTE_15,
            Timeframe::MINUTE_30,
            Timeframe::MINUTE_60,
        ] {
            let got = as_expected(&bar_file(&a.0, tf));
            let want = model_rung(&month.kept, &days, i64::from(tf.secs()));
            assert_eq!(
                got,
                want,
                "case {case}: {} differs from the model",
                tf.as_str()
            );
        }

        // A RERUN WRITES NOTHING AND CHANGES NO BYTE.
        let before = bar_images(&a.0);
        let again = land_in_chunks(&a.0, &month, &cut_a);
        assert_eq!(again.bars_committed, 0, "case {case}");
        assert_eq!(again.rows_read, done.rows_read, "case {case}");
        assert_eq!(
            bar_images(&a.0),
            before,
            "case {case}: a rerun changed bytes"
        );

        // ANOTHER CHUNKING LANDS THE SAME RECORDS. Not the same file bytes:
        // the header's generation counts commits, and four chunks are four
        // commits where two are two. Every record of every rung must agree.
        let cut_b = chunks(&mut mix, &days);
        let other = land_in_chunks(&b.0, &month, &cut_b);
        assert_eq!(other.bars_stored, done.bars_stored, "case {case}");
        for &tf in Timeframe::KNOWN {
            assert_eq!(
                bar_file(&b.0, tf),
                bar_file(&a.0, tf),
                "case {case}: {}: chunking {cut_a:?} and {cut_b:?} landed different records",
                tf.as_str()
            );
        }

        // THE SPOT JOIN PRICING READS: every stored minute answers its own
        // close, no stamp is ambiguous, and a stamp the store does not hold is
        // refused rather than answered by a neighbour.
        let book = pull::pricing::SpotBook::of(&minutes);
        assert_eq!(book.len(), minutes.len(), "case {case}");
        assert_eq!(book.ambiguous(), 0, "case {case}");
        for bar in &minutes {
            assert_eq!(book.lookup(bar.ts_micros).unwrap(), bar.close);
            assert!(book.lookup(bar.ts_micros + 1).is_err());
            assert!(book.lookup(bar.ts_micros + 30_000_000).is_err());
        }
    }
}
