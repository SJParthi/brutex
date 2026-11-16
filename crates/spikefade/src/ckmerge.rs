//! v3.4 E3 (build:shard; cloud-kit/CLOUD-KIT-DESIGN.md E3 with critic F2, F3, F7a, F16): `spikefade ckpt-merge` folds the batch files of a
//! sharded day-major run (E1), in ascending batch order, into the engine's OWN checkpoint (`ckpt::save`, next_b0 = the list count, batches =
//! B), so ONE ordinary resume runs the closing passes.
//!
//! Merge laws (design 1.2), each through the engine's own rule (one common runtime): sums for steps, accounts, per-N counts, window
//! counters, split counters and the D2-1 safety counters; the kept accounts offered view by view through `Tops::offer_keyed` (= `push_view`,
//! top-K under the strict `Keyed` order, so the top-K of the union = the top-K of the per-batch top-Ks); the window picks by `row_better`
//! (the larger learning key, then the smaller unique `wtie`); the split picks by `split18::better_pick`; the flagged rows concatenated in
//! ASCENDING batch order (= the single run's push order). Timing (S18 ns_sum, the S18N samples) is summed / concatenated and never compared.
//!
//! Refused, by name: a fingerprint, plan or merge-switch that differs; a gap; a batch file whose body sha256 fails, that is truncated or
//! carries an unknown tag (critic F3, parsed in kit.rs); a duplicate whose body differs (a byte-identical duplicate is accepted and counted);
//! a foreign engine source or inputs manifest (E4); a batch whose D2-1 "exits at or before their booking second" is not 0 (critic F16).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::acct::{Sum, Tops};
use crate::kit::{Bck, Plan, WinBest};

/// The merge's counts (the report).
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub files: usize,
    pub dups: usize,
    pub batches: usize,
    pub safe: [u64; 7],
    pub kept: usize,
    pub flagged: usize,
    /// MEASURED: per file parse + fold time (ns).
    pub ns: Vec<u64>,
}

