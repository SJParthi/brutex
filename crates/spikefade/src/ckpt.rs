//! v3.2 REPAIR A (addendum 20.3, check-engine-2 S6): CHECKPOINT AND RESUME of a day-major run (`run3 --day-major --checkpoint <dir>`).
//!
//! The unit is a FINISHED BATCH (a set of unique lists whose every account has walked every day of the run): after each batch the segment's
//! accumulated results (the kept accounts of every group and view, the window rows, the flagged account-rows, the per-N booked counts, the
//! steps and accounts) are written to `<dir>/<seg>.ckpt` (atomic: a `.part` file renamed), with a fingerprint of the run (segment, days,
//! settings, lists, batch size, share mask, switches). A resumed run with the same arguments skips the WALKS of every finished batch and
//! prices the finished batches' events once in a union stage A pass (the per-event facts and the stage A counters are deterministic, counted
//! once per event), so its outputs equal an uninterrupted run byte for byte (proof XCKPT). Inside a running batch a reboot redoes at most
//! that batch's walk: month-level account states (about 1 KB an account) are not written.

use std::fmt::Write as _;

use crate::acct::{Sum, Tops};

/// One account summary as one tab-separated line (every field, in declaration order).
#[must_use]
pub fn sum_line(s: &Sum) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        s.seg, s.list, s.x, s.n, s.net, s.trades, s.wins, s.mdd, s.worst, s.worst_day, s.days_pos, s.days_neg, s.days_traded, s.ndays, u8::from(s.spike), s.pk_margin, s.mdd_alt, s.net_s2, s.nc_trades, s.nc_units, s.nc_net, s.mtm_ref, s.tie, s.mo_seen, s.mo_ok, s.mo_min, s.wk_seen, s.wk_pos, s.wk_min
    ) + &if crate::sw::on(crate::sw::SPLITS18) { format!("\t{}", crate::split18::sp_line(&s.sp)) } else { String::new() }
}

fn num<T: std::str::FromStr>(f: &mut std::str::Split<'_, char>) -> Result<T, String> {
    f.next().ok_or("short checkpoint row")?.parse::<T>().map_err(|_| "bad checkpoint number".to_owned())
}

/// The inverse of [`sum_line`].
///
/// # Errors
/// A malformed line.
pub fn sum_of_line(l: &str) -> Result<Sum, String> {
    let mut f = l.split('\t');
    Ok(Sum {
        seg: num(&mut f)?,
        list: num(&mut f)?,
        x: num(&mut f)?,
        n: num(&mut f)?,
        net: num(&mut f)?,
        trades: num(&mut f)?,
        wins: num(&mut f)?,
        mdd: num(&mut f)?,
        worst: num(&mut f)?,
        worst_day: num(&mut f)?,
        days_pos: num(&mut f)?,
        days_neg: num(&mut f)?,
        days_traded: num(&mut f)?,
        ndays: num(&mut f)?,
        spike: num::<u8>(&mut f)? == 1,
        pk_margin: num(&mut f)?,
        mdd_alt: num(&mut f)?,
        net_s2: num(&mut f)?,
        nc_trades: num(&mut f)?,
        nc_units: num(&mut f)?,
        nc_net: num(&mut f)?,
        mtm_ref: num(&mut f)?,
        tie: num(&mut f)?,
        mo_seen: num(&mut f)?,
        mo_ok: num(&mut f)?,
        mo_min: num(&mut f)?,
        wk_seen: num(&mut f)?,
        wk_pos: num(&mut f)?,
        wk_min: num(&mut f)?,
        // round B: the split statistics when the line carries them (SPLITS18)
        sp: crate::split18::sp_of(&mut f)?,
    })
}

/// One composed window figure as a line.
#[must_use]
pub fn fig_line(f: &crate::wf::Fig) -> String {
    format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", f.net, f.mdd, f.up, f.low, f.trades, f.wins, u8::from(f.all_pos), f.mo_ok, f.wk_pos, f.d_pos)
}

/// The inverse of [`fig_line`].
///
/// # Errors
/// A malformed line.
pub fn fig_of_line(l: &str) -> Result<crate::wf::Fig, String> {
    let mut f = l.split('\t');
    Ok(crate::wf::Fig { net: num(&mut f)?, mdd: num(&mut f)?, up: num(&mut f)?, low: num(&mut f)?, trades: num(&mut f)?, wins: num(&mut f)?, all_pos: num::<u8>(&mut f)? == 1, mo_ok: num(&mut f)?, wk_pos: num(&mut f)?, d_pos: num(&mut f)? })
}

