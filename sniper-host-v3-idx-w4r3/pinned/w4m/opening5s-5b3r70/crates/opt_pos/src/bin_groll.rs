//! `g-roll` (S8 of SNIPER-IDXOPT-PLAN-v3.1, gate G-ROLL's Rust side): forced positions on the named real expiry weeks, priced by
//! `opt_pos::pos` on the frozen store's 1-s option bars: per week, a long (CE) and a short (PE) entered at the week's entry instant
//! (10:00:00 on its first session), each its own Rs 20 L account (or ONE account for a MIXED row), the exit forced at the week's exit
//! instant (after the expiry, or R1's flip day), on both protection paths (kept / off) and both open rules ((a) sequential, the
//! headline; (b) same decision, beside). Every leg, fill row, trip (charges, settlement) and counted flag is written in one canonical
//! TSV that the independent node walker writes too (`groll/node/groll-walker.mjs`).
//!
//! `g-roll --store ROOT --ladder F --freeze F --s1 DIR --halts F --weeks F --out FILE [--only ID,ID] [--slot-rule base|fixed]`
//!
//! s8b: `--slot-rule base` prices every run under `SlotRule::BaseOverN` (MAIN ruling 2 read literally, BESIDE only); the default is
//! the headline fixed slot.
//! W4 REPAIR (check-wave4-S9 D-W4-3): the default is now THE headline `SlotRule::default()` = base / N (`--slot-rule base`); the
//! fixed Rs 20 L / N slot is the BESIDE run (`--slot-rule fixed`). The rule is named on the last stdout line. D-W4-2: S1's
//! `MUHURAT-LIST.tsv` (in `--s1`) is loaded into the session facts (a carried exit rest skips a Muhurat session).
//! RESUMED WAVE 4: `--newpx PACK` prices every leg on the PRODUCTION source (`PACK/newpxF-<IX>` day files + every month's
//! `DP-NEWPX-FAILS-<IX>.tsv`, the DATA law of `opt_pos::newpx`); each leg spanning a hole adds an `H` row; the counts of how the
//! contract-days were made close the summary. Without it the output is the store source's, byte for byte.
//! `--times FILE` writes each account run's legs, fill rows and the wall micros of `pos::run` (the p50 / p99 measurement; the
//! first runs of a book include its store reads, later ones read the cache).

#![allow(clippy::too_many_lines, reason = "one driver")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use idx_cal::day;
use idx_cal::storeio;
use opt_pos::chain::Folders;
use opt_pos::freeze::FreezeTable;
use opt_pos::ladder::Ladder;
use opt_pos::pos::{self, Book, OpenRule, RunOut, SlotRule, Spec, Stream};
use opt_pos::sess::Sessions;

fn main() {
    if let Err(e) = run() {
        eprintln!("g-roll: {e}");
        std::process::exit(2);
    }
}

fn ts(t: i64) -> String {
    if t == 0 {
        return "-".to_string();
    }
    let (d, sod) = storeio::ist_of(t);
    format!("{} {}", day::fmt(d), storeio::hms(sod))
}

fn hms(s: &str) -> Result<i64, String> {
    let p: Vec<i64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
    match p.as_slice() {
        [h, m, x] => Ok(h * 3600 + m * 60 + x),
        _ => Err(format!("time {s}")),
    }
}