/// Merges batch files (one per batch 0..nb, any order given; sorted here) into the cumulative checkpoint state.
///
/// # Errors
/// A gap, a fingerprint / shape / switch that differs, a non-zero D2-1 safety counter.
pub fn merge(mut bs: Vec<Bck>, nb: usize) -> Result<(crate::ckpt::Ckpt, Stats), String> {
    bs.sort_by_key(|b| b.k);
    let mut st = Stats { batches: bs.len(), ..Stats::default() };
    let missing: Vec<usize> = (0..nb).filter(|k| bs.binary_search_by_key(k, |b| b.k).is_err()).collect();
    // MUTANT m_merge_drops_batch: a missing batch accepted (the merged state silently short of its accounts)
    if !missing.is_empty() && !cfg!(feature = "m_merge_drops_batch") {
        return Err(format!("{} of {nb} batch(es) missing (a gap): {:?}{}: refused; run them (a free shard or the Mac, E7)", missing.len(), missing.iter().take(20).collect::<Vec<_>>(), if missing.len() > 20 { " ..." } else { "" }));
    }
    if bs.windows(2).any(|w| w.first().map(|x| x.k) == w.get(1).map(|x| x.k)) {
        return Err("two batch files of one batch reached the fold (duplicates are resolved before): refused".to_owned());
    }
    let first = bs.first().ok_or("no batch file")?;
    let (fp0, nl0, tie52, splits18) = (first.fp.clone(), first.nl, first.tie52, first.splits18);
    // the merge reads the rules the run used: the Keyed order (TIE52) and the summary line format (SPLITS18)
    crate::sw::set(crate::sw::TIE52, tie52);
    crate::sw::set(crate::sw::SPLITS18, splits18);
    let shape_t: Vec<(usize, usize)> = first.tops.iter().map(|(k, v)| (*k, v.len())).collect();
    let nrows = first.win.len();
    let shape18 = first.s18shape.clone();
    let mut tops: Vec<Tops> = shape_t.iter().map(|&(k, nv)| Tops::import(k, &vec![Vec::new(); nv])).collect();
    let mut win: Vec<(u64, u64, u64, WinBest)> = vec![(0, 0, 0, None); nrows];
    let mut flagged: Vec<crate::report3::Flag> = Vec::new();
    let (mut steps, mut accts, mut nbk) = (0_u64, 0_u64, [(0_u64, 0_u64); 4]);
    let (mut read, mut offered, mut ns_sum, mut ns) = (0_u64, 0_u64, 0_u64, Vec::<u32>::new());
    let mut best: Vec<Vec<Option<crate::split18::Best>>> = shape18.iter().map(|&n| vec![None; n]).collect();
    let mut p_end = 0_usize;
    for b in bs {
        let t0 = std::time::Instant::now();
        // EXHAUSTIVE destructuring (design E3): a field added to the batch file without a merge law here fails to compile
        let Bck { fp, k, nb: bnb, p0, p1, nl, tie52: bt, splits18: bsp, steps: bsteps, accts: baccts, nbk: bnbk, safe, tops: btops, win: bwin, flagged: bflag, s18, s18shape, s18b, acct: _acct, body_sha: _body, ns_sum: bns_sum, s18n, prov: _prov } = b;
        if fp != fp0 || nl != nl0 || bnb != nb || bt != tie52 || bsp != splits18 {
            return Err(format!("batch {k}: fingerprint / list count / batch count / merge switches differ from the first batch file's: another run: refused"));
        }
        if p0 != p_end && !cfg!(feature = "m_merge_drops_batch") {
            return Err(format!("batch {k} starts at position {p0}, the previous batch ended at {p_end}: refused"));
        }
        p_end = p1;
        if btops.iter().map(|(k2, v)| (*k2, v.len())).collect::<Vec<_>>() != shape_t || bwin.len() != nrows || s18shape != shape18 {
            return Err(format!("batch {k}: its groups, views, window rows or split folds differ from the first file's: refused"));
        }
        if safe.get(3).copied().unwrap_or(0) != 0 {
            return Err(format!("batch {k}: D2-1 exits at or before their booking second = {} (must be 0, DESIGN-D2 Lemma D's premise): refused", safe.get(3).copied().unwrap_or(0)));
        }
        // MUTANT m_merge_double_steps: one batch's steps counted twice
        steps = steps.checked_add(bsteps).ok_or("steps overflow")?;
        if cfg!(feature = "m_merge_double_steps") && k == 0 {
            steps += bsteps;
        }
        accts = accts.checked_add(baccts).ok_or("accounts overflow")?;
        for (x, y) in nbk.iter_mut().zip(bnbk) {
            x.0 += y.0;
            x.1 += y.1;
        }
        for (x, y) in st.safe.iter_mut().zip(safe) {
            *x += y;
        }
        for (g, (_, views)) in btops.iter().enumerate() {
            let Some(t) = tops.get_mut(g) else { continue };
            for (v, list) in views.iter().enumerate() {
                for &(key, sm) in list {
                    t.offer_keyed(v, key, sm);
                }
            }
        }
        for (w, (ck, rw, pe, bb)) in win.iter_mut().zip(bwin) {
            w.0 += ck;
            w.1 += rw;
            w.2 += pe;
            if let Some((key, sm, lf, tf)) = bb {
                crate::report3::row_take(&mut w.3, key, sm, lf, tf);
            }
        }
        // MUTANT m_merge_flagged_shard_order: the flagged rows put in front (the shard / file order instead of the batch order)
        if cfg!(feature = "m_merge_flagged_shard_order") {
            let mut x = bflag;
            x.append(&mut flagged);
            flagged = x;
        } else {
            flagged.extend(bflag);
        }
        read += s18.0;
        offered += s18.1;
        ns_sum += bns_sum;
        ns.extend(s18n);
        for (si, fi, p) in s18b {
            let slot = best.get_mut(si).and_then(|v| v.get_mut(fi)).ok_or_else(|| format!("batch {k}: split {si} / fold {fi} outside the shape"))?;
            if slot.as_ref().is_none_or(|x| crate::split18::better_pick(&p.key, &p.tie, x)) {
                *slot = Some(p);
            }
        }
        st.ns.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX));
    }
    st.kept = tops.iter().map(|t| t.export().1.iter().map(Vec::len).sum::<usize>()).sum();
    st.flagged = flagged.len();
    let sp18 = crate::split18::ckpt_lines_of(read, offered, ns_sum, &ns, &best);
    let c = crate::ckpt::Ckpt { fingerprint: fp0, next_b0: nl0, batches: nb, steps, accts, nbk, tops, win, flagged, sp18 };
    Ok((c, st))
}

