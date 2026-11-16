#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::print_stdout,
    clippy::too_many_lines,
    clippy::cast_possible_wrap
)]
//! SC-1's acceptance on REAL months, which never enter this repository.
//!
//! The design (`store-compression-20261012.md` §9) holds codec 1 to the
//! prototype it was measured with: the eight sample months and the 539-month
//! stratified manifest must encode to the prototype's frame bytes less the
//! computed saving of a varint first row, within 0.5%. The months are clones of
//! real store months kept outside the repository, so this test reads them from
//! the directory `BRUTEX_SEAL_SAMPLES` names (the design's `store-compression`
//! evidence directory) and is `#[ignore]`d: `cargo test` never needs them, and
//! a run that asks for it without the directory FAILS rather than passes.
//!
//! `BRUTEX_SEAL_EXTRA` may list more `.bin` months, one absolute path a line
//! (read only, never written): each is round-tripped bit for bit and its size
//! reported, with no prototype figure to compare against.
//!
//! ```text
//! BRUTEX_SEAL_SAMPLES=…/state/design/store-compression \
//!   cargo test -p store --release --test sealed_codec_samples -- --ignored --nocapture
//! ```
//!
//! Besides the sizes it times, per month, one frame's decode with its checksum,
//! one frame's stamps alone, one frame's encode, and a 100-row page at a random
//! offset (the at most two frames it spans, decoded from memory), and prints
//! p50, p99 and max with n. It prints them and does not gate on them: a shared
//! machine's tail is not this codec's (the gated benches are SC-11's C-SEAL-*).

use std::path::Path;
use std::time::Instant;
use store::crc::crc32c;
use store::format::{Bar, OI_NULL};
use store::header::Header;
use store::path::YearMonth;
use store::sealed_codec::{BLOCK_ROWS, MAX_FRAME_LEN, decode_frame, decode_stamps, encode_frame};

/// Bytes before the first record of a version-3 bar file, and one record's
/// bytes: the store's own layout constants, never a second copy of them.
const RECORDS_AT: usize = store::format::HEADER_LEN as usize;
const STRIDE: usize = store::format::RECORD_STRIDE as usize;

/// The eight sample months: directory, month, the prototype's frame bytes
/// (`blocks/proto/enc.txt`, `body=`), and the judge's computed row-0 saving
/// (`judge/row0.txt`, `saved=`).
const SAMPLES: [(&str, &str, u64, u64); 8] = [
    ("nifty-spot-1s", "2026-08", 1_502_343, 76_910),
    ("opt-large", "2025-06", 275_741, 17_860),
    ("opt-atm", "2025-06", 294_449, 17_161),
    ("opt-mid", "2025-06", 75_494, 4_001),
    ("opt-tiny", "2025-06", 2_686, 122),
    ("opt-onebar", "2025-05", 88, 44),
    ("reliance-1s", "2026-08", 630_096, 37_589),
    ("zerodha-nifty-1min", "2026-08", 35_261, 1_205),
];

fn read_month(path: &Path) -> Vec<Bar> {
    // A month is a `.bin`; anything else (a list of months handed in as a
    // file, a sidecar) is refused by name, never read as bars.
    assert!(
        path.extension().is_some_and(|e| e == "bin"),
        "{}: not a bar month (`.bin`); BRUTEX_SEAL_EXTRA holds the months themselves, one path a line",
        path.display()
    );
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let header = data.get(..RECORDS_AT).unwrap_or_else(|| {
        panic!(
            "{}: {} bytes, shorter than a bar file's {RECORDS_AT}-byte header",
            path.display(),
            data.len()
        )
    });
    let header = Header::read_region(header, data.len() as u64)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let n = header.n_valid as usize;
    data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)
        .unwrap_or_else(|| {
            panic!(
                "{}: the header commits {n} records, the file holds fewer",
                path.display()
            )
        })
        .chunks_exact(STRIDE)
        .map(|record| Bar::decode(record).expect("a committed record decodes"))
        .collect()
}

/// 00:00 IST on the 1st of the month named `yyyy-mm` at the end of `name`.
fn origin_of(name: &str) -> i64 {
    let stem = name.trim_end_matches(".bin");
    let month = &stem[stem.len() - 7..];
    let year: u16 = month[..4].parse().expect("year");
    let mm: u8 = month[5..].parse().expect("month");
    YearMonth::new(year, mm)
        .expect("a month")
        .ist_bounds_micros()
        .0
}