fn write_run(t: &mut String, run: &str, r: &RunOut) {
    for (si, o) in r.streams.iter().enumerate() {
        for (li, l) in o.legs.iter().enumerate() {
            let p = l.pick.clone().unwrap_or_default();
            let _ = writeln!(
                t,
                "L\t{run}\t{si}\t{li}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                l.kind,
                day::fmt(l.day),
                ts(l.decided),
                l.folder,
                l.strike,
                l.lot,
                l.freeze,
                l.max_order,
                l.want,
                l.units,
                l.value,
                l.unpaid,
                l.slices,
                l.limit.map_or("-".to_string(), |x| x.to_string()),
                l.budget.map_or("-".to_string(), |x| x.to_string()),
                l.free,
                ts(l.e0),
                ts(l.known_at),
                p.spot,
                p.depth,
                p.reference,
                p.ref_age_s,
                p.skipped_no_trade,
                p.skipped_big,
                p.qty_a2
            );
            if l.hole {
                let _ = writeln!(t, "H\t{run}\t{si}\t{li}");
            }
            for f in &l.fills {
                let _ = writeln!(t, "F\t{run}\t{si}\t{li}\t{}\t{}\t{}\t{}\t{}", ts(f.0), f.1, f.2, f.3, ts(f.4));
            }
        }
        for (ti, x) in o.trips.iter().enumerate() {
            let sells: Vec<String> = x.sells.iter().map(|s| format!("{}:{}:{}", day::fmt(s.0), s.1, s.2)).collect();
            let _ = writeln!(
                t,
                "T\t{run}\t{si}\t{ti}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                x.folder,
                day::fmt(x.buy_day),
                x.units_in,
                x.value_in,
                if sells.is_empty() { "-".to_string() } else { sells.join(",") },
                x.leftover,
                ts(x.w0),
                x.bullet1_px,
                x.settle_close,
                x.intrinsic,
                u8::from(x.settled),
                x.net_settle,
                x.net_bullet1,
                x.charges,
                x.net,
                x.net_dated,
                x.net_per_leg,
                ts(x.closed_at)
            );
        }
        let fl: Vec<String> = o.flags.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let _ = writeln!(t, "G\t{run}\t{si}\t{}\t{}\t{}\t{}", o.label, if fl.is_empty() { "-".to_string() } else { fl.join(",") }, o.net, o.net_per_leg);
    }
    let _ = writeln!(t, "N\t{run}\t{}", r.net);
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let mut kv: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 1;
    while let (Some(k), Some(v)) = (a.get(i), a.get(i + 1)) {
        kv.insert(k.trim_start_matches("--").to_string(), v.clone());
        i += 2;
    }
    let g = |k: &str| kv.get(k).cloned().ok_or_else(|| format!("--{k} is required"));
    let root = PathBuf::from(g("store")?);
    let s1 = PathBuf::from(g("s1")?);
    let rd = |p: &PathBuf| fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let ladder_text = rd(&PathBuf::from(g("ladder")?))?;
    let freeze_text = rd(&PathBuf::from(g("freeze")?))?;
    let sess_text = rd(&s1.join("FNO-END-ROWS.tsv"))?;
    let muhurat_text = rd(&s1.join("MUHURAT-LIST.tsv"))?;
    let halts_text = rd(&PathBuf::from(g("halts")?))?;
    let weeks = rd(&PathBuf::from(g("weeks")?))?;
    let only: Option<Vec<String>> = kv.get("only").map(|s| s.split(',').map(str::to_string).collect());
    let times_path = kv.get("times").map(PathBuf::from);
    let mut times = String::from("run\tlegs\tfills\tmicros\n");
    let slot_rule = match kv.get("slot-rule").map(String::as_str) {
        None => SlotRule::default(),
        Some("base") => SlotRule::BaseOverN,
        Some("fixed") => SlotRule::Fixed,
        Some(x) => return Err(format!("--slot-rule {x}: base (the headline, default) | fixed (beside)")),
    };
    let newpx_pack = kv.get("newpx").map(PathBuf::from);
    let mut books: BTreeMap<String, Book> = BTreeMap::new();
    for ix in ["NIFTY", "BANKNIFTY"] {
        let mut b = Book::new(
            ix,
            pos::parse_cal(&rd(&s1.join(format!("DP-IDXCAL-{ix}.tsv")))?)?,
            pos::parse_series(&rd(&s1.join(format!("IDXCAL-SERIES-{ix}.tsv")))?)?,
            Ladder::parse(&ladder_text)?,
            Folders::list(&root, ix)?,
            FreezeTable::parse(&freeze_text)?,
            pos::parse_close(&rd(&s1.join(format!("INDEX-CLOSE-{ix}.tsv")))?)?,
            Sessions::parse(&sess_text, &halts_text)?.with_muhurat(&muhurat_text)?,
            Some(root.clone()),
        );
        if let Some(pk) = &newpx_pack {
            let dir = pk.join(format!("newpxF-{ix}"));
            let mut months: Vec<PathBuf> = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok).map(|e| e.path()).collect();
            months.sort();
            let mut fl = opt_pos::newpx::Fails::default();
            for m in &months {
                let f = m.join(format!("DP-NEWPX-FAILS-{ix}.tsv"));
                if f.exists() {
                    fl.add(&rd(&f)?)?;
                }
            }
            b = b.with_newpx(pos::NewpxSrc::new(dir, fl));
        }
        books.insert(ix.to_string(), b);
    }
    let names: Vec<String> = books.keys().cloned().collect();
    let blist: Vec<Book> = names.iter().filter_map(|n| books.remove(n)).collect();
    let bidx = |ix: &str| names.iter().position(|n| n == ix).ok_or_else(|| format!("index {ix}"));
    let mut t = String::from("# g-roll rows (S8 G-ROLL, opt_pos::pos on the frozen store's 1-s option bars). L = leg, F = fill row, T = trip, G = stream flags and nets, N = the account net. Times IST.\n");
    let mut summary = String::new();
    for l in weeks.lines() {
        if l.starts_with('#') || l.starts_with("id\t") || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let col = |k: usize| c.get(k).copied().ok_or_else(|| format!("weeks row {l}: column {k}"));
        let id = col(0)?;
        if only.as_ref().is_some_and(|o| !o.iter().any(|x| x == id)) {
            continue;
        }
        let mixed = col(1)? == "mixed";
        let streams_txt = col(2)?;
        let entry_day = day::parse(col(3)?).ok_or("entry day")?;
        let entry = storeio::at_sod(entry_day, hms(col(4)?)?);
        let exit_day = day::parse(col(5)?).ok_or("exit day")?;
        let exit = storeio::at_sod(exit_day, hms(col(6)?)?);
        let n: i64 = col(7)?.parse().map_err(|_| "n")?;
        let bp: u16 = col(8)?.parse().map_err(|_| "bp")?;
        let account = 200_000_000i64;
        let mk = |s: &str| -> Result<Stream, String> {
            let (ix, side) = s.split_once(':').ok_or("stream ix:side")?;
            Ok(Stream { book: bidx(ix)?, ce: side == "CE", entry, exit, slot: account / n, label: s.to_string() })
        };
        let groups: Vec<(String, Vec<Stream>)> = if mixed {
            vec![("MIX".to_string(), streams_txt.split(',').map(mk).collect::<Result<Vec<_>, _>>()?)]
        } else {
            streams_txt.split(',').map(|s| mk(s).map(|x| (s.replace(':', "-"), vec![x]))).collect::<Result<Vec<_>, _>>()?
        };
        for (gname, streams) in groups {
            for (rule, rname) in [(OpenRule::Sequential, "seq"), (OpenRule::SameDecision, "same")] {
                let mut nets = [0i64; 2];
                for (pi, kept) in [true, false].into_iter().enumerate() {
                    let run_id = format!("{id}-{gname}-{}-{rname}", if kept { "kept" } else { "off" });
                    let spec = Spec { streams: streams.clone(), account, bp, kept, open_rule: rule, target: 50_000_000, slot_rule };
                    let t0 = std::time::Instant::now();
                    match pos::run(&blist, &spec) {
                        Ok(r) => {
                            let us = t0.elapsed().as_micros();
                            let legs: usize = r.streams.iter().map(|o| o.legs.len()).sum();
                            let fills: usize = r.streams.iter().flat_map(|o| o.legs.iter()).map(|l| l.fills.len()).sum();
                            let _ = writeln!(times, "{run_id}\t{legs}\t{fills}\t{us}");
                            write_run(&mut t, &run_id, &r);
                            if let Some(x) = nets.get_mut(pi) {
                                *x = r.net;
                            }
                        }
                        Err(e) => {
                            let _ = writeln!(t, "E\t{run_id}\t{e}");
                            let _ = writeln!(summary, "{run_id}: ERROR {e}");
                        }
                    }
                }
                let booked = if nets.get(1) < nets.first() { "off" } else { "kept" };
                let _ = writeln!(summary, "{id} {gname} {rname}: net kept {} off {} booked {booked}", nets.first().copied().unwrap_or(0), nets.get(1).copied().unwrap_or(0));
                let _ = writeln!(t, "B\t{id}-{gname}-{rname}\t{booked}\t{}\t{}", nets.first().copied().unwrap_or(0), nets.get(1).copied().unwrap_or(0));
            }
        }
    }
    let out = PathBuf::from(g("out")?);
    fs::write(&out, &t).map_err(|e| e.to_string())?;
    if let Some(p) = times_path {
        fs::write(&p, &times).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    if newpx_pack.is_some() {
        for (n, b) in names.iter().zip(&blist) {
            let c = b.newpx().map(|s| s.counts.borrow().clone()).unwrap_or_default();
            let _ = writeln!(summary, "production source {n}: contract-days by how they were made {c:?}");
        }
    }
    let _ = writeln!(summary, "slot rule: {} (W4 REPAIR D-W4-3: --slot-rule base is the headline default, fixed beside)", slot_rule.name());
    print!("{summary}");
    Ok(())
}
