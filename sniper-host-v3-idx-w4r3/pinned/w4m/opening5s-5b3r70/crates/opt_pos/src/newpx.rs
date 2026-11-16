//! RESUMED WAVE 4 (S5 / S6, plan v3.1 s4.3a + s8 S5 "decoder check: every failing contract-day listed and refused when touched,
//! as a LATER refusal"): the PRODUCTION option seconds = DP-NEWPX-IDXOPT (the corrected seconds from the RAW NFO ticks written by
//! `idx-pack newpx`) read back, under the DATA law of lane B's repaired design (POSITIONAL-SEARCH-DESIGN-v2 s5.4, signed in
//! MAIN-SIGNED-POSITIONAL-v2; a data defect never moves FIRST and never removes a row):
//!
//! * a **decoder-failure second** (`DP-NEWPX-FAILS` kinds `both`, `ticks_only`, `store_only`) reads the WORSE of the two sources: a
//!   buy at the higher HIGH, a sale at the lower LOW, volume = the smaller (0 when one source has no trade in that second: no fill
//!   there), and it is a HOLE `[s, s + 1 s)` on the contract-day's tape (crate `Tape::with_holes`), so every order whose blocks
//!   span it carries the HOLE flag;
//! * a contract-day failing as a whole is a **whole-session HOLE**: `missing` (the store has rows, the tick file has no member: no
//!   tick rows, so nothing fills that day and the order waits for the first valid block after the hole), `unchecked` (no store row
//!   at all, e.g. 2026-10-30: the corrected rows are the only source) and `hl_miss` (the corrected day high / low misses the F&O
//!   bhavcopy: rows kept, the session flagged);
//! * `empty` (no member and no store row) is no trade that day in either source: no rows, no hole;
//! * price-only seconds (volume 0: LTQ = 0 new-price rows alone) are not fill rows ("volume alone decides").
//!
//! O(1) per second read: a day file is decoded once (one zstd frame of the day's compact blocks) and indexed by contract; the
//! sidecar is a map keyed (contract, day).

use std::collections::BTreeMap;
use std::path::Path;

use idx_cal::day::{self, Day};
use idx_cal::storeio::at_sod;

use crate::pack::{self, NewSec};
use crate::secs::Sec;

const US_S: i64 = 1_000_000;

/// One day file of DP-NEWPX-IDXOPT, decoded and indexed by contract.
#[derive(Clone, Debug, Default)]
pub struct NewpxDay {
    blocks: BTreeMap<String, Vec<NewSec>>,
}

impl NewpxDay {
    /// Decodes one day file's bytes (a zstd frame of concatenated compact blocks).
    ///
    /// # Errors
    /// A malformed frame or block, or a contract written twice.
    pub fn parse(zst: &[u8]) -> Result<Self, String> {
        let raw = pack::unzst(zst)?;
        let mut blocks = BTreeMap::new();
        let mut at = 0usize;
        while at < raw.len() {
            let (name, secs) = pack::dec_block(&raw, &mut at)?;
            if blocks.insert(name.clone(), secs).is_some() {
                return Err(format!("newpx day file: contract {name} written twice"));
            }
        }
        Ok(Self { blocks })
    }

    /// Opens `<dir>/<YYYY>/<YYYY-MM-DD>.newpx.zst` (`Ok(None)`: the day has no file: nothing of that day is in the pack).
    ///
    /// # Errors
    /// An unreadable or malformed file.
    pub fn open(dir: &Path, d: Day) -> Result<Option<Self>, String> {
        let f = dir.join(day::fmt(d).get(0..4).unwrap_or("0000")).join(format!("{}.newpx.zst", day::fmt(d)));
        match std::fs::read(&f) {
            Ok(b) => Self::parse(&b).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", f.display())),
        }
    }

    /// The contract-day's corrected seconds (`None`: not in the file).
    #[must_use]
    pub fn get(&self, contract: &str) -> Option<&[NewSec]> {
        self.blocks.get(contract).map(Vec::as_slice)
    }

    /// The contracts in the file.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// No contract in the file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// The kind of one sidecar row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailKind {
    /// A second where both sources traded and differ.
    Both,
    /// A second traded in the ticks only.
    TicksOnly,
    /// A second traded in the store only.
    StoreOnly,
    /// The store has rows, the tick file has no member.
    Missing,
    /// No member and no store row.
    Empty,
    /// No store row at all, ticks present.
    Unchecked,
    /// The corrected day high / low misses the F&O bhavcopy.
    HlMiss,
}

