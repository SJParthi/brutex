//! spikefade: SNIPER s25b spike-fade option SELLING from the Top Volume screener (LABELLED DIAGNOSTIC, 8 Tickvault days).
//!
//! Commands:
//!   spikefade cube <stage dir> <cube dir> <YYYY-MM-DD>   build (or keep, when its inputs are unchanged) one day's cube + seconds tape
//!   spikefade run <cube dir> <out dir> <days...>          the screener, events, trade engine, account sweep, top picks, twin, reports
//!   spikefade aggcheck <stage dir> <cube dir> <day>       our 1m-fold timeframes against Tickvault's own 3/5/10/15/30/60m files
#![allow(clippy::too_many_arguments, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss, clippy::cast_possible_wrap)]

mod acct;
mod cal;
mod charges;
mod csvgz;
mod cube;
mod margin;
// v3.1 (MAIN-RULINGS-ENGINE-v22 + the operator's decisions of 15 Nov)
mod mtable;
mod stress;
mod sw;
#[cfg(test)]
mod tests31;
mod report;
mod rules;
mod screen;
mod trade;
mod v2;
// v3 (EXITS-DYNAMIC as signed with the critic's R1-R6 + the liquidity dial)
mod kern;
mod menu;
mod p3;
mod read;
mod report3;
mod stage;
mod wf;
#[cfg(test)]
mod tests3;
// v3.2 (MAIN-RULINGS-ENGINE-v22 addenda 8b, 10-16; the v3.1 check M1-M4 / S1-S8; the checklist audit's fixes)
mod ban;
mod bhav;
mod expiry;
mod freeze;
mod gk;
mod v32;
// v3.2 REPAIR A: day-major checkpoint and resume (addendum 20.3)
mod ckpt;
// v3.2 round B (MAIN-RULINGS-ENGINE-v22 addenda 17 + 18; ported from v3.2-B, job v32-par, Addendum 21.1)
mod side17;
mod split18;
#[cfg(test)]
mod tests32;
// v3.2 REPAIR A (check-engine-1 / check-engine-2 of 15 Nov)
#[cfg(test)]
mod tests32r;
// v3.2 round B (addenda 17 + 18 on repair A)
#[cfg(test)]
mod tests32rb;
// v3.3 (Addendum 28.2, job v33-fast): the exact speed switches of v32/speed/SPEED-PLAN.md
mod sw33;
// v3.3 integrate (Addendum 28.2, integrate:v33): the BLOCKING fixes of the re-checks check-engine-1r / check-engine-2r (switches, off by default)
mod fix33;
#[cfg(test)]
mod tests33f;
// v3.3 fix:v33 (Addenda 24-28): the BLOCKING items of the v3.3 gates and the re-check (switches, off by default)
mod fixg;
#[cfg(test)]
mod tests33g;
// v3.4 round D build:D-high (Addendum 30): the HIGH items that can change a result + the quick result-neutral ones (switches, off by default)
mod fix34;
#[cfg(test)]
mod tests34;
#[cfg(test)]
mod tests33;
// v3.3 SEARCH-1D (MAIN-RULINGS-ENGINE-v22 Addendum 28.1, search:v33): the every-year search set's sets file (role SEARCH1D)
#[cfg(test)]
mod tests33s;
// v3.4 (build:shard, MAIN-RULINGS-ENGINE-v22 Addendum 30.3): the cloud shard kit (cloud-kit/CLOUD-KIT-DESIGN.md E1-E6, E8, E9, E12 + the
// critic's F1-F5); every switch off by default (v3.3 final 8d9a5b67 byte for byte)
mod ckmerge;
mod kit;
mod slice;
mod synth;
#[cfg(test)]
mod tests34s;
// v3.4 integrate:D (round D, Addendum 30.2): data:F3's dated stock freeze reader and data:F7's bands / protection model (switches, off by
// default: build:D-high's v3.4 byte for byte)
mod f7;
mod int34;
#[cfg(test)]
mod tests34i;

