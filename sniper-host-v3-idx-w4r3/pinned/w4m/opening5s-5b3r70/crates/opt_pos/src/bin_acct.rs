//! `idx-acct` (S6 positional half, wave 4): the one account re-walk (`opt_pos::acct`) on real stored decisions (the idx1s trade
//! files: each file, or each side of a file, is one stream), with the proofs of plan s5.2:
//! * P-ACCT1: every stream re-walked ALONE equals its stored rows (the stream priced in its own account) byte for byte;
//! * P-ACCT2 (Rust side): the streams together in ONE account (N per stream, Rs 20 L) with the forced `cash_short` cases, every
//!   booked row written in full for the independent node re-walk (`w4/s6/node/p-acct2.mjs`).
//!
//! `idx-acct --store ROOT --freeze F --n N --out DIR [--split-sides] STREAM_FILE...`

#![allow(clippy::too_many_lines, reason = "one driver")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use opt_pos::acct::{self, Account, Booked, Decision};
use opt_pos::freeze::FreezeTable;
use opt_pos::secs::{self, Sec};

fn hms(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
    Some(p.first()? * 3600 + p.get(1)? * 60 + p.get(2)?)
}
fn stamp(s: &str) -> Option<i64> {
    Some(at_sod(day::parse(s.get(0..10)?)?, hms(s.get(11..19)?)?))
}
fn rs(s: &str) -> Option<i64> {
    let neg = s.starts_with('-');
    let t: String = s.trim_start_matches('-').trim_start_matches("Rs ").chars().filter(|c| *c != ',').collect();
    let (a, b) = t.split_once('.').unwrap_or((&t, "0"));
    let p = a.parse::<i64>().ok()? * 100 + format!("{b:0<2}").get(0..2)?.parse::<i64>().ok()?;
    Some(if neg { -p } else { p })
}
fn ist(us: i64) -> String {
    let (d, s) = storeio::ist_of(us);
    format!("{} {}", day::fmt(d), storeio::hms(s))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("idx-acct: {e}");
        std::process::exit(2);
    }
}

fn decisions(path: &PathBuf, freeze: &FreezeTable) -> Result<Vec<Decision>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut th: Vec<String> = Vec::new();
    let mut v = Vec::new();
    for l in text.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        match c.first().copied() {
            Some("#trade") => th = c.iter().map(|x| (*x).to_owned()).collect(),
            Some("trade") => {
                let g = |n: &str| th.iter().position(|h| h == n).and_then(|k| c.get(k)).copied().unwrap_or("");
                let Some(d) = day::parse(g("date")) else { continue };
                let ix = g("index").to_owned();
                let lot: u64 = g("lot").parse().unwrap_or(0);
                let xd = g("exit_decision");
                let exit = if xd.len() >= 19 && g("late_exit_rejected") != "true" { stamp(xd) } else { None };
                let li = i64::try_from(lot).unwrap_or(0);
                v.push(Decision {
                    no: g("no").to_owned(),
                    d,
                    t: stamp(g("decision_t")).ok_or("decision_t")?,
                    contract: g("contract").to_owned(),
                    lot,
                    reference: rs(g("r_high")).ok_or("r_high")?,
                    buy_lim: rs(g("buy_protection_limit")).ok_or("buy_protection_limit")?,
                    exit,
                    bp: g("share_level").trim_end_matches('%').parse::<u16>().unwrap_or(0) * 100,
                    freeze: u64::try_from(freeze.units(&ix, d).unwrap_or(0)).unwrap_or(0),
                    max_order: u64::try_from(freeze.max_one_order(&ix, d, li).unwrap_or(0)).unwrap_or(0),
                    ix,
                });
            }
            _ => {}
        }
    }
    Ok(v)
}

