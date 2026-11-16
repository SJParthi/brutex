//! POSITIONAL v2 (design 5.1 / 5.2): the Supertrend 10 x 3 stream of the decision timeframe over EVERY candle (special sessions
//! included, continuous across nights, reset only at K13), the two exit rules XS (headline, F17 included) and FLIP (beside) across
//! days, the caps (K13 CA close, data end), and the GROUPS: the candidates that share an exit (P-MONO), with the kernel's FIRST mode
//! per group = one position per side at a time.
//!
//! The stream is folded by THE SAME CODE and configuration as `cands::build` (series `a`: the module's sniper rows with the first
//! eight Supertrend rows, the row 10 x 3000 at index 5; `ctx_of` copied verbatim from cands.rs), exactly as pos-diag's `st::build`,
//! whose stream equalled the binary's stored flip anchors on every row (stproof).

use std::collections::BTreeMap;

use common_feat::series::{CandleCtx, Series, SeriesCfg};
use feat_io::host::{Ctx, Stream};

/// The index of (10, 3000) in series `a`.
const ST_J: usize = 5;

/// One candle of the decision stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StC {
    /// END, epoch seconds.
    pub end: i64,
    /// The session index.
    pub sess: u32,
    /// The IST day.
    pub day: i32,
    /// The candle number in its session.
    pub k: u16,
    /// Supertrend 10 x 3: Some(true) = up (green), Some(false) = down, None = not ready.
    pub up: Option<bool>,
    /// A positional exit candle: `Session::entry_at(END)` (an entry session, END outside every halt), any time of day.
    pub elig: bool,
}

/// COPIED verbatim from cands.rs `ctx_of`.
fn ctx_of(ctx: &Ctx, i: usize, prev: Option<usize>) -> CandleCtx {
    let s = ctx.sessions.get(i);
    let day = s.map_or(0, |s| s.day);
    let new_session = prev != Some(i);
    CandleCtx {
        session_open: s.map_or(0, feat_io::data::Session::open),
        day,
        new_session,
        regular_full: s.is_some_and(|s| s.regular_full),
        cont_reset: new_session && ctx.ca.cont_reset(day),
    }
}

/// COPIED from cands.rs `cfgs` (series `a` only).
fn cfg_a(tf_s: i64) -> SeriesCfg {
    let mut a = SeriesCfg::sniper(tf_s);
    a.st = crate::cands::ST_ROWS.iter().take(8).copied().collect();
    a.ema = crate::cands::EMA_ROWS.to_vec();
    a
}

/// The Supertrend 10 x 3 stream over every candle of `stream`.
///
/// # Errors
/// A series configuration the module refuses.
pub fn st_stream(ctx: &Ctx, stream: &Stream) -> Result<Vec<StC>, String> {
    let mut sa = Series::new(cfg_a(stream.tf_s))?;
    let mut prev = None;
    let mut out = Vec::with_capacity(stream.candles.len());
    for &(i, c) in &stream.candles {
        let cc = ctx_of(ctx, i, prev);
        prev = Some(i);
        let (r, _) = sa.fold(&c, &cc);
        let Some(sess) = ctx.sessions.get(i) else { continue };
        // MUTANT m_pos_look (P3 (a) must catch it): the exit finder reads the NEXT candle's side (filled below).
        let up = if r.st_ready.get(ST_J).copied().unwrap_or(false) { r.st.get(ST_J).copied().flatten().map(|s| s.up) } else { None };
        out.push(StC { end: c.end, sess: u32::try_from(i).unwrap_or(u32::MAX), day: r.day, k: u16::try_from(r.k.max(0)).unwrap_or(u16::MAX), up, elig: sess.entry_at(c.end) });
    }
    if cfg!(feature = "m_pos_look") {
        for j in 0..out.len().saturating_sub(1) {
            let nx = out.get(j + 1).and_then(|x| x.up);
            if let Some(x) = out.get_mut(j) {
                x.up = nx;
            }
        }
    }
    Ok(out)
}

