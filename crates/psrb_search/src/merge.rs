//! The merge (SEARCH-SPEC 6.6, 7, 8): the GLOBAL heaps, the account-control stage over
//! every heap's recipes, each window's picks, the unseen judging, the chains, the fixed
//! split and the luck verdict's inputs.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rayon::prelude::*;

use crate::acct::{Book, C0, Heaps, Key, Luck, N_OPT, N_SCOPE_TF, N_WIN, ROW0, Row, TopK, by_strength, flips, key_range, opt_of, prefix_keys, strength_order};
use crate::common::{N_MONTHS, N_TF, UNSEEN_FIRST};
use crate::phase_a::Sym;
use crate::search::{TfOut, build_unit, stage, unpack};
use crate::spool::{Hot, Record, load_full, load_hot};

/// 43 scopes: 0 GLOBAL, 1..=6 TF, 7..=42 TF x SIDE.
pub(crate) const N_SCOPE: usize = 1 + N_TF + N_TF * 6;

pub(crate) fn scope_name(s: usize) -> String {
    let tf = |ti: usize| crate::common::TFS.get(ti).copied().unwrap_or(0);
    if s == 0 {
        "GLOBAL".to_owned()
    } else if s <= N_TF {
        format!("TF {} min", tf(s - 1))
    } else {
        let x = s - 1 - N_TF;
        format!("TF {} min x {}", tf(x / 6), crate::common::SIDE6.get(x % 6).copied().unwrap_or("?"))
    }
}

/// The unseen months of window `w` (Y2020..Y2026), and FIXED (index 7 here).
pub(crate) fn unseen(w: usize) -> (usize, usize) {
    match w {
        0 => (16, 28),
        1 => (28, 40),
        2 => (40, 52),
        3 => (52, 64),
        4 => (64, 76),
        5 => (76, 88),
        6 => (88, 98),
        _ => (52, 98),
    }
}

/// A judged pick (window, scope, rank) on its unseen months, or a chain.
#[derive(Clone, Debug)]
pub(crate) struct Judged {
    pub(crate) ids: Vec<Option<u64>>,
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) row: Box<Row>,
    pub(crate) key: Key,
    pub(crate) counters: crate::acct::Counters,
    pub(crate) dd_bp: i128,
    pub(crate) dd_paisa: i128,
    pub(crate) dd_peak_ts: i64,
    pub(crate) dd_low_ts: i64,
    /// Every trade (chains and picks that print their trades): the full record and the took line.
    pub(crate) trades: Vec<(Record, crate::acct::Took, u64)>,
}

/// What the merge produced.
pub(crate) struct MergeOut {
    /// `[w * N_SCOPE + s]` plain top-20.
    pub(crate) plain: Vec<TopK>,
    /// `[w * N_SCOPE + s]` with the controls, top-5.
    pub(crate) ctl: Vec<TopK>,
    pub(crate) luck: Luck,
    pub(crate) tried_b: u64,
    pub(crate) tried_buildup: u64,
    pub(crate) tried_ctl: u64,
    pub(crate) recipes_ctl: usize,
    /// `[w * N_SCOPE + s][k]` judged on window w's unseen months (w 0..7: Y2020..Y2026).
    pub(crate) judged: Vec<Vec<Judged>>,
    /// `[s][k]` the chain of picks #k+1 over Y2020..Y2026 (82 unseen months).
    pub(crate) chains: Vec<Vec<Judged>>,
    /// The plain GLOBAL pick #1 chain (no controls), from the phase B rows.
    pub(crate) plain_chain: Judged,
    /// FIXED: GLOBAL picks #1..#5 of the 2018-09..2022-12 choice on 2023-01..2026-10.
    pub(crate) fixed: Vec<Judged>,
}

fn heap_ix(w: usize, s: usize) -> usize {
    w * N_SCOPE + s
}

/// Replay `recs` (already restricted and ordered) on a book.
fn feed(book: &mut Book, recs: &[Hot], which: usize, base: u32) {
    for (i, r) in recs.iter().enumerate() {
        book.step(r, which, base + u32::try_from(i).unwrap_or(0));
    }
}

