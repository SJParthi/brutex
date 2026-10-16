//! C-TSW-01 and C-TSW-02 (R4-08, E2E-02): the tick-store writer's cost per
//! member at 1x, 10x and 100x a day's members, and a written day file's point
//! lookup, flat across the same three sizes.
//!
//! Writing is O(the day's bytes) by design (every byte is read, encoded,
//! written and read back once), so the bound asserted is the cost PER MEMBER
//! of equal size: flat when the writer does no work that grows with the day
//! beyond its bytes. A day's fixed cost (the lock, the folders, the fsyncs,
//! the threads) dominates a 1x day, so the 1x day is not the baseline: the
//! MARGINAL cost per member from 1x to 10x is, against the marginal cost from
//! 10x to 100x (the second check of R4-08 found the old ratio could hardly
//! fail). Seconds per day and MB/s are printed beside it. The
//! lookup is the readers' own: one listing of the written file, then one
//! probe by ticker, which must cost the same at 100 times the members.
//!
//! Every input is invented here; each run writes into a new folder under the
//! system temp directory, printed, and left there (this harness removes no
//! file; the main session's clean-up does, R4-08's no-delete rule).

use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pull::gdfl_nfo::{NfoSource as _, NfoTickStore};
use pull::session::Day;
use pull::tickstore_write::{Outcome, WriteOptions, write_day};

use super::{cost_ps, ratio, refuse, spread};

/// Members of the 1x day; 10x and 100x are ten and a hundred times it.
const BASE_MEMBERS: u32 = 40;
/// Rows of each invented option file.
const ROWS: u32 = 60;
/// Writes of each size; the fastest is taken, as every ratio here does.
const WRITES: u32 = 3;

/// A stored-only zip of `members`, in order.
fn zip_of(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let crc = |bytes: &[u8]| {
        let mut c = flate2::Crc::new();
        c.update(bytes);
        c.sum()
    };
    let n16 = |v: usize| u16::try_from(v).unwrap_or_else(|_| refuse("a short zip field"));
    let n32 = |v: usize| u32::try_from(v).unwrap_or_else(|_| refuse("a small zip"));
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, data) in members {
        let offset = n32(out.len());
        let fixed = |made_by: bool| {
            let mut head = Vec::new();
            if made_by {
                head.extend_from_slice(&20_u16.to_le_bytes());
            }
            for half in [20_u16, 0, 0, 0x6000, 0x5B3D] {
                head.extend_from_slice(&half.to_le_bytes());
            }
            head.extend_from_slice(&crc(data).to_le_bytes());
            head.extend_from_slice(&n32(data.len()).to_le_bytes());
            head.extend_from_slice(&n32(data.len()).to_le_bytes());
            head.extend_from_slice(&n16(name.len()).to_le_bytes());
            head.extend_from_slice(&0_u16.to_le_bytes());
            head
        };
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&fixed(false));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&fixed(true));
        central.extend_from_slice(&[0; 10]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let at = n32(out.len());
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&n16(members.len()).to_le_bytes());
    out.extend_from_slice(&n16(members.len()).to_le_bytes());
    out.extend_from_slice(&n32(central.len()).to_le_bytes());
    out.extend_from_slice(&at.to_le_bytes());
    out.extend_from_slice(&[0; 2]);
    out
}

/// The ticker of invented option file `i`.
fn ticker(i: u32) -> String {
    format!("X{i}29SEP2699000CE")
}

/// An options day zip of `n` invented files of [`ROWS`] rows each.
fn options_day(n: u32) -> Vec<u8> {
    let folder = "GFDLNFO_TICK_29092026";
    let mut members = vec![
        (format!("{folder}/"), Vec::new()),
        (format!("{folder}/Options/"), Vec::new()),
    ];
    for i in 0..n {
        let mut text = String::from("Ticker,Date,Time,LTP,LTQ,OpenInterest\r\n");
        for r in 0..ROWS {
            let secs = 33_300 + r * 11 + i % 7;
            let _infallible = write!(
                text,
                "{},31/12/1999,{:02}:{:02}:{:02},{}.{:02},{},{}\r\n",
                ticker(i),
                secs / 3_600,
                secs / 60 % 60,
                secs % 60,
                900 + (r + i) % 17,
                (r * 5) % 100,
                75 * (1 + r % 4),
                10_000 + r * 3
            );
        }
        members.push((
            format!("{folder}/Options/{}.NFO.csv", ticker(i)),
            text.into_bytes(),
        ));
    }
    zip_of(&members)
}