/// The two positional exit rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// XS (the HEADLINE, M1): the first later eligible candle against the trade; F17: an entry candle already against exits at the next
    /// eligible candle.
    Xs,
    /// FLIP (beside): the first eligible candle against AFTER the side has been with the trade (at the entry candle or later).
    Flip,
}

impl Rule {
    /// The exit id (exits rows `POS-XS10-30` / `POS-FLIP10-30`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Xs => "POS-XS10-30",
            Self::Flip => "POS-FLIP10-30",
        }
    }
}

/// A candidate's exit (5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exit {
    /// The exit candle (its stream index).
    Candle(usize),
    /// The K13 CA close instant (UTC microseconds).
    CaClose(i64),
    /// No exit before the data end.
    DataEnd,
}

/// The exit of the candidate at stream index `i` (5.1), before the caps: XS or FLIP over the eligible candles, across sessions.
#[must_use]
pub fn exit_of(s: &[StC], i: usize, long: bool, rule: Rule) -> Option<usize> {
    let c = s.get(i)?;
    match rule {
        Rule::Xs => {
            let immediate = c.up == Some(!long);
            let mut j = i + 1;
            while let Some(x) = s.get(j) {
                if x.elig && (immediate || x.up == Some(!long)) {
                    return Some(j);
                }
                j += 1;
            }
            None
        }
        Rule::Flip => {
            let mut seen_with = c.up == Some(long);
            let mut j = i + 1;
            while let Some(x) = s.get(j) {
                if x.up == Some(long) {
                    seen_with = true;
                } else if seen_with && x.elig && x.up == Some(!long) {
                    return Some(j);
                }
                j += 1;
            }
            None
        }
    }
}

/// The exits of every stream candle for one side and rule in ONE backward pass (O(candles)): `next_against[i]` = the first eligible
/// candle > i against the trade; `next_elig[i]` = the first eligible candle > i; `next_with[i]` = the first candle > i with the trade.
#[must_use]
pub fn exits_all(s: &[StC], long: bool, rule: Rule) -> Vec<Option<usize>> {
    let n = s.len();
    let mut next_against: Vec<Option<usize>> = vec![None; n + 1];
    let mut next_elig: Vec<Option<usize>> = vec![None; n + 1];
    let mut next_with: Vec<Option<usize>> = vec![None; n + 1];
    for i in (0..n).rev() {
        let (na, ne, nw) = (next_against.get(i + 1).copied().flatten(), next_elig.get(i + 1).copied().flatten(), next_with.get(i + 1).copied().flatten());
        let x = s.get(i + 1);
        let a = if x.is_some_and(|x| x.elig && x.up == Some(!long)) { Some(i + 1) } else { na };
        let e = if x.is_some_and(|x| x.elig) { Some(i + 1) } else { ne };
        let w = if x.is_some_and(|x| x.up == Some(long)) { Some(i + 1) } else { nw };
        if let (Some(p), Some(q), Some(r)) = (next_against.get_mut(i), next_elig.get_mut(i), next_with.get_mut(i)) {
            *p = a;
            *q = e;
            *r = w;
        }
    }
    (0..n)
        .map(|i| {
            let c = s.get(i)?;
            match rule {
                Rule::Xs => {
                    // MUTANT m_pos_group: an against-side (F17) entry is grouped with the with-side episode (its exit = the next against candle).
                    if c.up == Some(!long) && !cfg!(feature = "m_pos_group") {
                        next_elig.get(i).copied().flatten()
                    } else {
                        next_against.get(i).copied().flatten()
                    }
                }
                Rule::Flip => {
                    let u = if c.up == Some(long) { Some(i) } else { next_with.get(i).copied().flatten() };
                    u.and_then(|u| next_against.get(u).copied().flatten())
                }
            }
        })
        .collect()
}

/// One group (5.2): its candidates are contiguous in time order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The shared exit.
    pub exit: Exit,
    /// The exit decision instant (UTC microseconds; 0 for DataEnd).
    pub exit_at: i64,
    /// The first candidate (index into the candidate list) and the count.
    pub first: usize,
    /// See `first`.
    pub n: usize,
}