impl FailKind {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "both" => Self::Both,
            "ticks_only" => Self::TicksOnly,
            "store_only" => Self::StoreOnly,
            "missing" => Self::Missing,
            "empty" => Self::Empty,
            "unchecked" => Self::Unchecked,
            "hl_miss" => Self::HlMiss,
            _ => return None,
        })
    }
}

/// (high, low, close, volume) of one source in one second.
pub type Quad = (i64, i64, i64, u64);

/// One row of `DP-NEWPX-FAILS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailRow {
    /// The second of day (None: a contract-day row).
    pub sod: Option<i64>,
    /// Its kind.
    pub kind: FailKind,
    /// The store's bar there.
    pub store: Option<Quad>,
    /// The ticks' traded (LTQ > 0) fold there.
    pub ticks: Option<Quad>,
}

/// The sidecar: failure rows keyed (contract, day), each key's rows in file order.
#[derive(Clone, Debug, Default)]
pub struct Fails {
    map: BTreeMap<(String, Day), Vec<FailRow>>,
}

fn quad(c: &[&str], k: usize) -> Option<Quad> {
    let g = |i: usize| c.get(k + i).copied().unwrap_or("-");
    if g(0) == "-" {
        return None;
    }
    Some((g(0).parse().ok()?, g(1).parse().ok()?, g(2).parse().unwrap_or(0), g(3).parse().ok()?))
}

fn hms(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split(':').map(|x| x.parse().ok()).collect::<Option<Vec<i64>>>()?;
    match p.as_slice() {
        [h, m, s] => Some(h * 3600 + m * 60 + s),
        _ => None,
    }
}

impl Fails {
    /// Parses one `DP-NEWPX-FAILS-<IX>.tsv` and adds its rows.
    ///
    /// # Errors
    /// A malformed row (named by line).
    pub fn add(&mut self, text: &str) -> Result<(), String> {
        for (i, l) in text.lines().enumerate() {
            if l.starts_with("contract\t") || l.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            let bad = || format!("DP-NEWPX-FAILS line {}: {l}", i + 1);
            let f = c.first().ok_or_else(bad)?;
            let d = c.get(1).and_then(|x| day::parse(x)).ok_or_else(bad)?;
            let sod = match c.get(2).copied() {
                Some("-") => None,
                Some(x) => Some(hms(x).ok_or_else(bad)?),
                None => return Err(bad()),
            };
            let kind = c.get(3).and_then(|x| FailKind::parse(x)).ok_or_else(bad)?;
            let (store, ticks) = match kind {
                FailKind::Both | FailKind::TicksOnly | FailKind::StoreOnly => (quad(&c, 4), quad(&c, 8)),
                _ => (None, None),
            };
            self.map.entry(((*f).to_owned(), d)).or_default().push(FailRow { sod, kind, store, ticks });
        }
        Ok(())
    }

    /// The rows of one contract-day (empty: it passed every check).
    #[must_use]
    pub fn get(&self, contract: &str, d: Day) -> &[FailRow] {
        self.map.get(&(contract.to_owned(), d)).map_or(&[], Vec::as_slice)
    }

    /// Every failing contract-day key.
    pub fn keys(&self) -> impl Iterator<Item = &(String, Day)> {
        self.map.keys()
    }
}

/// How the contract-day's rows were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Src {
    /// Every check passed: the corrected seconds as written.
    Agree,
    /// That many decoder-failure seconds read the worse of the two sources (holes).
    Seconds(u32),
    /// A whole-session hole (`missing`, `unchecked`, `hl_miss`), plus any failing seconds.
    WholeHole(FailKind, u32),
    /// No trade in either source.
    Empty,
    /// Neither a day block nor a sidecar row: the contract-day is not in the pack (the caller refuses it by name).
    NotInPack,
}

/// One contract-day's production rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayRows {
    /// Fill rows (volume > 0, 0 < LOW <= HIGH), in time order, inside `[from, to)` with the derivatives halt cut.
    pub rows: Vec<Sec>,
    /// Holes (UTC micros, `[from, to)`), merged and sorted.
    pub holes: Vec<(i64, i64)>,
    /// How they were made.
    pub src: Src,
}

