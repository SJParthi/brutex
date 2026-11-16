//! `sniper-q`: O(1) lookups in a run's partitions (plan 13): `sniper-q DB_DIR PART IX SESS K [--data DIR]` prints the record of
//! (session, candle) through the offset index; `sniper-q DB_DIR PART - REC [--data DIR]` prints record number REC.
//!
//! HOST REPAIR B3 (check-build-host-v1 section 7): the record width comes from the run's own layout rows (the RUN data rows'
//! `n_grid` and `levels_bp`, the same rows the host priced with) through `price::record_width`: 352 B for every OUT store this
//! host writes (11 N x 5 levels). It is never guessed from the part's name, and a partition whose length is not a whole number of
//! records is refused. Every net is printed with its own N and share level, every N of the grid.

use std::path::PathBuf;

fn main() {
    let mut pos: Vec<String> = Vec::new();
    let mut data: Option<PathBuf> = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == "--data" {
            data = it.next().map(PathBuf::from);
        } else {
            pos.push(a);
        }
    }
    let (Some(dir), Some(part), Some(ix)) = (pos.first(), pos.get(1), pos.get(2)) else {
        eprintln!("usage: sniper-q DB_DIR PART IX|- SESS K | REC [--data DIR]");
        std::process::exit(2);
    };
    let data = data.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data"));
    let layout = sniper_host::rows::read_with_sha(&data.join("SNIPER-RUN-v3.tsv").to_string_lossy())
        .and_then(|(t, _)| sniper_host::rows::RunRows::parse(&t))
        .and_then(|r| Ok((r.list::<i64>("n_grid")?, r.list::<u16>("levels_bp")?)));
    let (n_grid, levels) = match layout {
        Ok(x) => x,
        Err(e) => {
            eprintln!("sniper-q: the RUN rows (n_grid, levels_bp) are unreadable: {e}");
            std::process::exit(2);
        }
    };
    let (nn, nl) = (n_grid.len(), levels.len());
    let width = if cfg!(feature = "m_b3") {
        // MUTANT m_b3 (the v1 sniper-q): the width guessed from the part's name (the matrix test SN-B3 must catch it).
        if part.contains("-N55") { 352 } else { 112 }
    } else {
        sniper_host::price::record_width(nn, nl)
    };
    let d = PathBuf::from(dir);
    let Ok(p) = std::fs::read(d.join(part)) else {
        eprintln!("sniper-q: cannot read {part}");
        std::process::exit(2);
    };
    if !cfg!(feature = "m_b3") && (width == 0 || !p.len().is_multiple_of(width)) {
        eprintln!("sniper-q: {part} holds {} bytes, not a whole number of {width}-byte records ({nn} N x {nl} levels): refused", p.len());
        std::process::exit(2);
    }
    let rec = if ix == "-" {
        pos.get(3).and_then(|x| x.parse::<u32>().ok())
    } else {
        let Ok(i) = std::fs::read(d.join(ix)) else {
            eprintln!("sniper-q: cannot read {ix}");
            std::process::exit(2);
        };
        let (s, k) = (pos.get(3).and_then(|x| x.parse().ok()).unwrap_or(u32::MAX), pos.get(4).and_then(|x| x.parse().ok()).unwrap_or(0));
        sniper_host::query::lookup(&i, s, k)
    };
    match rec.and_then(|r| sniper_host::query::record(&p, width, r).map(|b| (r, b))) {
        Some((r, b)) => {
            let nets: Vec<i32> = b.chunks_exact(4).take(nn * nl).filter_map(|c| c.try_into().ok()).map(i32::from_le_bytes).collect();
            let lv: Vec<String> = levels.iter().map(|x| format!("{}%", x / 100)).collect();
            println!("record {r} of {} ({width} B each; N grid {n_grid:?} x share levels {lv:?}); nets after every Dhan charge, paisa:", p.len() / width.max(1));
            for (ni, n) in n_grid.iter().enumerate() {
                println!("  N={n}: {:?}", nets.get(ni * nl..(ni + 1) * nl).unwrap_or(&[]));
            }
        }
        None => {
            eprintln!("sniper-q: no record");
            std::process::exit(1);
        }
    }
}