/// The groups and, per candidate, its (group, ordinal from 1).
pub type Grouped = (Vec<Group>, Vec<(u32, u32)>);

/// The capped exit of the candidate at stream index `si` (5.1): its raw exit candle `raw` (XS or FLIP), capped by the first K13 event
/// ahead of the entry (`cas`: (ex-date session open, epoch seconds; CA close instant, UTC microseconds), ascending) when the ex-date falls
/// before the exit (or there is no exit while the ex-date lies ahead), else the data end when there is no exit. Returns the exit and its
/// decision instant (UTC microseconds; 0 for the data end).
///
/// # Errors
/// An exit at or before its own entry (P3 (f), never silent).
pub fn cap_exit(s: &[StC], si: usize, raw: Option<usize>, cas: &[(i64, i64)]) -> Result<(Exit, i64), String> {
    let c = s.get(si).ok_or("candidate stream index")?;
    let t = c.end * 1_000_000;
    // The K13 event ahead of this entry: the first (ex-date open, close instant) whose close instant is after the entry.
    let ca = cas.iter().copied().find(|x| x.1 > t);
    let ex = match (raw, ca) {
        (Some(j), Some((ex_open, close_at))) if t < close_at && s.get(j).is_some_and(|x| x.end >= ex_open) => Exit::CaClose(close_at),
        (None, Some((_, close_at))) if t < close_at => Exit::CaClose(close_at),
        (Some(j), _) => Exit::Candle(j),
        (None, _) => Exit::DataEnd,
    };
    let exit_at = match ex {
        Exit::Candle(j) => s.get(j).map_or(0, |x| x.end * 1_000_000),
        Exit::CaClose(a) => a,
        Exit::DataEnd => 0,
    };
    if exit_at != 0 && exit_at <= t {
        return Err(format!("P3 (f): an exit at or before its own entry (stream {si})"));
    }
    Ok((ex, exit_at))
}