/// The worse of two sources in one second for the block law (named mutant `m_newpx_kinder`: the kinder source).
fn worse(a: Quad, b: Quad) -> Quad {
    if cfg!(feature = "m_newpx_kinder") {
        (a.0.min(b.0), a.1.max(b.1), a.2, a.3.max(b.3))
    } else {
        (a.0.max(b.0), a.1.min(b.1), a.2, a.3.min(b.3))
    }
}

/// The DATA law over one contract-day: `newpx` = its corrected seconds (None: no block), `fails` = its sidecar rows, the window
/// `[from, to)` (UTC micros: 09:15:00 to the dated F&O end) and the derivatives halt (cut). See the module text.
#[must_use]
pub fn day_rows(d: Day, newpx: Option<&[NewSec]>, fails: &[FailRow], from: i64, to: i64, halt: Option<(i64, i64)>) -> DayRows {
    let whole = fails.iter().find(|f| matches!(f.kind, FailKind::Missing | FailKind::Unchecked | FailKind::HlMiss)).map(|f| f.kind);
    if fails.iter().any(|f| f.kind == FailKind::Empty) {
        return DayRows { rows: Vec::new(), holes: Vec::new(), src: Src::Empty };
    }
    if newpx.is_none() && fails.is_empty() {
        return DayRows { rows: Vec::new(), holes: Vec::new(), src: Src::NotInPack };
    }
    let ts = |sod: i64| at_sod(d, sod);
    let inside = |t: i64| t >= from && t < to && !halt.is_some_and(|(a, z)| t >= a && t < z);
    // the second-level failures by second
    let per_sec: BTreeMap<i64, &FailRow> = fails.iter().filter_map(|f| f.sod.map(|s| (s, f))).collect();
    let mut rows = Vec::new();
    let mut holes: Vec<(i64, i64)> = Vec::new();
    let mut n_fail = 0u32;
    let mut seen_fail: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    if whole != Some(FailKind::Missing) {
        for &(sod, _o, h, l, c, v) in newpx.unwrap_or(&[]) {
            let s = i64::from(sod);
            let t = ts(s);
            if !inside(t) {
                continue;
            }
            let mut q: Quad = (h, l, c, v);
            if let Some(f) = per_sec.get(&s) {
                seen_fail.insert(s);
                n_fail += 1;
                holes.push((t, t + US_S));
                q = match (f.kind, f.store) {
                    (FailKind::Both, Some(st)) => worse(q, st),
                    // one source has no trade in that second: volume = the smaller = 0
                    _ => (q.0, q.1, q.2, if cfg!(feature = "m_newpx_kinder") { q.3 } else { 0 }),
                };
            }
            if q.3 > 0 && q.1 > 0 && q.0 >= q.1 {
                rows.push(Sec { ts: t, high: q.0, low: q.1, close: q.2, vol: q.3 });
            }
        }
    }
    // failing seconds with no corrected second (store_only, or a both / ticks_only second cut from the block): holes, no fill
    for (&s, _) in per_sec.iter().filter(|(s, _)| !seen_fail.contains(s)) {
        let t = ts(s);
        if inside(t) {
            n_fail += 1;
            holes.push((t, t + US_S));
        }
    }
    if whole.is_some() {
        holes = vec![(from, to)];
    } else {
        holes.sort_unstable();
        let mut merged: Vec<(i64, i64)> = Vec::with_capacity(holes.len());
        for h in holes {
            match merged.last_mut() {
                Some(last) if h.0 <= last.1 => last.1 = last.1.max(h.1),
                _ => merged.push(h),
            }
        }
        holes = merged;
    }
    let src = match whole {
        Some(k) => Src::WholeHole(k, n_fail),
        None if n_fail > 0 => Src::Seconds(n_fail),
        None => Src::Agree,
    };
    DayRows { rows, holes, src }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    use crate::price::{self, Leg};
    use common_fill::std70::{Kind, Order};
    use common_fill::tape::{SecIndex, Seg, Tape};
    use common_fill::Side;

    fn d() -> Day {
        day::parse("2024-05-03").unwrap()
    }
    const O: i64 = 9 * 3600 + 15 * 60;
    fn win() -> (i64, i64) {
        (at_sod(d(), O), at_sod(d(), 15 * 3600 + 30 * 60))
    }
    fn fail(sod: i64, kind: FailKind, store: Option<Quad>, ticks: Option<Quad>) -> FailRow {
        FailRow { sod: Some(sod), kind, store, ticks }
    }
    fn whole(kind: FailKind) -> FailRow {
        FailRow { sod: None, kind, store: None, ticks: None }
    }
    /// three seconds: a widened one (LTQ = 0 rows made its range wider), a price-only one (volume 0) and a plain one
    fn secs3() -> Vec<NewSec> {
        vec![(33_310, 1_000, 1_050, 990, 1_020, 150), (33_311, 1_020, 1_020, 1_020, 1_020, 0), (33_312, 1_020, 1_030, 1_010, 1_015, 75)]
    }

    /// Every check passed: the corrected seconds as written (the widened range kept), the price-only second is no fill row.
    #[test]
    fn data_law_agree_reads_the_corrected_seconds() {
        let (a, z) = win();
        let r = day_rows(d(), Some(&secs3()), &[], a, z, None);
        assert_eq!(r.src, Src::Agree);
        assert!(r.holes.is_empty());
        assert_eq!(r.rows.len(), 2, "the price-only second is not a fill row");
        assert_eq!((r.rows[0].high, r.rows[0].low, r.rows[0].vol), (1_050, 990, 150));
        assert_eq!(r.rows[1].ts, at_sod(d(), 33_312));
    }

    /// A `both` second reads the WORSE of the two sources: the higher HIGH, the lower LOW, the smaller volume; it is a hole.
    #[test]
    fn data_law_both_takes_the_worse_of_two_sources_and_holes_the_second() {
        let (a, z) = win();
        let f = [fail(33_310, FailKind::Both, Some((1_080, 995, 1_000, 90)), Some((1_040, 990, 1_020, 150)))];
        let r = day_rows(d(), Some(&secs3()), &f, a, z, None);
        assert_eq!(r.src, Src::Seconds(1));
        assert_eq!((r.rows[0].high, r.rows[0].low, r.rows[0].vol), (1_080, 990, 90), "max HIGH, min LOW, min volume");
        assert_eq!(r.holes, vec![(at_sod(d(), 33_310), at_sod(d(), 33_311))]);
    }

    /// A second traded in one source only has volume 0 (the smaller): no fill there; both kinds are holes, adjacent ones merged.
    #[test]
    fn data_law_one_source_only_second_has_no_fill() {
        let (a, z) = win();
        let f = [fail(33_312, FailKind::TicksOnly, None, Some((1_030, 1_010, 1_015, 75))), fail(33_313, FailKind::StoreOnly, Some((1_015, 1_015, 1_015, 25)), None)];
        let r = day_rows(d(), Some(&secs3()), &f, a, z, None);
        assert_eq!(r.src, Src::Seconds(2));
        assert_eq!(r.rows.len(), 1, "only the first second fills");
        assert_eq!(r.holes, vec![(at_sod(d(), 33_312), at_sod(d(), 33_314))], "the two holes merge");
    }

    /// `missing` (the store has rows, the ticks no member): no rows and a whole-session hole.
    #[test]
    fn data_law_missing_member_is_a_whole_session_hole_with_no_rows() {
        let (a, z) = win();
        let r = day_rows(d(), None, &[whole(FailKind::Missing)], a, z, None);
        assert_eq!(r.src, Src::WholeHole(FailKind::Missing, 0));
        assert!(r.rows.is_empty());
        assert_eq!(r.holes, vec![(a, z)]);
    }

    /// `unchecked` (2026-10-30: no store row): the corrected rows are kept and the whole session is a hole.
    #[test]
    fn data_law_unchecked_day_keeps_the_ticks_and_holes_the_session() {
        let (a, z) = win();
        let r = day_rows(d(), Some(&secs3()), &[whole(FailKind::Unchecked)], a, z, None);
        assert_eq!(r.src, Src::WholeHole(FailKind::Unchecked, 0));
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.holes, vec![(a, z)]);
        let r2 = day_rows(d(), Some(&secs3()), &[whole(FailKind::HlMiss)], a, z, None);
        assert_eq!((r2.src, r2.rows.len(), r2.holes.len()), (Src::WholeHole(FailKind::HlMiss, 0), 2, 1));
    }

    /// `empty`: no trade in either source; a contract-day absent from both the day file and the sidecar is not in the pack.
    #[test]
    fn data_law_empty_and_not_in_pack() {
        let (a, z) = win();
        let r = day_rows(d(), None, &[whole(FailKind::Empty)], a, z, None);
        assert_eq!((r.src, r.rows.len(), r.holes.len()), (Src::Empty, 0, 0));
        let r2 = day_rows(d(), None, &[], a, z, None);
        assert_eq!(r2.src, Src::NotInPack);
    }

    /// The window and the derivatives halt are cut (a failing second inside the halt is no hole either).
    #[test]
    fn data_law_cuts_the_derivatives_halt_and_the_window() {
        let (a, z) = win();
        let halt = Some((at_sod(d(), 33_310), at_sod(d(), 33_311)));
        let f = [fail(33_310, FailKind::Both, Some((1_080, 995, 1_000, 90)), None)];
        let r = day_rows(d(), Some(&secs3()), &f, a, z, halt);
        assert_eq!((r.rows.len(), r.holes.len(), r.src), (1, 0, Src::Agree));
        let r2 = day_rows(d(), Some(&secs3()), &[], at_sod(d(), 33_312), z, None);
        assert_eq!(r2.rows.len(), 1);
    }

    /// The sidecar and a day file round-trip through the writer's own formats (`idx-pack newpx`).
    #[test]
    fn fails_and_day_file_round_trip() {
        let mut fl = Fails::default();
        let text = "contract\tday\tsod\tkind\ts_high\ts_low\ts_close\ts_vol\tt_high\tt_low\tt_close\tt_vol\n\
            2024-05-08-4900000-CE\t2024-05-03\t09:15:10\tboth\t1080\t995\t1000\t90\t1040\t990\t1020\t150\n\
            2024-05-08-4900000-CE\t2024-05-03\t09:15:13\tstore_only\t1015\t1015\t1015\t25\t-\t-\t-\t0\n\
            2024-05-08-4800000-PE\t2024-05-03\t-\tmissing\t-\t-\t-\t40\t-\t-\t-\t0\n";
        assert!(fl.add(text).is_ok());
        let g = fl.get("2024-05-08-4900000-CE", d());
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], fail(33_310, FailKind::Both, Some((1_080, 995, 1_000, 90)), Some((1_040, 990, 1_020, 150))));
        assert_eq!(fl.get("2024-05-08-4800000-PE", d())[0].kind, FailKind::Missing);
        assert!(fl.add("x\tnot-a-day\t-\tboth\n").is_err());
        let mut raw = pack::enc_block("2024-05-08-4900000-CE", &secs3());
        raw.extend_from_slice(&pack::enc_block("2024-05-08-4800000-PE", &secs3()[..1]));
        let day_file = NewpxDay::parse(&pack::zst(&raw)).unwrap();
        assert_eq!(day_file.len(), 2);
        assert_eq!(day_file.get("2024-05-08-4900000-CE").unwrap(), secs3().as_slice());
        let mut dup = pack::enc_block("A", &secs3());
        dup.extend_from_slice(&pack::enc_block("A", &secs3()));
        assert!(NewpxDay::parse(&pack::zst(&dup)).is_err(), "a contract written twice is refused");
    }

    /// The holes reach the crate walk: an order whose blocks span a failed second carries the HOLE flag; without it, none.
    #[test]
    fn walk_flags_hole_span_on_an_order_over_a_failed_second() {
        let (a, z) = win();
        let f = [fail(33_312, FailKind::Both, Some((1_040, 1_000, 1_015, 50)), Some((1_030, 1_010, 1_015, 75)))];
        let r = day_rows(d(), Some(&secs3()), &f, a, z, None);
        let segs = [Seg { open: a, last_sec: z - US_S, auction_open: false }];
        let ix = SecIndex::new(&r.rows, a, z).unwrap();
        let o = Order { side: Side::Buy, anchor: at_sod(d(), 33_310), stop: z, unit: 25, limit: None, kind: Kind::Entry };
        let with = Tape::new(&r.rows, &ix, None).unwrap().with_segments(&segs).unwrap().with_block(1).unwrap().with_holes(&r.holes);
        let (fills, lf) = price::walk1(&with, &o, 175, None, 10_000, Leg::Entry).unwrap();
        assert!(lf.hole_span, "the order spans the failed second");
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[1].4, 1_040, "the second block fills at the worse (higher) HIGH");
        let without = Tape::new(&r.rows, &ix, None).unwrap().with_segments(&segs).unwrap().with_block(1).unwrap();
        let (_, lf2) = price::walk1(&without, &o, 175, None, 10_000, Leg::Entry).unwrap();
        assert!(!lf2.hole_span);
    }
}