/// The judge's own estimate of a varint row 0 (`judge/row0.py`), reproduced
/// exactly: relative to the month's FIRST stamp, with one byte of flags and
/// one byte of open-interest presence a frame that codec 1 does not spend.
fn judge_row0(rows: &[Bar]) -> u64 {
    fn len(mut v: u64) -> u64 {
        let mut n = 1;
        while v >= 0x80 {
            v >>= 7;
            n += 1;
        }
        n
    }
    fn zz(x: i64) -> u64 {
        ((x << 1) ^ (x >> 63)).cast_unsigned()
    }
    let first = rows.first().map_or(0, |b| b.ts_micros);
    rows.iter()
        .step_by(256)
        .map(|b| {
            let d = b.ts_micros - first;
            let ts = if d % 1_000_000 == 0 {
                len(zz(d / 1_000_000))
            } else {
                len(zz(d))
            };
            let oi = if b.open_interest == OI_NULL {
                1
            } else {
                1 + len(zz(b.open_interest))
            };
            ts + 1
                + len(zz(b.close))
                + len(zz(b.open - b.close))
                + len(zz(b.high - b.close))
                + len(zz(b.low - b.close))
                + len(zz(b.volume))
                + oi
        })
        .sum()
}

/// Zigzag, as codec 1 and the prototype both define it.
const fn zz(x: i64) -> u64 {
    ((x << 1) ^ (x >> 63)).cast_unsigned()
}

const fn varint_len(value: u64) -> usize {
    let bits = (64 - value.leading_zeros()) as usize;
    if bits == 0 { 1 } else { bits.div_ceil(7) }
}

/// Codec 1's row 0, computed from the specification rather than read back.
fn row0_len(first: &Bar, origin: i64) -> usize {
    let d = first.ts_micros.wrapping_sub(origin);
    let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
    let oi = if first.open_interest == OI_NULL {
        0
    } else {
        varint_len(zz(first.open_interest))
    };
    varint_len(zz(stamp))
        + varint_len(zz(first.close))
        + varint_len(zz(first.open.wrapping_sub(first.close)))
        + varint_len(zz(first.high.wrapping_sub(first.close)))
        + varint_len(zz(first.low.wrapping_sub(first.close)))
        + varint_len(zz(first.volume))
        + oi
}

/// One column's residuals in one mode: the predictor table of the design,
/// written again here so the acceptance does not lean on the codec's own.
fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {
    rows.windows(2)
        .map(|pair| {
            let (p, b) = (pair[0], pair[1]);
            let (value, predictor) = match (column, mode) {
                (0, _) => (b.ts_micros, p.ts_micros),
                (1, _) => (b.close, p.close),
                (2, 0) => (b.open, p.close),
                (2, _) => (b.open, b.close),
                (3, _) => (b.high, b.open.max(b.close)),
                (4, _) => (b.low, b.open.min(b.close)),
                (5, 0) => (b.volume, 0),
                (5, _) => (b.volume, p.volume),
                (6, 0) => (b.open_interest, p.open_interest),
                _ => (b.open_interest, 0),
            };
            value.wrapping_sub(predictor)
        })
        .collect()
}

/// The stream the PROTOTYPE would write for these residuals
/// (`blocks/proto/src/codec.rs`, `pack_stream`, with exceptions on): the same
/// gcd, base and byte layout as codec 1, but the width chosen by its
/// approximate bit cost `n·w + exceptions·(p + top) + 16`, scanning down from
/// the widest and keeping only a strictly smaller cost.
fn prototype_stream_len(residuals: &[i64]) -> usize {
    fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    let n = residuals.len();
    let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));
    if g == 0 || g >= 1 << 62 {
        g = 1;
    }
    let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();
    let base = values.iter().copied().min().unwrap_or(0);
    let mut histogram = [0usize; 65];
    for v in &values {
        histogram[(64 - (v - base).leading_zeros()) as usize] += 1;
    }
    let mut top = 64;
    while top > 0 && histogram[top] == 0 {
        top -= 1;
    }
    let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);
    let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);
    while w > 0 {
        exceptions += histogram[w];
        w -= 1;
        let cost = n * w + exceptions * (p + top) + 16;
        if cost < best {
            best = cost;
            best_w = w;
        }
    }
    let limit = if best_w == 64 {
        u64::MAX
    } else {
        (1 << best_w) - 1
    };
    let exceptions = values.iter().filter(|&&v| v - base > limit).count();
    let mut len = 1
        + varint_len(exceptions as u64)
        + varint_len(base)
        + varint_len(g)
        + (n * best_w).div_ceil(8);
    if exceptions > 0 {
        len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);
    }
    len
}

/// The frame the prototype's rule writes with codec 1's head and row 0.
fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {
    let streams: usize = (0..7)
        .map(|column| {
            let zero = prototype_stream_len(&residuals(rows, column, 0));
            if matches!(column, 2 | 5 | 6) {
                zero.min(prototype_stream_len(&residuals(rows, column, 1)))
            } else {
                zero
            }
        })
        .sum();
    4 + row0_len(&rows[0], origin) + streams
}

