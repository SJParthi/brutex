//! NIFTY 50 for the market filters E08-E10 (SEARCH-SPEC 3.6): Zerodha's stored index
//! minutes as they are (APPROXIMATE 34), opened read-only through the store's own door.

use std::collections::BTreeMap;
use std::path::Path;

use brutex_core::vendor::Vendor;
use indicators::trend::{Ema, SCALE};
use pull::calendar::{DayKind, Runtime, Session};
use pull::fold::{Bucket, derive_minutes};
use pull::vendor::Venue;
use store::format::Bar;
use store::path::Timeframe;

use crate::common::{CLOSE_FULL, N_TF, OPEN_MIN, TFS, WIN_LAST_MI, at_min, ist_min_of, month_index};
use crate::phase_a::{MonthFile, by_day, open_month, read_minutes, sym_id};

/// NIFTY 50's minutes and readings.
#[derive(Debug, Default)]
pub(crate) struct Nifty {
    /// Per IST day: the close of each stored minute by minute index from 09:15.
    days: BTreeMap<i64, Vec<Option<i64>>>,
    /// Per M2 day with stored minutes: its last stored minute's close.
    last: BTreeMap<i64, i64>,
    /// Per timeframe: (candle close instant, +1 close above EMA 20 / -1 below / 0 equal),
    /// for candles that hold a bar, are not holes and whose EMA is warm and untainted.
    ema: Vec<Vec<(i64, i8)>>,
    pub(crate) months_read: u32,
    pub(crate) months_missing: u32,
    pub(crate) months_refused: u32,
    pub(crate) minutes: u64,
}

impl Nifty {
    /// An empty NIFTY (every E08-E10 test false), for unit tests.
    pub(crate) fn empty() -> Self {
        Self { ema: (0..N_TF).map(|_| Vec::new()).collect(), ..Self::default() }
    }

    /// Load 2018-01 ..= 2026-10.
    pub(crate) fn load(store: &Path) -> Result<Self, String> {
        let mut me = Self::empty();
        let id = sym_id("NIFTY")?;
        let mut minutes: Vec<Bar> = Vec::new();
        let mut withheld = 0_u64;
        for mi in 0..=WIN_LAST_MI {
            match open_month(store, Vendor::Zerodha, "INDEX", "NIFTY", id, Timeframe::MINUTE_1, mi) {
                MonthFile::Open(f) => {
                    let v = read_minutes(&f, &mut withheld)?;
                    me.months_read += 1;
                    minutes.extend(v);
                }
                MonthFile::Missing => me.months_missing += 1,
                MonthFile::Refused(why) => {
                    me.months_refused += 1;
                    return Err(format!("NIFTY month refused: {why}"));
                }
            }
        }
        me.minutes = u64::try_from(minutes.len()).unwrap_or(0);
        for &(d, a, b) in &by_day(&minutes) {
            let mut closes = vec![None; usize::try_from(CLOSE_FULL - OPEN_MIN).unwrap_or(375)];
            let mut lastc = None;
            for bar in minutes.get(a..b).unwrap_or(&[]) {
                let j = ist_min_of(bar.ts_micros) - OPEN_MIN;
                if let Ok(j) = usize::try_from(j)
                    && let Some(slot) = closes.get_mut(j)
                {
                    *slot = Some(bar.close);
                    lastc = Some(bar.close);
                }
            }
            if Runtime::default().kind_of(d) == DayKind::Open(Session::full())
                && let Some(c) = lastc
            {
                me.last.insert(d, c);
            }
            me.days.insert(d, closes);
        }
        // EMA 20 on each timeframe's NIFTY candles, folded month by month in order.
        let groups = by_day(&minutes);
        for (ti, &k) in TFS.iter().enumerate() {
            let secs = u32::try_from(60 * k).map_err(|_| "bucket".to_owned())?;
            let bucket = Bucket::of_secs(secs).ok_or_else(|| format!("bucket {secs} refused"))?;
            let dv = derive_minutes(&minutes, bucket, Venue::NseIndex, None, Runtime::default())
                .map_err(|e| format!("NIFTY {k}min fold refused: {e}"))?;
            let dg = by_day(&dv.bars);
            let mut ema = Ema::new(20);
            let mut since = u32::MAX;
            let mut out: Vec<(i64, i8)> = Vec::new();
            for &(d, a, b) in &dg {
                let bars = dv.bars.get(a..b).unwrap_or(&[]);
                let m2 = Runtime::default().kind_of(d) == DayKind::Open(Session::full());
                if !m2 {
                    for bar in bars {
                        ema.fold(bar.close);
                        since = since.saturating_add(1);
                    }
                    continue;
                }
                let closes = me.days.get(&d);
                let nb = (CLOSE_FULL - OPEN_MIN + k - 1).div_euclid(k);
                let mut cur = 0_usize;
                for bi in 0..nb {
                    let sm = OPEN_MIN + bi * k;
                    let em = (sm + k).min(CLOSE_FULL);
                    let t_open = at_min(d, sm);
                    while bars.get(cur).is_some_and(|x| x.ts_micros < t_open) {
                        cur += 1;
                    }
                    let bar = bars.get(cur).filter(|x| x.ts_micros == t_open).copied();
                    if bar.is_some() {
                        cur += 1;
                    }
                    let lo = usize::try_from(sm - OPEN_MIN).unwrap_or(0);
                    let hi = usize::try_from(em - OPEN_MIN).unwrap_or(0);
                    let missing = closes.is_none_or(|c| c.get(lo..hi).is_none_or(|w| w.iter().any(Option::is_none)));
                    match bar {
                        Some(bar) => {
                            ema.fold(bar.close);
                            if missing {
                                since = 0;
                            } else {
                                since = since.saturating_add(1);
                                if ema.warm() && since >= 20 {
                                    let cs = i128::from(bar.close) * SCALE;
                                    let es = ema.scaled();
                                    let side = if cs > es {
                                        1
                                    } else if cs < es {
                                        -1
                                    } else {
                                        0
                                    };
                                    out.push((at_min(d, em), side));
                                }
                            }
                        }
                        None => since = 0,
                    }
                }
            }
            let _ = &groups;
            if let Some(slot) = me.ema.get_mut(ti) {
                *slot = out;
            }
        }
        Ok(me)
    }