use std::path::PathBuf;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("cube") => cmd_cube(&a),
        Some("run") => report::cmd_run(&a),
        Some("aggcheck") => screen::cmd_aggcheck(&a),
        Some("run3") => report3::cmd_run3(&a),
        Some("p3") => p3::cmd_p3(&a),
        Some("margincheck") => mtable::cmd_margincheck(&a),
        Some("margin-one") => mtable::cmd_margin_one(&a),
        Some("margincheck2") => mtable::cmd_margincheck2(&a),
        // v3.2: the engine's Greeks module on the runner's greeks-tsv contract (the 8b proof feeds it the live app's inputs)
        Some("greeks-tsv") => gk::cmd_greeks_tsv(),
        // v3.2 addendum 16: the dated expiry calendar of NIFTY / BANKNIFTY from NSE's F&O bhavcopies
        Some("expiry-cal") => v32::cmd_expiry_cal(&a),
        // v3.2 REPAIR A (check-engine-2 M3): the recorded option-bar span of given days (the dated special-session table's slots)
        Some("special-spans") => cmd_special_spans(&a),
        // v3.2 round B (addendum 17): the side-by-side account across two separate runs (index and stock options)
        Some("side17-combine") => side17::cmd_combine(a.get(2..).unwrap_or(&[])).map(|()| "side17-combine: SIDE17-COMBINED.md and side17-walk.tsv written".to_owned()),
        // v3.4 shard kit: E3 merge, the canonical checkpoint (P-S3), the batch-file check, E8 / E9 slices, E12 synthetic days
        Some("ckpt-merge") => ckmerge::cmd_ckpt_merge(&a),
        Some("ckpt-canon") => ckmerge::cmd_ckpt_canon(&a),
        Some("bck-check") => ckmerge::cmd_bck_check(&a),
        Some("cube-slice") => slice::cmd_cube_slice(&a),
        Some("synth") => synth::cmd_synth(&a),
        _ => Err("usage: spikefade cube|run|run3|p3|aggcheck|margincheck|margin-one|margincheck2|greeks-tsv|expiry-cal|ckpt-merge|ckpt-canon|bck-check|cube-slice|synth ...".to_owned()),
    };
    match r {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("REFUSED: {e}");
            std::process::exit(2);
        }
    }
}

fn cmd_cube(a: &[String]) -> Result<String, String> {
    let (Some(stage), Some(out), Some(d)) = (a.get(2), a.get(3), a.get(4)) else { return Err("cube <stage> <out> <day>".to_owned()) };
    let (stage, out) = (PathBuf::from(stage), PathBuf::from(out));
    let force = a.iter().any(|x| x == "--force");
    let (cur, why) = cube::is_current(&out, &stage, d);
    if cur && !force {
        return Ok(format!("{d}: cube current ({why}); not rebuilt (incremental)"));
    }
    let log = cube::build(&stage, &out, d)?;
    Ok(format!("{log} [rebuilt: {why}]"))
}

/// `spikefade special-spans <cube dir> <days...>`: per day the first and last 1-minute slot (09:15 + slot) holding an option bar, and the
/// minutes between (the v3.1 `Cal3::add_specials` rule, read once to write the DATED special-session table; read only on the cubes).
fn cmd_special_spans(a: &[String]) -> Result<String, String> {
    let dir = a.get(2).ok_or("special-spans <cube dir> <days...>")?;
    let mut out = String::from("day\tfirst_slot\tlast_slot\tminutes\tfirst_bar\tlast_bar_end\tminutes_with_a_bar\n");
    for d in a.iter().skip(3) {
        let c = cube::load(std::path::Path::new(dir), d)?;
        let (mut lo, mut hi) = (usize::MAX, 0_usize);
        let mut seen = vec![false; cal::NM];
        for (i, f) in c.flags.iter().enumerate() {
            if f & 1 == 1 {
                lo = lo.min(i % cal::NM);
                hi = hi.max(i % cal::NM);
                if let Some(x) = seen.get_mut(i % cal::NM) {
                    *x = true;
                }
            }
        }
        let nseen = seen.iter().filter(|&&x| x).count();
        if lo <= hi {
            let m = i64::try_from(hi + 1 - lo).unwrap_or(0);
            out.push_str(&format!("{d}\t{lo}\t{hi}\t{m}\t{}\t{}\t{nseen}\n", cal::fmt_sod(cal::OPEN_S + 60 * i64::try_from(lo).unwrap_or(0)), cal::fmt_sod(cal::OPEN_S + 60 * i64::try_from(hi + 1).unwrap_or(0))));
        } else {
            out.push_str(&format!("{d}\t-\t-\t0\t-\t-\n"));
        }
    }
    Ok(out)
}