/// The groups of a side and rule over the candidates (`cand_si`: each candidate's stream index, ascending), with the caps applied (`cas`:
/// every K13 event as (ex-date session open, epoch seconds; CA close instant, UTC microseconds), ascending), and
/// each candidate's (group, ordinal from 1). P-MONO is asserted: two candidates either share one exit, or the later one is entered at or
/// after the earlier one's exit decision (the touching convention: an exit decided at t is sent before an entry decided at t).
///
/// # Errors
/// A P-MONO violation (never silent: the search refuses).
pub fn groups(s: &[StC], cand_si: &[usize], exits: &[Option<usize>], cas: &[(i64, i64)]) -> Result<Grouped, String> {
    let mut gs: Vec<Group> = Vec::new();
    let mut of: Vec<(u32, u32)> = Vec::with_capacity(cand_si.len());
    let mut seen: BTreeMap<Exit, usize> = BTreeMap::new();
    for (ci, &si) in cand_si.iter().enumerate() {
        let c = s.get(si).ok_or("candidate stream index")?;
        let raw = exits.get(si).copied().flatten();
        let t = c.end * 1_000_000;
        // The caps (5.1): a K13 ex-date before the exit (or no exit at all while the ex-date lies ahead): the CA close; else DataEnd.
        let (ex, exit_at) = cap_exit(s, si, raw, cas).map_err(|e| format!("{e} (candidate {ci})"))?;
        // MUTANT m_pos_sess: the group key is the SESSION (the intraday episode), so two positions of one side can be open at once.
        let same = if cfg!(feature = "m_pos_sess") { gs.last().is_some_and(|g| cand_si.get(g.first).and_then(|&f| s.get(f)).is_some_and(|f| f.sess == c.sess)) } else { gs.last().is_some_and(|g| g.exit == ex) };
        let ng = gs.len();
        match gs.last_mut() {
            Some(g) if same => {
                g.n += 1;
                of.push((u32::try_from(ng - 1).map_err(|_| "groups")?, u32::try_from(g.n).map_err(|_| "ordinal")?));
            }
            prev => {
                if let Some(p) = prev {
                    // P-MONO: the new group's first entry is at or after the previous group's exit decision.
                    if !cfg!(feature = "m_pos_sess") && (p.exit == Exit::DataEnd || (p.exit_at != 0 && t < p.exit_at)) {
                        return Err(format!("P-MONO violated at candidate {ci} (stream {si}): entry before the previous group's exit"));
                    }
                }
                if !cfg!(feature = "m_pos_sess") && seen.insert(ex, gs.len()).is_some() {
                    return Err(format!("P-MONO violated: exit {ex:?} met again after another exit (candidate {ci})"));
                }
                gs.push(Group { exit: ex, exit_at, first: ci, n: 1 });
                of.push((u32::try_from(gs.len() - 1).map_err(|_| "groups")?, 1));
            }
        }
    }
    Ok((gs, of))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(ups: &[Option<bool>], elig: &[bool]) -> Vec<StC> {
        ups.iter().zip(elig).enumerate().map(|(i, (&u, &e))| StC { end: 1_000 + 60 * i64::try_from(i).unwrap(), sess: 0, day: 0, k: u16::try_from(i).unwrap(), up: u, elig: e }).collect()
    }

    #[test]
    fn xs_flip_and_the_one_pass_exits_agree_with_the_direct_finder() {
        // A deterministic pseudo-random stream: sides, NONE, and ineligible candles (special sessions / halts).
        let mut x = 12_345_u64;
        let mut ups = Vec::new();
        let mut el = Vec::new();
        for _ in 0..4_000 {
            x = sniper_kernel::scale::splitmix(x);
            ups.push(match x % 7 {
                0 => None,
                1..=3 => Some(true),
                _ => Some(false),
            });
            el.push(!x.is_multiple_of(11));
        }
        let s = st(&ups, &el);
        for long in [true, false] {
            for rule in [Rule::Xs, Rule::Flip] {
                let all = exits_all(&s, long, rule);
                for (i, a) in all.iter().enumerate() {
                    assert_eq!(*a, exit_of(&s, i, long, rule), "i {i} long {long} {rule:?}");
                }
            }
        }
    }

    #[test]
    fn f17_one_candle_groups_and_flip_waits() {
        // up: U U D D U D ; all eligible. A long entry at 0 (with): XS exit at 2 (first DOWN). Entry at 2 (against): XS F17 exit at 3.
        let s = st(&[Some(true), Some(true), Some(false), Some(false), Some(true), Some(false)], &[true; 6]);
        let xs = exits_all(&s, true, Rule::Xs);
        assert_eq!(xs[0], Some(2));
        assert_eq!(xs[1], Some(2));
        assert_eq!(xs[2], Some(3));
        assert_eq!(xs[3], Some(4));
        let fl = exits_all(&s, true, Rule::Flip);
        // FLIP: an entry at 2 (against) waits for the turn with the trade (4) and exits at the next DOWN (5).
        assert_eq!(fl[2], Some(5));
        assert_eq!(fl[3], Some(5));
        // Groups under XS over candidates 0..=4: {0,1} exit 2; {2} exit 3 (F17 one-candle); {3} exit 4; {4} exit 5.
        let (g, of) = groups(&s, &[0, 1, 2, 3, 4], &xs, &[]).unwrap();
        assert_eq!(g.len(), 4);
        assert_eq!(of, vec![(0, 1), (0, 2), (1, 1), (2, 1), (3, 1)]);
        // FLIP groups: {0,1} exit 2... candidate 2 exits at 5 with 3 and 4: {2,3,4}.
        let (g2, of2) = groups(&s, &[0, 1, 2, 3, 4], &fl, &[]).unwrap();
        assert_eq!(g2.len(), 2);
        assert_eq!(of2, vec![(0, 1), (0, 2), (1, 1), (1, 2), (1, 3)]);
    }

    /// A synthetic 60-minute stream over `days` weekday sessions (6 candles a day, 09:15-15:15 + a 15:15-15:30 candle), a rise then a
    /// fall, so the Supertrend 10 x 3 turns.
    fn synthetic(days: i32) -> (Ctx, Stream) {
        let mut sessions = Vec::new();
        let mut candles = Vec::new();
        let d0 = common_feat::cal::parse_ymd("2024-01-01").unwrap();
        let mut px: i64 = 100_000;
        let mut n = 0_i64;
        let mut day = d0;
        while i32::try_from(sessions.len()).unwrap() < days {
            if matches!((day + 4).rem_euclid(7), 0 | 6) {
                day += 1;
                continue;
            }
            let open = i64::from(day) * 86_400 + 9 * 3_600 + 15 * 60 - 19_800;
            let end = open + 22_500;
            let si = sessions.len();
            sessions.push(feat_io::data::Session { day, kind: "REGULAR".into(), segs: vec![(open, end)], entry: true, regular_full: true, halts: Vec::new() });
            let mut s = open;
            while s < end {
                let e = (s + 3_600).min(end);
                // up for 120 candles, then down, then up again (the side turns twice)
                let step = if n < 120 { 300 } else if n < 220 { -400 } else { 350 };
                let o = px;
                px += step;
                let (h, l) = (o.max(px) + 50, o.min(px) - 50);
                candles.push((si, common_feat::fold::Candle { start: s, end: e, open: o, high: h, low: l, close: px, volume: 1_000 }));
                n += 1;
                s = e;
            }
            day += 1;
        }
        let ctx = Ctx { sessions, ca: common_feat::ca::CaBook::default(), ticks: Vec::new(), bhav: BTreeMap::new(), cont_start: d0 };
        (ctx, Stream { tf_s: 3_600, source: "synthetic".into(), candles })
    }

    #[test]
    fn p3a_the_stream_side_is_each_candles_own_close_never_the_next_candles() {
        // The positional stream against a direct fold of the same module and rows: equal on every candle. m_pos_look (the exit finder
        // reads the NEXT candle's Supertrend) is red here.
        let (ctx, stream) = synthetic(60);
        let s = st_stream(&ctx, &stream).unwrap();
        let mut sa = Series::new(cfg_a(stream.tf_s)).unwrap();
        let mut prev = None;
        let mut turns = 0;
        for (j, &(i, c)) in stream.candles.iter().enumerate() {
            let cc = ctx_of(&ctx, i, prev);
            prev = Some(i);
            let (r, _) = sa.fold(&c, &cc);
            let up = if r.st_ready.get(ST_J).copied().unwrap_or(false) { r.st.get(ST_J).copied().flatten().map(|x| x.up) } else { None };
            assert_eq!(s[j].up, up, "candle {j}");
            if j > 0 && s[j].up.is_some() && s[j - 1].up.is_some() && s[j].up != s[j - 1].up {
                turns += 1;
            }
        }
        assert!(turns >= 2, "the synthetic stream must turn ({turns})");
        assert!(s.iter().all(|c| c.elig));
    }

    #[test]
    fn p3h_p_mono_holds_on_every_candidate_of_random_streams_both_rules_both_sides() {
        let mut x = 777_u64;
        for round in 0..20 {
            let mut ups = Vec::new();
            let mut el = Vec::new();
            for _ in 0..3_000 {
                x = sniper_kernel::scale::splitmix(x);
                ups.push(match x % 9 {
                    0 => None,
                    1..=4 => Some(true),
                    _ => Some(false),
                });
                el.push(!x.is_multiple_of(13));
            }
            let s = st(&ups, &el);
            for long in [true, false] {
                for rule in [Rule::Xs, Rule::Flip] {
                    let ex = exits_all(&s, long, rule);
                    let cand: Vec<usize> = (0..s.len()).filter(|&i| s[i].up.is_some() && s[i].elig).collect();
                    let (g, of) = groups(&s, &cand, &ex, &[]).unwrap_or_else(|e| panic!("round {round} {rule:?} long {long}: {e}"));
                    // FIRST per group never overlaps: each group's first entry is at or after the previous group's exit decision.
                    for w in g.windows(2) {
                        let first_t = s[cand[w[1].first]].end * 1_000_000;
                        assert!(w[0].exit_at == 0 || first_t >= w[0].exit_at);
                    }
                    assert_eq!(of.len(), cand.len());
                }
            }
        }
    }

    #[test]
    fn xs_an_against_side_entry_exits_at_the_next_eligible_candle_across_a_night_and_a_weekend() {
        // Candle 2 (against, F17) is a one-candle group whose exit is the next ELIGIBLE candle (3 and 4 are a special session and a halt
        // END, not eligible; 5 is the next session after a weekend). m_pos_group (grouped with the episode) is red here.
        // ups U U D D U D; candle 3 is not eligible (a special session or a halt END); 4 opens the next session after a weekend.
        let s = st(&[Some(true), Some(true), Some(false), Some(false), Some(true), Some(false)], &[true, true, true, false, true, true]);
        let xs = exits_all(&s, true, Rule::Xs);
        assert_eq!(xs[2], Some(4), "F17: the next ELIGIBLE candle whatever its side");
        let (g, _) = groups(&s, &[0, 1, 2], &xs, &[]).unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[1].n, 1);
    }

    #[test]
    fn groups_are_keyed_by_the_exit_never_the_session_two_positions_of_one_side_are_never_open() {
        // All candles in ONE session: XS gives {0,1} exit 2, {2} exit 3, {3} exit 4: three positions in sequence, never two at once.
        // m_pos_sess (one group per session) is red here.
        let s = st(&[Some(true), Some(true), Some(false), Some(false), Some(true), Some(false)], &[true; 6]);
        let xs = exits_all(&s, true, Rule::Xs);
        let (g, _) = groups(&s, &[0, 1, 2, 3], &xs, &[]).unwrap();
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn flip_a_group_runs_from_one_flip_exit_candle_inclusive_to_the_next_exclusive() {
        // FLIP exits for a long: candidates 0..=6; the flip exit candles are 2 and 6 (DOWN after UP).
        let s = st(&[Some(true), Some(true), Some(false), Some(false), Some(true), Some(true), Some(false), Some(true)], &[true; 8]);
        let fl = exits_all(&s, true, Rule::Flip);
        let (g, of) = groups(&s, &[0, 1, 2, 3, 4, 5], &fl, &[]).unwrap();
        assert_eq!(g.len(), 2);
        // {0,1} exit 2; {2 (inclusive: an against-side entry at that instant),3,4,5} exit 6.
        assert_eq!(of.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 0, 1, 1, 1, 1]);
    }

    #[test]
    fn b5_a_700_candle_group_replicated_100x_keeps_its_first_copy() {
        use sniper_kernel::line::Row;
        let rows: Vec<Row> = (1..=700_u32).map(|k| Row { sess: 0, k, rb: 1, cb: 1, wb: 1 }).collect();
        let (rt, src) = sniper_kernel::scale::replicate(&rows, 100, 7);
        assert_eq!(rt.len(), 70_000);
        // u32 ordinals: no saturation, every copy distinct, and the first copy of the first candle is the smallest (FIRST keeps it).
        let mut ks: Vec<u32> = rt.iter().map(|r| r.k).collect();
        ks.sort_unstable();
        ks.dedup();
        assert_eq!(ks.len(), 70_000);
        let min = rt.iter().enumerate().min_by_key(|(_, r)| r.k).map(|(i, _)| i).unwrap();
        assert_eq!(src[min], 0);
        assert!(rt.iter().all(|r| r.k < u32::MAX));
    }

    #[test]
    fn the_ca_close_caps_a_stretch_and_data_end_is_one_group() {
        let s = st(&[Some(true); 8], &[true; 8]);
        let xs = exits_all(&s, true, Rule::Xs);
        assert!(xs.iter().all(Option::is_none));
        // An ex-date after candle 5 (open at its end + 1 s); close at candle 3's END: candidates before it share the CA close.
        let close_at = s[3].end * 1_000_000;
        let (g, _) = groups(&s, &[0, 1, 2], &xs, &[(s[5].end + 1, close_at)]).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].exit, Exit::CaClose(close_at));
        let (g2, _) = groups(&s, &[0, 1, 2], &xs, &[]).unwrap();
        assert_eq!(g2[0].exit, Exit::DataEnd);
    }
}