struct Sealed {
    bytes: u64,
    /// The same month under the prototype's width rule (codec 1's layout).
    prototype_rule: u64,
    /// Codec 1's row 0 of every frame, computed from the specification.
    row0: u64,
    frames: Vec<(Vec<u8>, u32)>,
}

/// Seals a month frame by frame, proves every frame round-trips bit for bit
/// through both decoders, and returns the frames.
fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {
    let mut frames = Vec::with_capacity(rows.len().div_ceil(256));
    let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);
    for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {
        let rule = prototype_frame_len(chunk, origin);
        first_rows += row0_len(&chunk[0], origin) as u64;
        let mut frame = Vec::new();
        let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");
        assert!(frame.len() <= MAX_FRAME_LEN, "{label}");
        assert_eq!(encoded.crc, crc32c(&frame));
        let mut back = Vec::new();
        decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");
        assert_eq!(back, chunk, "{label}: a frame came back different");
        let mut stamps = Vec::new();
        decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");
        assert!(
            stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
            "{label}"
        );
        // The exact choice is never longer than the prototype's approximate one.
        assert!(
            frame.len() <= rule,
            "{label}: {} against {rule}",
            frame.len()
        );
        bytes += frame.len() as u64;
        prototype_rule += rule as u64;
        frames.push((frame, encoded.crc));
    }
    Sealed {
        bytes,
        prototype_rule,
        row0: first_rows,
        frames,
    }
}

fn percentiles(label: &str, mut ns: Vec<u64>) -> String {
    ns.sort_unstable();
    let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];
    format!(
        "{label}: n={} p50={}ns p99={}ns max={}ns",
        ns.len(),
        at(50),
        at(99),
        ns.last().copied().unwrap_or(0)
    )
}

/// Times one month's operations and prints them.
fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {
    let mut decode = Vec::new();
    let mut stamps = Vec::new();
    let mut out = Vec::with_capacity(512);
    let mut ts = Vec::with_capacity(256);
    for _ in 0..(2_000 / sealed.frames.len()).max(1) {
        for (frame, crc) in &sealed.frames {
            out.clear();
            let t = Instant::now();
            decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
            decode.push(t.elapsed().as_nanos() as u64);
            ts.clear();
            let t = Instant::now();
            decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");
            stamps.push(t.elapsed().as_nanos() as u64);
        }
    }
    let mut encode = Vec::new();
    let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);
    for chunk in rows.chunks(256).take(2_000) {
        buffer.clear();
        let t = Instant::now();
        encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");
        encode.push(t.elapsed().as_nanos() as u64);
    }
    // A 100-row page at a pseudo-random offset: the frames it spans.
    let mut page = Vec::new();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let n = rows.len() as u64;
    for _ in 0..2_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let first = state % n;
        let last = (first + 99).min(n - 1);
        out.clear();
        let t = Instant::now();
        for k in (first / 256)..=(last / 256) {
            let (frame, crc) = &sealed.frames[k as usize];
            decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
        }
        page.push(t.elapsed().as_nanos() as u64);
        let skip = (first % 256) as usize;
        assert_eq!(out[skip], rows[first as usize]);
    }
    println!(
        "  {}",
        percentiles(&format!("{label} frame decode+crc"), decode)
    );
    println!(
        "  {}",
        percentiles(&format!("{label} frame stamps+crc"), stamps)
    );
    println!(
        "  {}",
        percentiles(&format!("{label} frame encode"), encode)
    );
    println!(
        "  {}",
        percentiles(&format!("{label} page100 from memory"), page)
    );
}

/// Sums of one set of months, and the months that broke a rule.
#[derive(Default)]
struct Totals {
    codec1: u64,
    rule: u64,
    exact: u64,
    judge: u64,
    /// Months whose prototype-rule frames are not byte for byte the
    /// prototype's measured frames with codec 1's row 0.
    not_exact: Vec<String>,
    /// Months of 1,024 bars or more more than 0.5% LARGER than the judge's figure.
    larger: Vec<String>,
}