/// A segment's accumulated day-major results after its finished batches.
#[derive(Clone, Debug, Default)]
pub struct Ckpt {
    pub fingerprint: String,
    /// The first list of the next batch (= lists done).
    pub next_b0: usize,
    pub batches: usize,
    pub steps: u64,
    pub accts: u64,
    pub nbk: [(u64, u64); 4],
    pub tops: Vec<Tops>,
    /// Per window row: (checked, rewalked, pending, best: (key, sum, learning fig, test fig)).
    pub win: Vec<(u64, u64, u64, Option<((i64, i64, i64), Sum, crate::wf::Fig, crate::wf::Fig)>)>,
    pub flagged: Vec<crate::report3::Flag>,
    /// v3.2 round B (addendum 18): the split picks accumulated so far (`split18::Sel18::ckpt_lines`, one line each, tags S18*).
    pub sp18: Vec<String>,
}

/// Writes a checkpoint (text; atomic rename).
///
/// # Errors
/// An unwritable file.
pub fn save(path: &std::path::Path, c: &Ckpt) -> Result<(), String> {
    let mut s = String::new();
    let _ = writeln!(s, "SPIKEFADE-CKPT\t1\t{}", c.fingerprint);
    let _ = writeln!(s, "POS\t{}\t{}\t{}\t{}", c.next_b0, c.batches, c.steps, c.accts);
    let _ = writeln!(s, "NBK\t{}", c.nbk.iter().map(|x| format!("{},{}", x.0, x.1)).collect::<Vec<_>>().join("\t"));
    for (g, t) in c.tops.iter().enumerate() {
        let (k, views) = t.export();
        let _ = writeln!(s, "TOPS\t{g}\t{k}\t{}", views.len());
        for (v, list) in views.iter().enumerate() {
            for (key, sm) in list {
                let _ = writeln!(s, "T\t{g}\t{v}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, sum_line(sm));
            }
        }
    }
    for (r, w) in c.win.iter().enumerate() {
        let _ = writeln!(s, "W\t{r}\t{}\t{}\t{}", w.0, w.1, w.2);
        if let Some((key, sm, lf, tf)) = &w.3 {
            let _ = writeln!(s, "WB\t{r}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, sum_line(sm));
            let _ = writeln!(s, "WL\t{r}\t{}", fig_line(lf));
            let _ = writeln!(s, "WT\t{r}\t{}", fig_line(tf));
        }
    }
    for f in &c.flagged {
        let _ = writeln!(s, "F\t{}\t{}\t{}\t{}\t{}\t{}", f.row, f.list, f.si, f.l, f.ni, sum_line(&f.sm));
    }
    for l in &c.sp18 {
        let _ = writeln!(s, "{l}");
    }
    let _ = writeln!(s, "END");
    let part = path.with_extension("ckpt.part");
    std::fs::write(&part, s).map_err(|e| format!("{}: {e}", part.display()))?;
    std::fs::rename(&part, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a checkpoint written by [`save`] (None: no file; Err: a file whose fingerprint differs or that is malformed / incomplete).
///
/// # Errors
/// A malformed or foreign checkpoint.
pub fn load(path: &std::path::Path, fingerprint: &str, ngroup: usize, nrows: usize) -> Result<Option<Ckpt>, String> {
    let Ok(text) = std::fs::read_to_string(path) else { return Ok(None) };
    if !text.ends_with("END\n") {
        return Err(format!("{}: incomplete checkpoint", path.display()));
    }
    let mut c = Ckpt { win: vec![(0, 0, 0, None); nrows], ..Ckpt::default() };
    let mut tk: Vec<(usize, Vec<Vec<((i64, i64, i64), Sum)>>)> = Vec::new();
    let mut wb: Vec<Option<((i64, i64, i64), Sum)>> = vec![None; nrows];
    let mut wl: Vec<Option<crate::wf::Fig>> = vec![None; nrows];
    let mut wt: Vec<Option<crate::wf::Fig>> = vec![None; nrows];
    for l in text.lines() {
        let (tag, rest) = l.split_once('\t').unwrap_or((l, ""));
        let mut f = rest.split('\t');
        match tag {
            "SPIKEFADE-CKPT" => {
                let _v: u32 = num(&mut f)?;
                let fp = rest.split_once('\t').map_or("", |x| x.1);
                if fp != fingerprint {
                    return Err(format!("{}: written by another run (fingerprint differs): refused; move it aside to start fresh", path.display()));
                }
                c.fingerprint = fp.to_owned();
            }
            "POS" => {
                c.next_b0 = num(&mut f)?;
                c.batches = num(&mut f)?;
                c.steps = num(&mut f)?;
                c.accts = num(&mut f)?;
            }
            "NBK" => {
                for (i, x) in rest.split('\t').enumerate().take(4) {
                    let (a, b) = x.split_once(',').ok_or("NBK")?;
                    if let Some(y) = c.nbk.get_mut(i) {
                        *y = (a.parse().map_err(|_| "NBK")?, b.parse().map_err(|_| "NBK")?);
                    }
                }
            }
            "TOPS" => {
                let _g: usize = num(&mut f)?;
                let k: usize = num(&mut f)?;
                let nv: usize = num(&mut f)?;
                tk.push((k, vec![Vec::new(); nv]));
            }
            "T" => {
                let g: usize = num(&mut f)?;
                let v: usize = num(&mut f)?;
                let key = (num(&mut f)?, num(&mut f)?, num(&mut f)?);
                let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                if let Some(x) = tk.get_mut(g).and_then(|t| t.1.get_mut(v)) {
                    x.push((key, sm));
                }
            }
            "W" => {
                let r: usize = num(&mut f)?;
                if let Some(w) = c.win.get_mut(r) {
                    w.0 = num(&mut f)?;
                    w.1 = num(&mut f)?;
                    w.2 = num(&mut f)?;
                }
            }
            "WB" => {
                let r: usize = num(&mut f)?;
                let key = (num(&mut f)?, num(&mut f)?, num(&mut f)?);
                let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                if let Some(x) = wb.get_mut(r) {
                    *x = Some((key, sm));
                }
            }
            "WL" | "WT" => {
                let r: usize = num(&mut f)?;
                let fg = fig_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                if let Some(x) = if tag == "WL" { wl.get_mut(r) } else { wt.get_mut(r) } {
                    *x = Some(fg);
                }
            }
            "F" => {
                let row: u8 = num(&mut f)?;
                let list: u32 = num(&mut f)?;
                let si: u32 = num(&mut f)?;
                let lv: u8 = num(&mut f)?;
                let ni: u8 = num(&mut f)?;
                let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                c.flagged.push(crate::report3::Flag { row, list, si, l: lv, ni, sm });
            }
            t if t.starts_with("S18") => c.sp18.push(l.to_owned()),
            _ => {}
        }
    }
    if tk.len() != ngroup {
        return Err(format!("{}: {} groups, the run keeps {ngroup}", path.display(), tk.len()));
    }
    c.tops = tk.iter().map(|(k, views)| Tops::import(*k, views)).collect();
    for (r, w) in c.win.iter_mut().enumerate() {
        if let (Some(Some((key, sm))), Some(Some(lf)), Some(Some(tf))) = (wb.get(r), wl.get(r), wt.get(r)) {
            w.3 = Some((*key, *sm, *lf, *tf));
        }
    }
    Ok(Some(c))
}

/// The checkpoint directory of the run (`--checkpoint <dir>`, day-major only).
pub static CKPT_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
/// Test hook: stop after this many finished batches (0 = never; `--stop-after-batches N`).
pub static STOP_AFTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Reads `--checkpoint <dir>` and `--stop-after-batches N`.
///
/// # Errors
/// A checkpoint without the day-major mode, an unusable directory, a bad number.
pub fn from_args(a: &[String]) -> Result<Option<String>, String> {
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let Some(d) = get("--checkpoint") else { return Ok(None) };
    if !a.iter().any(|x| x == "--day-major") {
        return Err("--checkpoint needs --day-major (the batch is the checkpoint unit)".to_owned());
    }
    std::fs::create_dir_all(d).map_err(|e| format!("--checkpoint {d}: {e}"))?;
    let _ = CKPT_DIR.set(std::path::PathBuf::from(d));
    if let Some(v) = get("--stop-after-batches") {
        STOP_AFTER.store(v.parse::<usize>().map_err(|_| format!("--stop-after-batches {v}"))?, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(Some(format!("checkpoint dir {d} (a finished batch is never lost; resume = the same arguments)")))
}