/// One day of `n` members written `WRITES` times into fresh roots under
/// `base`: the fastest write in nanoseconds, the zip's member bytes, and the
/// root of the last write.
fn write_size(base: &Path, n: u32) -> (u128, u64, PathBuf) {
    let zips = base.join(format!("zips-{n}"));
    std::fs::create_dir_all(&zips).unwrap_or_else(|_| refuse("a scratch folder"));
    let zip = zips.join("GFDLNFO_TICK_29092026.zip");
    std::fs::write(&zip, options_day(n)).unwrap_or_else(|_| refuse("the day zip"));
    let (mut best, mut csv, mut last) = (u128::MAX, 0, PathBuf::new());
    for trial in 0..WRITES {
        let root = base.join(format!("root-{n}-{trial}"));
        std::fs::create_dir_all(&root).unwrap_or_else(|_| refuse("a root"));
        let start = Instant::now();
        let run = write_day(&zip, &root, &WriteOptions::new(&format!("bench-{trial}")));
        let ns = start.elapsed().as_nanos();
        let Ok(Outcome::Written(report)) = run.result else {
            refuse("a written day")
        };
        best = best.min(ns);
        csv = report.csv_bytes;
        last = root;
    }
    (best, csv, last)
}

/// C-TSW-01 and C-TSW-02.
pub(crate) fn tickstore_write_is_flat_per_member() -> bool {
    let base = std::env::temp_dir().join(format!(
        "brutex-tsw-bench-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    println!("  C-TSW scratch (left for clean-up): {}", base.display());
    let sizes = [BASE_MEMBERS, BASE_MEMBERS * 10, BASE_MEMBERS * 100];
    let mut days = Vec::new();
    let mut roots = Vec::new();
    for n in sizes {
        let (ns, csv, root) = write_size(&base, n);
        let mb_per_s = u128::from(csv) * 1_000 / ns.max(1);
        println!(
            "  C-TSW-01 {n:>5} members: {:>9} us a day, {csv:>10} CSV bytes, {mb_per_s} MB/s",
            ns / 1_000
        );
        days.push(ns);
        roots.push((n, root));
    }
    let mut ok = true;
    if let [one, ten, hundred] = days.as_slice() {
        // Picoseconds a member, the fixed part of a day subtracted.
        let marginal = |small: u128, big: u128, members: u32| {
            big.saturating_sub(small) * 1_000 / u128::from(members)
        };
        let low = marginal(*one, *ten, BASE_MEMBERS * 9);
        let high = marginal(*ten, *hundred, BASE_MEMBERS * 90);
        println!(
            "  C-TSW-01 fixed part of a day about {} us (1x day less 1x marginal)",
            one.saturating_sub(low * u128::from(BASE_MEMBERS) / 1_000) / 1_000
        );
        ok &= ratio("C-TSW-01 marginal write per member, 10x->100x", low, high);
    }
    let day = Day::new(2026, 9, 29).unwrap_or_else(|_| refuse("a real day"));
    let mut probes = Vec::new();
    for (n, root) in &roots {
        let store = NfoTickStore::new(root);
        let Ok(Some(listed)) = store.day(day) else {
            refuse("the written day reads back")
        };
        let want = ticker(n / 2);
        probes.push(cost_ps(20_000, || {
            black_box(&listed).locate(black_box(&want)).is_ok()
        }));
        if *n == BASE_MEMBERS * 100 {
            spread(
                "C-TSW-02 lookup by ticker, written 100x day",
                20_000,
                || {
                    black_box(listed.locate(black_box(&want)).is_ok());
                },
            );
            let Ok(Some(file)) = listed.locate(&want) else {
                refuse("the probed file")
            };
            spread("C-TSW-02 one file rebuilt, written 100x day", 200, || {
                black_box(store.fetch(file).is_ok());
            });
        }
    }
    if let [one, ten, hundred] = probes.as_slice() {
        ok &= ratio("C-TSW-02 lookup by ticker, 10x day", *one, *ten);
        ok &= ratio("C-TSW-02 lookup by ticker, 100x day", *one, *hundred);
    }
    ok
}