impl Totals {
    fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {
        if sealed.prototype_rule != exact {
            self.not_exact.push(format!(
                "{label}: rule {} exact {exact}",
                sealed.prototype_rule
            ));
        }
        if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
            self.larger
                .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));
        }
        self.codec1 += sealed.bytes;
        self.rule += sealed.prototype_rule;
        self.exact += exact;
        self.judge += judge;
    }

    fn check(&self, label: &str) {
        let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);
        println!(
            "{label}: codec1={} rule={} exact={} judge={}; codec1 against exact {}, against judge {}; months not byte-exact {}; months more than 0.5% above judge {}",
            self.codec1,
            self.rule,
            self.exact,
            self.judge,
            d(self.codec1, self.exact),
            d(self.codec1, self.judge),
            self.not_exact.len(),
            self.larger.len()
        );
        // The stream format IS the prototype's: byte for byte, every month.
        assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);
        assert_eq!(self.rule, self.exact, "{label}");
        // The exact width choice only ever removes bytes, and not many.
        assert!(self.codec1 <= self.rule, "{label}");
        assert!(
            within(self.codec1, self.exact).0,
            "{label}: codec1 against exact"
        );
        // The design's own figure: within 0.5% in all, and no month of
        // 1,024 bars or more above it by more than 0.5%.
        assert!(
            within(self.codec1, self.judge).0,
            "{label}: codec1 against judge"
        );
        assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);
    }
}

/// `diff` as a signed percentage of `of`, to three places, in integers.
fn per_cent(diff: i64, of: u64) -> String {
    let thousandths = diff.unsigned_abs() * 100_000 / of;
    let sign = if diff < 0 { "-" } else { "+" };
    format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)
}

/// `mine` against the prototype's figure less the row-0 saving.
fn within(mine: u64, expected: u64) -> (bool, i64) {
    let diff = mine.cast_signed() - expected.cast_signed();
    // 0.5% as an integer comparison: |diff| * 200 <= expected.
    (diff.unsigned_abs() * 200 <= expected, diff)
}

#[test]
#[ignore = "reads real months outside the repository; set BRUTEX_SEAL_SAMPLES"]
fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {
    let root = std::env::var("BRUTEX_SEAL_SAMPLES")
        .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");
    let root = Path::new(&root);

    // Three figures a month, all frames only:
    //   codec1   what this codec writes;
    //   rule     codec 1's layout with the prototype's width rule;
    //   exact    the prototype's measured frames with its 56-byte row 0
    //            replaced by codec 1's row 0 — "the prototype's sizes less the
    //            computed first-row saving", the saving computed exactly;
    //   judge    the same with the judge's estimate of the saving
    //            (`judge/row0.txt`), which charges a flags byte and an
    //            open-interest byte a frame that codec 1 keeps in its head.
    println!("eight sample months:");
    let mut totals = Totals::default();
    for (dir, month, body, saved) in SAMPLES {
        let path = root
            .join("blocks/samples")
            .join(dir)
            .join(format!("{month}.bin"));
        let rows = read_month(&path);
        let origin = origin_of(month);
        let sealed = seal(&rows, origin, dir);
        let blocks = sealed.frames.len() as u64;
        let exact = body - 56 * blocks + sealed.row0;
        let judge = body - saved;
        println!(
            "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",
            rows.len(),
            sealed.bytes,
            sealed.prototype_rule,
            per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),
            rows.len() * 56,
            sealed.bytes * 100 / rows.len() as u64,
        );
        time(&rows, origin, &sealed, dir);
        totals.add(dir, rows.len(), &sealed, exact, judge);
    }
    totals.check("eight samples in all");

    let manifest =
        std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");
    let out =
        std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");
    let mut totals = Totals::default();
    let (mut months, mut bars, mut empty) = (0, 0, 0);
    for (line, result) in manifest.lines().zip(out.lines()) {
        let fields: Vec<&str> = line.split('\t').collect();
        let measured: Vec<&str> = result.split('\t').collect();
        let rows = read_month(&root.join("blocks").join(fields[0]));
        assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);
        if rows.is_empty() {
            empty += 1;
            continue;
        }
        let origin = origin_of(fields[4]);
        let sealed = seal(&rows, origin, fields[4]);
        let body: u64 = measured[3].parse().expect("body");
        let blocks = sealed.frames.len() as u64;
        let exact = body - 56 * blocks + sealed.row0;
        let judge = body - (56 * blocks - judge_row0(&rows));
        totals.add(fields[4], rows.len(), &sealed, exact, judge);
        months += 1;
        bars += rows.len();
    }
    println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");
    totals.check("the manifest in all");
    if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
        println!("extra months (read only, round trip and size):");
        for path in extra.lines().filter(|l| !l.trim().is_empty()) {
            let path = Path::new(path.trim());
            let rows = read_month(path);
            let name = path.to_string_lossy();
            let origin = origin_of(&name);
            if rows.is_empty() {
                println!("{name}: empty month");
                continue;
            }
            let sealed = seal(&rows, origin, &name);
            println!(
                "{name}: bars={} frames={} codec1={} raw={} round trip bit for bit",
                rows.len(),
                sealed.frames.len(),
                sealed.bytes,
                rows.len() * 56
            );
            time(&rows, origin, &sealed, "extra");
        }
    }
}
