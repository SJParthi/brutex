//! `itm-pick` (S5, gate G-EQ3's Rust side): the A2 / s23a ITM-fit contract at the 1-min family's candle-end instants
//! (09:16:00 .. 15:07:00) of the asked index-days, both sides, with the current expiry as known from DP-IDXCAL (S1's
//! `DP-IDXCAL-<IX>.tsv`: the file read, the listed alias and the series' final folder date), the ladder of the file dated
//! before the session (C27), lots C17 / D9 per candidate, C11 closest of Rs 5,00,000 (A1), the dated one-order maximum
//! (12:06 note) and, on true expiry days, the NEXT expiry's pick at 15:00:00 (the roll's 15:00 instant; the sequential t2
//! re-read is S8's). Writes one row per (day, side, instant).
//!
//! `itm-pick --store ROOT --ladder F --freeze F --cal DIR --index IX --out FILE DAY...`

#![allow(clippy::too_many_lines, reason = "one driver")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use opt_pos::chain::{self, ItmCand};
use opt_pos::freeze::FreezeTable;
use opt_pos::ladder::Ladder;
use opt_pos::secs::{self, Sec};

const TARGET: i64 = 50_000_000;

struct CalRow {
    file_read: Day,
    known_listed: Day,
    known_eff: Day,
    next_eff: Option<Day>,
    d_exp: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("itm-pick: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let (mut root, mut lad, mut fz, mut cal, mut ix, mut out) = (PathBuf::new(), PathBuf::new(), PathBuf::new(), PathBuf::new(), String::new(), PathBuf::new());
    let mut days: Vec<Day> = Vec::new();
    let mut i = 1;
    while let Some(k) = a.get(i) {
        let v = || a.get(i + 1).cloned().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--store" => root = PathBuf::from(v()?),
            "--ladder" => lad = PathBuf::from(v()?),
            "--freeze" => fz = PathBuf::from(v()?),
            "--cal" => cal = PathBuf::from(v()?),
            "--index" => ix = v()?,
            "--out" => out = PathBuf::from(v()?),
            _ => {
                days.push(day::parse(k).ok_or_else(|| format!("day {k}"))?);
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    let ladder = Ladder::parse(&fs::read_to_string(&lad).map_err(|e| e.to_string())?)?;
    let freeze = FreezeTable::parse(&fs::read_to_string(&fz).map_err(|e| e.to_string())?)?;
    let mut calrows: BTreeMap<Day, CalRow> = BTreeMap::new();
    for l in fs::read_to_string(cal.join(format!("DP-IDXCAL-{ix}.tsv"))).map_err(|e| e.to_string())?.lines() {
        if l.starts_with('#') || l.starts_with("session") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let p = |k: usize| c.get(k).and_then(|x| day::parse(x));
        if let (Some(s), Some(f), Some(kl), Some(ke)) = (p(0), p(2), p(3), p(4)) {
            calrows.insert(s, CalRow { file_read: f, known_listed: kl, known_eff: ke, next_eff: p(6), d_exp: c.get(9) == Some(&"1") });
        }
    }
    // the series' final folder date: the listed alias -> the folder named by the final alias (IDXCAL-SERIES)
    let mut final_of: BTreeMap<Day, Vec<(Day, Day, Day)>> = BTreeMap::new(); // alias date -> (first file, last file, final date)
    for l in fs::read_to_string(cal.join(format!("IDXCAL-SERIES-{ix}.tsv"))).map_err(|e| e.to_string())?.lines() {
        if l.starts_with('#') || l.starts_with("series") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let als: Vec<(Day, Day, Day)> = c
            .get(1)
            .unwrap_or(&"")
            .split(" -> ")
            .filter_map(|a| Some((day::parse(a.get(0..10)?)?, day::parse(a.get(12..22)?)?, day::parse(a.get(24..34)?)?)))
            .collect();
        let Some(fin) = als.last().map(|x| x.0) else { continue };
        for (dt, f0, f1) in als {
            final_of.entry(dt).or_default().push((f0, f1, fin));
        }
    }
    let folder_date = |alias: Day, file: Day| -> Day { final_of.get(&alias).and_then(|v| v.iter().find(|(f0, f1, _)| *f0 <= file && file <= *f1)).map_or(alias, |x| x.2) };
    let fo = chain::Folders::list(&root, &ix)?;
    let mut t = String::from("# itm-pick rows: day side instant expiry_listed expiry_folder spot pick strike depth qty reference ref_age_s max_order lot skipped_no_trade skipped_big\nday\tside\tinstant\tlisted\tfolder\tspot\tpick\tstrike\tdepth\tqty\treference\tref_age_s\tmax_order\tlot\tskip_no_trade\tskip_big\n");
    let mut n_none = 0usize;
    let mut n = 0usize;
    for &d in &days {
        let Some(cr) = calrows.get(&d) else {
            let _ = writeln!(t, "{}\t-\t-\tno DP-IDXCAL row", day::fmt(d));
            continue;
        };
        let spot = secs::spot_day(&root, &ix, d)?;
        let mut jobs: Vec<(Day, Day, Vec<i64>)> = Vec::new(); // (listed alias, file, instants)
        let inst: Vec<i64> = (1..=352).map(|m| at_sod(d, 9 * 3600 + 15 * 60 + 60 * m)).collect();
        jobs.push((cr.known_listed, cr.file_read, inst));
        if cr.d_exp
            && cr.known_eff == d
            && let Some(ne) = cr.next_eff
        {
            // the next expiry's listed alias in the file read (the effective date equals the listed one unless shifted)
            let alias = ladder_alias(&ladder, &ix, cr.file_read, ne).unwrap_or(ne);
            jobs.push((alias, cr.file_read, vec![at_sod(d, 15 * 3600)]));
        }
        for (alias, file, instants) in jobs {
            let Some(row) = ladder.get(&ix, file, alias) else {
                let _ = writeln!(t, "{}\t-\t-\t{}\t-\tno ladder row in file {}", day::fmt(d), day::fmt(alias), day::fmt(file));
                continue;
            };
            let fdate = folder_date(alias, file);
            for ce in [true, false] {
                let strikes = if ce { &row.ce } else { &row.pe };
                let mut loaded: Vec<(i64, String, Vec<Sec>)> = Vec::new();
                for &k in strikes {
                    // F-e (RESUMED WAVE 4): a listed strike with no store folder (or no rows that day) stays a candidate with no
                    // rows, so the ITM rule counts it as a no-trade skip (plan text; m_fe_drop_ghosts drops it again)
                    let name = storeio::contract_name(fdate, k, ce);
                    let rows = if fo.has(fdate, k, ce) { secs::option_day(&root, &ix, &name, d)? } else { None };
                    match rows {
                        Some(rows) => loaded.push((k, name, rows)),
                        None if !cfg!(feature = "m_fe_drop_ghosts") => loaded.push((k, String::new(), Vec::new())),
                        None => {}
                    }
                }
                for &tt in &instants {
                    let Some(sp) = secs::last_before(&spot, tt).map(|s| s.close) else { continue };
                    n += 1;
                    // each candidate's own lot (C17 / D9 over its prints before t: the rule is per contract)
                    let cands: Vec<ItmCand<'_>> = loaded
                        .iter()
                        .map(|(k, nm, r)| (*k, nm.as_str(), r.as_slice(), u64::try_from(chain::lot_at(row, &r.iter().filter(|x| x.ts < tt).map(|x| x.vol).collect::<Vec<_>>()).0.max(0)).unwrap_or(0)))
                        .collect();
                    let fu = freeze.units(&ix, d).unwrap_or(0);
                    let picked = (fu > 0).then(|| chain::itm_fit(&cands, ce, sp, tt, TARGET, None, fu));
                    let side = if ce { "CE" } else { "PE" };
                    let (h, m, s) = { let (_, sod) = storeio::ist_of(tt); (sod / 3600, (sod / 60) % 60, sod % 60) };
                    let _ = write!(t, "{}\t{side}\t{h:02}:{m:02}:{s:02}\t{}\t{}\t{sp}\t", day::fmt(d), day::fmt(alias), day::fmt(fdate));
                    match picked {
                        Some(Ok(p)) => {
                            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", p.folder, p.strike, p.depth, p.qty, p.reference, p.ref_age_s, p.max_order, p.lot, p.skipped_no_trade, p.skipped_big);
                        }
                        Some(Err(e)) => {
                            n_none += 1;
                            let _ = writeln!(t, "{e}\t-\t-\t-\t-\t-\t-\t-\t-\t-");
                        }
                        None => {
                            n_none += 1;
                            let _ = writeln!(t, "no_freeze_row\t-\t-\t-\t-\t-\t-\t-\t-\t-");
                        }
                    }
                }
            }
        }
    }
    let _ = writeln!(t, "# instants {n}; no pick {n_none}");
    fs::write(&out, &t).map_err(|e| e.to_string())?;
    println!("itm-pick {ix}: days {}; instants {n}; no pick {n_none}", days.len());
    Ok(())
}

/// The listed alias in `file` whose effective date is `eff` (the alias itself when listed).
fn ladder_alias(l: &Ladder, ix: &str, file: Day, eff: Day) -> Option<Day> {
    if l.get(ix, file, eff).is_some() {
        return Some(eff);
    }
    (1..=3).map(|k| eff + k).find(|x| l.get(ix, file, *x).is_some())
}