/// The batch files of segment `seg` in one shard folder (name order).
///
/// # Errors
/// An unreadable folder.
pub fn bck_files(dir: &Path, seg: &str) -> Result<Vec<PathBuf>, String> {
    Ok(crate::kit::sorted_dir(dir)?.into_iter().filter(|(n, _)| n.starts_with(&format!("{seg}.b")) && n.ends_with(".bck")).map(|x| x.1).collect())
}

/// Reads every batch file of the shard folders: one per batch, a byte-identical duplicate counted, a differing one refused; the
/// provenance checked (the same engine source, rustc, inputs and plan in every file, and `want_source` when given).
///
/// # Errors
/// A file that does not parse (sha256, END, unknown tag), a duplicate that differs, a foreign provenance.
pub fn read_all(dirs: &[PathBuf], seg: &str, plan: &Plan, want_source: Option<&str>) -> Result<(Vec<Bck>, Stats, Vec<String>), String> {
    let mut by: BTreeMap<usize, (Bck, PathBuf)> = BTreeMap::new();
    let mut st = Stats::default();
    let mut notes = Vec::new();
    let mut prov0: Option<(String, String, String, String)> = None;
    for d in dirs {
        for p in bck_files(d, seg)? {
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let b = Bck::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
            st.files += 1;
            let pv = (b.prov_of("source").unwrap_or("?").to_owned(), b.prov_of("rustc").unwrap_or("?").to_owned(), b.prov_of("inputs").unwrap_or("?").to_owned(), b.prov_of("plan").unwrap_or("?").to_owned());
            // MUTANT m_merge_foreign_source: the provenance never compared
            if !cfg!(feature = "m_merge_foreign_source") {
                if let Some(w) = want_source
                    && pv.0 != w
                {
                    return Err(format!("{}: engine source manifest {} is not the expected {w} (E4): refused", p.display(), pv.0));
                }
                if pv.3 != plan.sha {
                    return Err(format!("{}: written under plan {} but the merge's plan is {} (E4): refused", p.display(), pv.3, plan.sha));
                }
                match &prov0 {
                    None => prov0 = Some(pv.clone()),
                    Some(x) if *x != pv => return Err(format!("{}: provenance (source, rustc, inputs, plan) = {pv:?} differs from the first file's {x:?} (E4: another source tree, compiler or input set): refused", p.display())),
                    Some(_) => {}
                }
            }
            match plan.b.get(b.k) {
                Some(x) if x.0 == b.p0 && x.1 == b.p1 => {}
                _ => return Err(format!("{}: batch {} at [{}, {}) is not the plan's batch: refused", p.display(), b.k, b.p0, b.p1)),
            }
            if let Some((o, op)) = by.get(&b.k) {
                if o.body_sha == b.body_sha {
                    st.dups += 1;
                    notes.push(format!("batch {}: byte-identical duplicate {} (kept {})", b.k, p.display(), op.display()));
                    continue;
                }
                return Err(format!("batch {}: {} and {} have different bodies ({} vs {}): refused (a determinism fault or a changed input: the run is void until named)", b.k, op.display(), p.display(), o.body_sha, b.body_sha));
            }
            by.insert(b.k, (b, p));
        }
    }
    if let Some(x) = &prov0 {
        notes.push(format!("provenance of every file: engine source {}, rustc {}, inputs {}, plan {}", x.0, x.1, x.2, x.3));
        if x.0 == "unset" {
            notes.push("WARNING: the engine source manifest was not baked into this build (SF_SOURCE_SHA256 unset): build with it for a cloud run".to_owned());
        }
    }
    Ok((by.into_values().map(|x| x.0).collect(), st, notes))
}