/// Every field of a booked row, every fill of its booked path (the comparison unit of P-ACCT1 / P-ACCT2).
fn row_text(b: &Booked, dc: &Decision) -> String {
    let g = b.priced.paths.get(b.priced.booked).cloned().unwrap_or_default();
    let mut s = format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        b.stream,
        b.no,
        day::fmt(dc.d),
        ist(dc.t),
        dc.contract,
        b.want,
        b.qty,
        u8::from(b.cash_short),
        u8::from(b.refused),
        b.reserve,
        g.units_in,
        g.value_in,
        g.net,
        if b.priced.booked == 0 { "kept" } else { "off" },
        ist(b.release),
        g.fills.len()
    );
    for f in &g.fills {
        let _ = write!(s, "\t{:?}:{}:{}:{}:{}:{}", f.0, ist(f.1), f.2, f.3, f.4, ist(f.5));
    }
    s
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let (mut root, mut fz, mut out, mut n, mut split) = (PathBuf::new(), PathBuf::new(), PathBuf::new(), 1i64, false);
    let mut files: Vec<PathBuf> = Vec::new();
    let mut i = 1;
    while let Some(k) = a.get(i) {
        let v = || a.get(i + 1).cloned().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--store" => root = PathBuf::from(v()?),
            "--freeze" => fz = PathBuf::from(v()?),
            "--out" => out = PathBuf::from(v()?),
            "--n" => n = v()?.parse().map_err(|_| "--n")?,
            "--split-sides" => {
                split = true;
                i += 1;
                continue;
            }
            _ => {
                files.push(PathBuf::from(k));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let freeze = FreezeTable::parse(&fs::read_to_string(&fz).map_err(|e| e.to_string())?)?;
    let mut streams: Vec<Vec<Decision>> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for f in &files {
        let v = decisions(f, &freeze)?;
        let nm = f.display().to_string();
        if split {
            for side in ["CE", "PE"] {
                streams.push(v.iter().filter(|x| x.contract.ends_with(side)).cloned().collect());
                names.push(format!("{nm} [{side}]"));
            }
        } else {
            streams.push(v);
            names.push(nm);
        }
    }
    let mut cache: BTreeMap<(String, Day), Vec<Sec>> = BTreeMap::new();
    let mut rows_of = |ix: &str, c: &str, d: Day| -> Result<Vec<Sec>, String> {
        if !cache.contains_key(&(c.to_owned(), d)) {
            if cache.len() > 400 {
                cache.clear();
            }
            cache.insert((c.to_owned(), d), secs::option_day(&root, ix, c, d)?.unwrap_or_default());
        }
        Ok(cache.get(&(c.to_owned(), d)).cloned().unwrap_or_default())
    };
    let base0 = 200_000_000i64;
    let mut stored: Vec<Vec<Booked>> = Vec::new();
    let mut rep = String::new();
    let mut st_txt = String::new();
    let (mut p1_rows, mut p1_diff) = (0usize, 0usize);
    for (s, v) in streams.iter().enumerate() {
        let mut one = acct::single(v, base0, n, &mut rows_of)?;
        for b in &mut one {
            b.stream = s;
        }
        // P-ACCT1: the stream re-walked alone (the general event machinery) equals its stored rows byte for byte
        let alone = acct::rewalk(std::slice::from_ref(v), std::slice::from_ref(&one), &Account { base0, n: vec![n] }, &mut rows_of)?;
        for ((x, y), dc) in one.iter().zip(&alone).zip(v) {
            p1_rows += 1;
            let mut y2 = y.clone();
            y2.stream = s;
            if row_text(x, dc) != row_text(&y2, dc) || !y.reused {
                p1_diff += 1;
            }
            let _ = writeln!(st_txt, "{}", row_text(x, dc));
        }
        let _ = writeln!(rep, "stream {s}: {} ({} decisions; stored in its own account at N {n}: cash_short {}, refused {})", names.get(s).map_or("", String::as_str), v.len(), one.iter().filter(|b| b.cash_short).count(), one.iter().filter(|b| b.refused).count());
        stored.push(one);
    }
    let _ = writeln!(rep, "P-ACCT1 (each stream re-walked alone = its stored rows, every field and fill, all reused): rows {p1_rows}, differences {p1_diff} -> {}", if p1_diff == 0 { "PASS" } else { "FAIL" });
    fs::write(out.join("STORED.tsv"), &st_txt).map_err(|e| e.to_string())?;
    let acc = Account { base0, n: vec![n; streams.len()] };
    let all = acct::rewalk(&streams, &stored, &acc, &mut rows_of)?;
    let mut rw = String::new();
    for b in &all {
        let dc = streams.get(b.stream).and_then(|v| v.iter().find(|x| x.no == b.no)).ok_or("decision")?;
        let _ = writeln!(rw, "{}", row_text(b, dc));
    }
    fs::write(out.join("REWALK.tsv"), &rw).map_err(|e| e.to_string())?;
    let _ = writeln!(
        rep,
        "one account (Rs 20 L, N {n} a stream, {} streams): {} entries; reused {}; re-priced {} (cash_short {}); refused {}; net Rs {}.{:02}",
        streams.len(),
        all.len(),
        all.iter().filter(|b| b.reused).count(),
        all.iter().filter(|b| !b.reused && !b.refused).count(),
        all.iter().filter(|b| b.cash_short).count(),
        all.iter().filter(|b| b.refused).count(),
        all.iter().map(Booked::net).sum::<i64>() / 100,
        (all.iter().map(Booked::net).sum::<i64>() % 100).abs()
    );
    fs::write(out.join("ACCT-REPORT.txt"), &rep).map_err(|e| e.to_string())?;
    print!("{rep}");
    Ok(())
}