fn judged_of(book: &Book, ids: Vec<Option<u64>>, a: usize, b: usize, id: u64) -> Judged {
    Judged {
        ids,
        a,
        b,
        row: Box::new(book.row),
        key: key_range(&book.row, a, b, id),
        counters: book.counters,
        dd_bp: book.dd_bp,
        dd_paisa: book.dd_paisa,
        dd_peak_ts: book.dd_peak_ts,
        dd_low_ts: book.dd_low_ts,
        trades: Vec::new(),
    }
}

/// The merge.
#[allow(clippy::too_many_lines, reason = "the merge's one sequence")]
pub(crate) fn merge(out: &Path, syms: &[Sym], tfs: &[TfOut], twin: Option<u64>, log: &dyn Fn(String)) -> Result<MergeOut, String> {
    let fl = flips();
    let mut luck = Luck::new();
    let mut tried_b = 0_u64;
    let mut tried_buildup = 0_u64;
    let empty_heaps = Heaps::new(20);
    let tf_heaps = |ti: usize| tfs.iter().find(|x| x.ti == ti).map_or(&empty_heaps, |x| &x.heaps);
    for t in tfs {
        luck.merge(&t.luck);
        tried_b += t.tried;
        tried_buildup += t.tried_buildup;
    }
    // Plain heaps of the 43 scopes; GLOBAL = the top 20 of the six TF heaps' union.
    let mut plain: Vec<TopK> = (0..N_WIN * N_SCOPE).map(|_| TopK::new(20)).collect();
    for w in 0..N_WIN {
        for ti in 0..N_TF {
            let hs = tf_heaps(ti);
            for s in 0..N_SCOPE_TF {
                let Some(h) = hs.get(w, s) else { continue };
                let scope = if s == 0 { 1 + ti } else { 1 + N_TF + ti * 6 + (s - 1) };
                if let Some(dst) = plain.get_mut(heap_ix(w, scope)) {
                    dst.merge(h);
                }
                if s == 0
                    && let Some(g) = plain.get_mut(heap_ix(w, 0))
                {
                    g.merge(h);
                }
            }
        }
    }
    // The control stage: every recipe of the GLOBAL and TF scopes' plain top 20 of the
    // seven test windows x every option set (APPROXIMATE 44: the TF x SIDE scopes keep
    // their plain picks; the full 43 scopes would be several hours on this machine).
    let mut member: BTreeMap<u64, Vec<(usize, usize)>> = BTreeMap::new();
    let mut plain_rows: HashMap<u64, Box<Row>> = HashMap::new();
    let mut ctl: Vec<TopK> = (0..N_WIN * N_SCOPE).map(|_| TopK::new(5)).collect();
    for w in 0..N_WIN {
        for s in 0..N_SCOPE {
            if let Some(h) = plain.get(heap_ix(w, s)) {
                if w >= 7 || s > N_TF {
                    if let Some(dst) = ctl.get_mut(heap_ix(w, s)) {
                        for e in h.v.iter().take(5) {
                            dst.offer(&e.key, e.row.as_deref());
                        }
                    }
                    continue;
                }
                for e in &h.v {
                    member.entry(e.key.id).or_default().push((w, s));
                    if let Some(r) = e.row.as_ref() {
                        plain_rows.entry(e.key.id).or_insert_with(|| r.clone());
                    }
                }
            }
        }
    }
    let recipes_ctl = member.len();
    let mut groups: BTreeMap<(usize, usize), Vec<u64>> = BTreeMap::new();
    for id in member.keys() {
        let r = unpack(*id);
        groups.entry((r.ti, r.lane)).or_default().push(*id);
    }
    let mut tried_ctl = 0_u64;
    let mut done = 0_usize;
    for ((ti, lane), ids) in &groups {
        let hot = load_hot(out, *ti, *lane, syms)?;
        let mut units: BTreeMap<usize, crate::search::Unit<'_>> = BTreeMap::new();
        for id in ids {
            let r = unpack(*id);
            let unit = units.entry(r.core).or_insert_with(|| build_unit(&hot, *ti, r.core, *lane, twin));
            let kept = unit.kept(r.combo);
            let mut recs = unit.records(&kept);
            match r.side {
                0 | 3 => recs.retain(|x| x.side == 0),
                1 | 4 => recs.retain(|x| x.side == 1),
                _ => {}
            }
            let mut strong = recs.clone();
            strength_order(&mut strong);
            let which = usize::from(r.side >= 3);
            let rows: Vec<(usize, Box<Row>)> = (0..N_OPT)
                .into_par_iter()
                .map(|o| {
                    let mut b = Book::new(opt_of(o), C0, false);
                    feed(&mut b, if by_strength(o) { &strong } else { &recs }, which, 0);
                    b.finish();
                    (o, Box::new(b.row))
                })
                .collect();
            let mut_list = member.get(id).cloned().unwrap_or_default();
            for (o, row) in &rows {
                let oid = (*id & !0x7FF) | u64::try_from(*o).unwrap_or(0);
                if *o > 0 {
                    luck.offer_row(&fl, row);
                    tried_ctl += 1;
                }
                let keys = prefix_keys(row, oid);
                for &(w, s) in &mut_list {
                    if let (Some(h), Some(k)) = (ctl.get_mut(heap_ix(w, s)), keys.get(w)) {
                        h.offer(k, Some(row));
                    }
                }
            }
            done += 1;
            if done % 50 == 0 {
                stage(format!("merge: control stage {done} of {recipes_ctl} recipes"));
            }
        }
        log(format!("merge: control stage, tf index {ti} lane X{}: {} recipes, {done} of {recipes_ctl} done", lane + 1, ids.len()));
    }
    // The picks and the records each needs (its judged months only).
    struct Need {
        id: u64,
        a: usize,
        b: usize,
        full: bool,
    }
    let mut needs: BTreeMap<(usize, usize), Vec<Need>> = BTreeMap::new();
    let mut add_need = |id: u64, a: usize, b: usize, full: bool, needs: &mut BTreeMap<(usize, usize), Vec<Need>>| {
        let r = unpack(id);
        let v = needs.entry((r.ti, r.lane)).or_default();
        if let Some(x) = v.iter_mut().find(|x| x.id == id && x.a == a && x.b == b) {
            x.full |= full;
        } else {
            v.push(Need { id, a, b, full });
        }
    };
    for w in 0..7 {
        let (a, b) = unseen(w);
        for s in 0..N_SCOPE {
            if let Some(h) = ctl.get(heap_ix(w, s)) {
                for (k, e) in h.v.iter().enumerate() {
                    add_need(e.key.id, a, b, k == 0 && s <= N_TF, &mut needs);
                }
            }
        }
    }
    if let Some(h) = ctl.get(heap_ix(3, 0)) {
        for (k, e) in h.v.iter().enumerate() {
            add_need(e.key.id, 52, 98, k == 0, &mut needs);
        }
    }
    let mut store: HashMap<(u64, usize, usize), (Vec<Hot>, Vec<Record>)> = HashMap::new();
    for ((ti, lane), list) in &needs {
        stage(format!("merge: picks' records tf index {ti} lane X{}", lane + 1));
        let hot = load_hot(out, *ti, *lane, syms)?;
        let full = if list.iter().any(|x| x.full) { Some(load_full(out, *ti, *lane, syms)?) } else { None };
        let mut units: BTreeMap<usize, crate::search::Unit<'_>> = BTreeMap::new();
        for nd in list {
            let r = unpack(nd.id);
            let unit = units.entry(r.core).or_insert_with(|| build_unit(&hot, *ti, r.core, *lane, twin));
            let kept = unit.kept(r.combo);
            let mut pairs: Vec<(Hot, u32)> = kept
                .iter()
                .filter_map(|&p| {
                    let ri = *unit.idx.get(usize::try_from(p).ok()?)?;
                    Some((*hot.get(usize::try_from(ri).ok()?)?, ri))
                })
                .filter(|(h, _)| match r.side {
                    0 | 3 => h.side == 0,
                    1 | 4 => h.side == 1,
                    _ => true,
                })
                .filter(|(h, _)| usize::from(h.month_t) >= nd.a && usize::from(h.month_t) < nd.b)
                .collect();
            if by_strength(r.opt) {
                let mut i = 0;
                while i < pairs.len() {
                    let t = pairs.get(i).map_or(0, |x| x.0.t);
                    let mut j = i;
                    while pairs.get(j).is_some_and(|x| x.0.t == t) {
                        j += 1;
                    }
                    if let Some(g) = pairs.get_mut(i..j) {
                        g.sort_by(|a, b| b.0.strength.cmp(&a.0.strength).then(a.0.stock.cmp(&b.0.stock)));
                    }
                    i = j;
                }
            }
            let recs: Vec<Hot> = pairs.iter().map(|x| x.0).collect();
            let fulls: Vec<Record> = match (&full, nd.full) {
                (Some(f), true) => pairs.iter().filter_map(|x| f.get(usize::try_from(x.1).ok()?).copied()).collect(),
                _ => Vec::new(),
            };
            store.insert((nd.id, nd.a, nd.b), (recs, fulls));
        }
    }
    let get = |id: u64, a: usize, b: usize| store.get(&(id, a, b));
    // Each window's picks on its unseen months, a fresh Rs 20 lakh account each.
    let mut judged: Vec<Vec<Judged>> = Vec::with_capacity(N_WIN * N_SCOPE);
    for w in 0..N_WIN {
        for s in 0..N_SCOPE {
            let mut v = Vec::new();
            if w < 7
                && let Some(h) = ctl.get(heap_ix(w, s))
            {
                let (a, b) = unseen(w);
                for e in &h.v {
                    let r = unpack(e.key.id);
                    let mut book = Book::new(opt_of(r.opt), C0, false);
                    if let Some((recs, _)) = get(e.key.id, a, b) {
                        feed(&mut book, recs, usize::from(r.side >= 3), 0);
                    }
                    book.finish();
                    v.push(judged_of(&book, vec![Some(e.key.id)], a, b, e.key.id));
                }
            }
            judged.push(v);
        }
    }
    // The chains: for each scope and k, picks #k of Y2020..Y2026 on ONE account.
    let mut chains: Vec<Vec<Judged>> = Vec::with_capacity(N_SCOPE);
    for s in 0..N_SCOPE {
        let mut per_k = Vec::new();
        for k in 0..5 {
            let record = k == 0 && s <= N_TF;
            let mut book = Book::new(opt_of(0), C0, record);
            let mut ids = Vec::new();
            let mut lists: Vec<(u64, usize, usize)> = Vec::new();
            let mut base = 0_u32;
            for w in 0..7 {
                let (a, b) = unseen(w);
                let pick = ctl.get(heap_ix(w, s)).and_then(|h| h.v.get(k)).map(|e| e.key.id);
                ids.push(pick);
                let Some(id) = pick else { continue };
                let r = unpack(id);
                book.set_opt(opt_of(r.opt));
                if let Some((recs, _)) = get(id, a, b) {
                    feed(&mut book, recs, usize::from(r.side >= 3), base);
                    lists.push((id, a, b));
                    base += u32::try_from(recs.len()).unwrap_or(0);
                }
            }
            book.finish();
            let id0 = ids.first().copied().flatten().unwrap_or(0);
            let mut j = judged_of(&book, ids, UNSEEN_FIRST, N_MONTHS, id0);
            if let Some(took) = book.took.as_ref() {
                for t in took {
                    // find the list and the record of this trade
                    let mut off = 0_u32;
                    for (id, a, b) in &lists {
                        let Some((recs, fulls)) = get(*id, *a, *b) else { continue };
                        let len = u32::try_from(recs.len()).unwrap_or(0);
                        if t.src >= off && t.src < off + len {
                            if let Some(fr) = fulls.get(usize::try_from(t.src - off).unwrap_or(usize::MAX)) {
                                j.trades.push((*fr, *t, *id));
                            }
                            break;
                        }
                        off += len;
                    }
                }
            }
            per_k.push(j);
        }
        chains.push(per_k);
    }
    // The plain GLOBAL pick #1 chain (no controls) from the phase B rows: plain nets do
    // not depend on capital at a fixed slot size, so each year's months are its pick's.
    let mut prow = ROW0;
    let mut pids = Vec::new();
    for w in 0..7 {
        let (a, b) = unseen(w);
        let pick = plain.get(heap_ix(w, 0)).and_then(|h| h.v.first());
        pids.push(pick.map(|e| e.key.id));
        if let Some(row) = pick.and_then(|e| e.row.as_ref()) {
            for m in a..b {
                if let (Some(dst), Some(src)) = (prow.get_mut(m), row.get(m)) {
                    *dst = *src;
                }
            }
        }
    }
    let plain_chain = Judged {
        ids: pids.clone(),
        a: UNSEEN_FIRST,
        b: N_MONTHS,
        row: Box::new(prow),
        key: key_range(&prow, UNSEEN_FIRST, N_MONTHS, pids.first().copied().flatten().unwrap_or(0)),
        counters: crate::acct::Counters::default(),
        dd_bp: 0,
        dd_paisa: 0,
        dd_peak_ts: 0,
        dd_low_ts: 0,
        trades: Vec::new(),
    };
    // FIXED: the 2018-09..2022-12 choice judged on 2023-01..2026-10.
    let mut fixed = Vec::new();
    if let Some(h) = ctl.get(heap_ix(3, 0)) {
        for (k, e) in h.v.iter().enumerate() {
            let r = unpack(e.key.id);
            let mut book = Book::new(opt_of(r.opt), C0, k == 0);
            let mut j0 = None;
            if let Some((recs, fulls)) = get(e.key.id, 52, 98) {
                feed(&mut book, recs, usize::from(r.side >= 3), 0);
                book.finish();
                let mut j = judged_of(&book, vec![Some(e.key.id)], 52, 98, e.key.id);
                if let Some(took) = book.took.as_ref() {
                    for t in took {
                        if let Some(fr) = fulls.get(usize::try_from(t.src).unwrap_or(usize::MAX)) {
                            j.trades.push((*fr, *t, e.key.id));
                        }
                    }
                }
                j0 = Some(j);
            }
            fixed.push(j0.unwrap_or_else(|| {
                let b = Book::new(opt_of(0), C0, false);
                judged_of(&b, vec![Some(e.key.id)], 52, 98, e.key.id)
            }));
        }
    }
    let _ = plain_rows;
    Ok(MergeOut { plain, ctl, luck, tried_b, tried_buildup, tried_ctl, recipes_ctl, judged, chains, plain_chain, fixed })
}