/// `spikefade ckpt-merge --seg idx|stk --plan PLAN --out DIR [--fingerprint kit|v33] [--expect-source SHA] <shard dir>...`.
///
/// # Errors
/// Any refusal of [`read_all`] / [`merge`]; an existing different output.
pub fn cmd_ckpt_merge(a: &[String]) -> Result<String, String> {
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let seg = get("--seg").ok_or("ckpt-merge --seg idx|stk --plan PLAN --out DIR [--fingerprint kit|v33] [--expect-source SHA] <shard dirs>")?;
    if seg != "idx" && seg != "stk" {
        return Err(format!("--seg idx|stk, not {seg}"));
    }
    let plan = Plan::load(Path::new(get("--plan").ok_or("--plan PLAN")?))?;
    let out = PathBuf::from(get("--out").ok_or("--out DIR")?);
    let fpmode = get("--fingerprint").unwrap_or("kit");
    let mut skip = std::collections::BTreeSet::new();
    for k in ["--seg", "--plan", "--out", "--fingerprint", "--expect-source"] {
        if let Some(i) = a.iter().position(|x| x == k) {
            skip.insert(i);
            skip.insert(i + 1);
        }
    }
    let dirs: Vec<PathBuf> = a.iter().enumerate().skip(2).filter(|(i, _)| !skip.contains(i)).map(|(_, x)| PathBuf::from(x)).collect();
    if dirs.is_empty() {
        return Err("ckpt-merge: no shard folder".to_owned());
    }
    let t0 = std::time::Instant::now();
    let (bs, mut st0, notes) = read_all(&dirs, seg, &plan, get("--expect-source"))?;
    let t_read = t0.elapsed().as_secs_f64();
    let (mut c, st) = merge(bs, plan.b.len())?;
    st0.safe = st.safe;
    st0.kept = st.kept;
    st0.flagged = st.flagged;
    st0.ns = st.ns;
    st0.batches = st.batches;
    if fpmode == "v33" {
        // the ONE resume on the exact gated v3.3 binary (critic F9): its fingerprint has no kit words; only a plan that IS v3.3's uniform cut
        let l = plan.b.iter().map(|x| x.1 - x.0).max().unwrap_or(0);
        if plan.b.iter().enumerate().any(|(k, x)| x.0 != k * l || x.1 != ((k + 1) * l).min(plan.nl)) {
            return Err("--fingerprint v33: the plan is not a uniform cut of one lists-per-batch value (v3.3 cuts uniformly): refused".to_owned());
        }
        let cut = c.fingerprint.rfind("; batch plan sha256 ").ok_or("--fingerprint v33: the batch files' fingerprint carries no plan words")?;
        c.fingerprint.truncate(cut);
    } else if fpmode != "kit" {
        return Err(format!("--fingerprint kit|v33, not {fpmode}"));
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let dst = out.join(format!("{seg}.ckpt"));
    let tmp = out.join(format!("{seg}.merging"));
    crate::ckpt::save(&tmp, &c)?;
    let new = std::fs::read(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let verdict = match std::fs::read(&dst) {
        Ok(old) if old == new => {
            std::fs::remove_file(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
            "already merged: the existing checkpoint is byte-identical (a re-run of an interrupted merge)".to_owned()
        }
        Ok(_) => {
            let aside = out.join(format!("{seg}.merging-{}", crate::kit::utc_now()));
            std::fs::rename(&tmp, &aside).map_err(|e| format!("{}: {e}", aside.display()))?;
            return Err(format!("{}: exists with OTHER content (never overwritten); this merge kept aside as {}: refused", dst.display(), aside.display()));
        }
        Err(_) => {
            std::fs::rename(&tmp, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
            "written".to_owned()
        }
    };
    // the inputs manifest every shard checked (E4 / F4): equal in every shard folder that holds one; copied beside the checkpoint so the
    // resume (with --batch-files or --expect-inputs) checks its own inputs against the shards'
    let mut inputs: Option<String> = None;
    for d in &dirs {
        if let Ok(t) = std::fs::read_to_string(d.join(format!("{seg}.inputs.tsv"))) {
            match &inputs {
                None => inputs = Some(t),
                Some(x) if *x != t => return Err(format!("{}: its inputs manifest differs from another shard's (F4): refused", d.display())),
                Some(_) => {}
            }
        }
    }
    if let Some(t) = &inputs {
        let p = out.join(format!("{seg}.inputs.tsv"));
        match std::fs::read_to_string(&p) {
            Ok(o) if o == *t => {}
            Ok(_) => return Err(format!("{}: exists with other content: refused", p.display())),
            Err(_) => std::fs::write(&p, t).map_err(|e| format!("{}: {e}", p.display()))?,
        }
    }
    let mut ns = st0.ns.clone();
    ns.sort_unstable();
    let q = |p: usize| ns.get((ns.len().saturating_sub(1)) * p / 100).copied().unwrap_or(0) as f64 / 1e6;
    let mut rep = String::new();
    let _ = writeln!(rep, "ckpt-merge {} at {}: {verdict}", dst.display(), crate::kit::ist_label(crate::kit::utc_now()));
    let _ = writeln!(rep, "segment {seg}; plan {} (sha256 {}): {} batches over {} unique lists; fingerprint mode {fpmode}", plan.path, plan.sha, plan.b.len(), plan.nl);
    let _ = writeln!(rep, "shard folders {}: {} batch file(s) read, {} byte-identical duplicate(s), {} batch(es) merged (exactly once each)", dirs.len(), st0.files, st0.dups, st0.batches);
    let _ = writeln!(rep, "merged: steps {}, accounts {}, kept accounts {}, flagged rows {}, per-N booked {:?}", c.steps, c.accts, st0.kept, st0.flagged, c.nbk);
    let _ = writeln!(rep, "D2-1 safety counters summed (replayed, fallen back, ghost-walked, exits at or before booking = must be 0, class-days, list-days in classes >= 2, list-days): {:?}", st0.safe);
    let _ = writeln!(rep, "MEASURED: read + verify {t_read:.2} s; fold per file p50 {:.2} ms / p99 {:.2} ms / max {:.2} ms; inputs manifest {}", q(50), q(99), q(100), if inputs.is_some() { "copied beside the checkpoint" } else { "none in the shard folders" });
    for n in &notes {
        let _ = writeln!(rep, "{n}");
    }
    std::fs::write(out.join(format!("{seg}.merge.txt")), &rep).map_err(|e| e.to_string())?;
    Ok(rep)
}

/// The canonical text of a checkpoint (critic F2, for P-S3): every line but the timing (S18N dropped, S18's ns_sum 0), the kept accounts of
/// every view BEST FIRST (the `Keyed` order; `export` writes heap order), the flagged rows in their order.
///
/// # Errors
/// An unreadable or malformed checkpoint.
pub fn canon(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !text.ends_with("END\n") {
        return Err(format!("{}: incomplete checkpoint", path.display()));
    }
    let fp = text.lines().next().and_then(|l| l.strip_prefix("SPIKEFADE-CKPT\t1\t")).ok_or("not a checkpoint")?.to_owned();
    let ng = text.lines().filter(|l| l.starts_with("TOPS\t")).count();
    let nrows = text.lines().filter(|l| l.starts_with("W\t")).count();
    // the summary line format of the run that wrote it (with the split statistics or not): a T line holds 6 + 29 fields without them
    let sp = text.lines().find(|l| l.starts_with("T\t")).map(|l| l.split('\t').count() > 6 + 29).or_else(|| text.lines().find(|l| l.starts_with("F\t")).map(|l| l.split('\t').count() > 6 + 29));
    if let Some(sp) = sp {
        crate::sw::set(crate::sw::SPLITS18, sp);
    }
    let c = crate::ckpt::load(path, &fp, ng, nrows)?.ok_or("checkpoint vanished")?;
    let mut s = String::new();
    let _ = writeln!(s, "SPIKEFADE-CKPT\t1\t{}", c.fingerprint);
    let _ = writeln!(s, "POS\t{}\t{}\t{}\t{}", c.next_b0, c.batches, c.steps, c.accts);
    let _ = writeln!(s, "NBK\t{}", c.nbk.iter().map(|x| format!("{},{}", x.0, x.1)).collect::<Vec<_>>().join("\t"));
    for (g, t) in c.tops.iter().enumerate() {
        let (k, views) = t.canonical();
        let _ = writeln!(s, "TOPS\t{g}\t{k}\t{}", views.len());
        for (v, list) in views.iter().enumerate() {
            for (key, sm) in list {
                let _ = writeln!(s, "T\t{g}\t{v}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, crate::ckpt::sum_line(sm));
            }
        }
    }
    for (r, w) in c.win.iter().enumerate() {
        let _ = writeln!(s, "W\t{r}\t{}\t{}\t{}", w.0, w.1, w.2);
        if let Some((key, sm, lf, tf)) = &w.3 {
            let _ = writeln!(s, "WB\t{r}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, crate::ckpt::sum_line(sm));
            let _ = writeln!(s, "WL\t{r}\t{}", crate::ckpt::fig_line(lf));
            let _ = writeln!(s, "WT\t{r}\t{}", crate::ckpt::fig_line(tf));
        }
    }
    for f in &c.flagged {
        let _ = writeln!(s, "F\t{}\t{}\t{}\t{}\t{}\t{}", f.row, f.list, f.si, f.l, f.ni, crate::ckpt::sum_line(&f.sm));
    }
    for l in &c.sp18 {
        if l.starts_with("S18N\t") {
            continue;
        }
        if let Some(r) = l.strip_prefix("S18\t") {
            let f: Vec<&str> = r.split('\t').collect();
            let _ = writeln!(s, "S18\t{}\t{}\t0", f.first().unwrap_or(&""), f.get(1).unwrap_or(&""));
            continue;
        }
        let _ = writeln!(s, "{l}");
    }
    let _ = writeln!(s, "END");
    Ok(s)
}

/// `spikefade ckpt-canon <file.ckpt> [--tie52 on|off]`: the canonical text (stdout), its sha256 last.
///
/// # Errors
/// As [`canon`].
pub fn cmd_ckpt_canon(a: &[String]) -> Result<String, String> {
    let p = a.get(2).ok_or("ckpt-canon <file.ckpt> [--tie52 on|off]")?;
    if let Some(i) = a.iter().position(|x| x == "--tie52") {
        crate::sw::set(crate::sw::TIE52, a.get(i + 1).map(String::as_str) == Some("on"));
    }
    let s = canon(Path::new(p))?;
    Ok(format!("{s}CANONICAL-SHA256\t{}", crate::kit::sha256_bytes(s.as_bytes())))
}

/// `spikefade bck-check <dir> <seg>`: every batch file of a folder parsed and verified (sha256, END, tags): one line each (batch, body sha256,
/// accounts, all-accounts digest, threads, wall), then the count.
///
/// # Errors
/// A file that does not verify.
pub fn cmd_bck_check(a: &[String]) -> Result<String, String> {
    let (Some(d), Some(seg)) = (a.get(2), a.get(3)) else { return Err("bck-check <dir> <idx|stk>".to_owned()) };
    let mut out = String::new();
    let mut n = 0;
    for p in bck_files(Path::new(d), seg)? {
        let t = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        let b = Bck::parse(&t).map_err(|e| format!("{}: {e}", p.display()))?;
        let _ = writeln!(out, "batch\t{}\tbody\t{}\taccounts\t{}\tacct-digest\t{}\tthreads\t{}\twall_s\t{}\tmode\t{}", b.k, b.body_sha, b.accts, b.acct.1, b.prov_of("threads").unwrap_or("?"), b.prov_of("wall_s").unwrap_or("?"), b.prov_of("mode").unwrap_or("?"));
        n += 1;
    }
    let _ = write!(out, "{n} batch file(s) verified in {d}");
    Ok(out)
}

/// For the tests: one Sum's identity (list, setting x share, N).
#[must_use]
pub fn ident(s: &Sum) -> (u32, u16, u8) {
    (s.list, s.x, s.n)
}