    /// The previous M2 session's last stored NIFTY minute close (E08's PN).
    pub(crate) fn prev_close(&self, d: i64) -> Option<i64> {
        self.last.range(..d).next_back().map(|(_, c)| *c)
    }

    /// Entry k: the close of the latest stored NIFTY minute among the first k minutes
    /// of day `d` (a minute that ends at or before 09:15 + k).
    pub(crate) fn last_closes(&self, d: i64) -> Vec<Option<i64>> {
        let Some(c) = self.days.get(&d) else { return Vec::new() };
        let mut out = Vec::with_capacity(c.len() + 1);
        let mut last = None;
        out.push(None);
        for x in c {
            if x.is_some() {
                last = *x;
            }
            out.push(last);
        }
        out
    }

    /// E09: the NIFTY candle of timeframe `ti` closing at `t` against its EMA 20.
    pub(crate) fn ema_side(&self, ti: usize, t: i64) -> Option<i8> {
        let v = self.ema.get(ti)?;
        let i = v.binary_search_by_key(&t, |x| x.0).ok()?;
        v.get(i).map(|x| x.1)
    }

    /// Ready E09 candles per timeframe (for the report).
    pub(crate) fn ema_counts(&self) -> Vec<usize> {
        self.ema.iter().map(Vec::len).collect()
    }
}

/// Self-check (SEARCH-SPEC 12.3): NIFTY 2024-03 through `derive_minutes(.., NseIndex, ..)`
/// gives 125 three-minute buckets on each full session holding all 375 minutes.
pub(crate) fn nifty_grid_check(store: &Path) -> Result<String, String> {
    let id = sym_id("NIFTY")?;
    let mi = month_index(2024, 3).ok_or("2024-03")?;
    let f = match open_month(store, Vendor::Zerodha, "INDEX", "NIFTY", id, Timeframe::MINUTE_1, mi) {
        MonthFile::Open(f) => f,
        MonthFile::Missing => return Err("NIFTY 2024-03 is not in the store".to_owned()),
        MonthFile::Refused(why) => return Err(why),
    };
    let mut withheld = 0_u64;
    let minutes = read_minutes(&f, &mut withheld)?;
    let groups = by_day(&minutes);
    let bucket = Bucket::of_secs(180).ok_or("bucket 180 refused")?;
    let dv = derive_minutes(&minutes, bucket, Venue::NseIndex, None, Runtime::default())
        .map_err(|e| format!("NIFTY 2024-03 3min fold refused: {e}"))?;
    let dg = by_day(&dv.bars);
    let (mut full, mut short) = (0_u32, 0_u32);
    for &(d, a, b) in &groups {
        if Runtime::default().kind_of(d) != DayKind::Open(Session::full()) {
            continue;
        }
        if b - a != 375 {
            short += 1;
            continue;
        }
        full += 1;
        let n = dg.iter().find(|g| g.0 == d).map_or(0, |g| g.2 - g.1);
        if n != 125 {
            return Err(format!("NIFTY 2024-03 day {d}: {n} three-minute buckets, expected 125"));
        }
    }
    if full == 0 {
        return Err("NIFTY 2024-03: no full session with all 375 minutes".to_owned());
    }
    Ok(format!(
        "NIFTY 2024-03 through derive_minutes(NseIndex): {full} full sessions with all 375 minutes, each 125 three-minute buckets; {short} regular sessions short of 375 minutes: OK"
    ))
}