/// The luck verdict's numbers (SEARCH-SPEC 8.2).
pub(crate) struct LuckVerdict {
    pub(crate) observed: u32,
    pub(crate) beaten: u32,
    pub(crate) clears: bool,
    pub(crate) l95: u32,
    pub(crate) median: u16,
}

pub(crate) fn luck_verdict(luck: &Luck, chain_row: &Row) -> LuckVerdict {
    let observed = u32::try_from(
        chain_row
            .iter()
            .skip(UNSEEN_FIRST)
            .filter(|m| m.trades > 0 && 40 * i128::from(m.net) >= C0)
            .count(),
    )
    .unwrap_or(0);
    let count_ge = |o: u32| u32::try_from(luck.best.iter().filter(|&&b| u32::from(b) >= o).count()).unwrap_or(0);
    let beaten = count_ge(observed);
    let clears = (1 + beaten) * 20 <= 1 + 1_000;
    let mut l95 = 83;
    for o in 0..=82_u32 {
        if (1 + count_ge(o)) * 20 <= 1 + 1_000 {
            l95 = o;
            break;
        }
    }
    let mut v = luck.best.clone();
    v.sort_unstable();
    let median = v.get(v.len() / 2).copied().unwrap_or(0);
    LuckVerdict { observed, beaten, clears, l95, median }
}
