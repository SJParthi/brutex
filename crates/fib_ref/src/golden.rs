//! The golden scenarios: seeded synthetic inputs and every expected output,
//! written as plain text a separate program can read
//! (`reports/fib-preview-ref-20261107/golden/FORMAT.md` is the format).
//!
//! Each scenario is one `.md` file whose body is one fenced `text` block of
//! records, one per line: `fibref-golden 1`, then `id`, `kind`, `seed`,
//! `readings`, `title`, then `cols <record> <names...>` and `in <record>
//! <values...>` for bulk inputs, `in <record> k=v ...` for the rest, `out
//! <record> k=v ...` for every expected output, and `end`. Prices are paise,
//! rung prices milli-paise (`_milli`), times `HH:MM:SS`, an absent value `-`.

use std::fmt::Write as _;
use std::path::Path;

use crate::account::{self, AccountOut, CEILING_B, Ceilings, Leverage, MAIN, Outcome, Signal};
use crate::bars::{Candle, Minute, Sec, fold};
use crate::charges::{self, Breakdown, Order, OrderSide, Priced};
use crate::clock::{Date, OPEN, SESSION_END, SESSION_END_AUCTION, hms, minute_start, month_label};
use crate::e1::{self, E1Params, Holding, LadderMode, Levels, SecondFeed};
use crate::e2::{self, E2Params, Reading, TouchClause};
use crate::e3::{self, BreakClause, E3Params, E3Reading};
use crate::gates;
use crate::ladder::{self, ExtremeTies, Ladder, RUNGS, TaintScope};
use crate::model::{
    AfterBreak, BelowHalf, BoxAnchor, DataChargeAt, DdRound, InterimLine, LockEnd, OptionBrokerage,
    OrbBoundary, Readings, RestingAfterCut, Retouch, Side, StateStamp, Status, TargetLadder,
    WithinSecond, ZeroFillCount, map_level, r1_at, stop_toward, target_toward,
};
use crate::money::{LAKH, rs};
use crate::rank::{self, Mover, MoverSign};
use crate::synth::{SplitMix64, random_minutes, seconds_for_all};
use crate::walk::{self, Exit, Plan, WalkOut};
use crate::wf::{self, ChainMonth, Key, Series, SeriesTrade, Twin, VerdictIn};

/// The format's first line.
pub const FORMAT: &str = "fibref-golden 1";

/// One scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    /// Scenario id, also the file's stem.
    pub id: String,
    /// The kind of records it holds.
    pub kind: &'static str,
    /// One line of plain words.
    pub title: String,
    /// The generator's seed.
    pub seed: u64,
    /// The readings it runs.
    pub readings: Readings,
    lines: Vec<String>,
}

impl Doc {
    /// A scenario with no records yet.
    pub fn new(id: &str, kind: &'static str, title: &str, seed: u64, readings: Readings) -> Doc {
        Doc {
            id: id.to_owned(),
            kind,
            title: title.to_owned(),
            seed,
            readings,
            lines: Vec::new(),
        }
    }

    fn push(&mut self, s: String) {
        self.lines.push(s);
    }

    fn rec(&mut self, dir: &str, head: &str, pairs: &[(&str, String)]) {
        let mut s = format!("{dir} {head}");
        for (k, v) in pairs {
            let _ = write!(s, " {k}={v}");
        }
        self.lines.push(s);
    }

    fn inp(&mut self, head: &str, pairs: &[(&str, String)]) {
        self.rec("in", head, pairs);
    }

    fn out(&mut self, head: &str, pairs: &[(&str, String)]) {
        self.rec("out", head, pairs);
    }

    /// The file name, `<id>.md`.
    pub fn file_name(&self) -> String {
        format!("{}.md", self.id)
    }

    /// The file's text.
    pub fn render(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "# {}: {}", self.id, self.title);
        let _ = writeln!(s);
        let _ = writeln!(s, "{}", crate::PREVIEW);
        let _ = writeln!(s);
        let _ = writeln!(s, "```text");
        let _ = writeln!(s, "{FORMAT}");
        let _ = writeln!(s, "id {}", self.id);
        let _ = writeln!(s, "kind {}", self.kind);
        let _ = writeln!(s, "seed {}", self.seed);
        let _ = writeln!(s, "readings {}", self.readings.label());
        let _ = writeln!(s, "title {}", self.title);
        for l in &self.lines {
            let _ = writeln!(s, "{l}");
        }
        let _ = writeln!(s, "end");
        let _ = writeln!(s, "```");
        s
    }

    /// The number of `out` records.
    pub fn outputs(&self) -> usize {
        self.lines.iter().filter(|l| l.starts_with("out ")).count()
    }
}

fn o<T: ToString>(x: Option<T>) -> String {
    x.map_or_else(|| "-".to_owned(), |v| v.to_string())
}

fn ot(x: Option<u32>) -> String {
    x.map_or_else(|| "-".to_owned(), hms)
}

fn bit(x: bool) -> String {
    if x { "1" } else { "0" }.to_owned()
}

fn put_minutes(d: &mut Doc, ms: &[Minute]) {
    d.push("cols minute m o h l c v held padded g".to_owned());
    for m in ms {
        d.push(format!(
            "in minute {} {} {} {} {} {} {} {} {}",
            m.m,
            m.o,
            m.h,
            m.l,
            m.c,
            m.v,
            u8::from(m.held),
            u8::from(m.padded),
            o(m.g)
        ));
    }
}

fn put_seconds(d: &mut Doc, ss: &[Sec]) {
    d.push("cols second t o h l c v".to_owned());
    for s in ss {
        d.push(format!(
            "in second {} {} {} {} {} {}",
            hms(s.t),
            s.o,
            s.h,
            s.l,
            s.c,
            s.v
        ));
    }
}

fn put_top(d: &mut Doc, top: &[bool]) {
    let s: String = top.iter().map(|&b| if b { '1' } else { '0' }).collect();
    d.inp("top", &[("by_candle", s)]);
}

fn put_candles(d: &mut Doc, cs: &[Candle]) {
    for c in cs {
        d.out(
            "candle",
            &[
                ("i", c.i.to_string()),
                ("close_t", hms(c.close_t)),
                ("o", c.o.to_string()),
                ("h", c.h.to_string()),
                ("l", c.l.to_string()),
                ("c", c.c.to_string()),
                ("incomplete", bit(c.incomplete)),
            ],
        );
    }
}

fn milli(x: Option<i128>) -> String {
    o(x)
}

/// Levels and their R1 mapping (rounded against the position) at `t`.
fn put_levels(
    d: &mut Doc,
    head: &str,
    lv: &Levels,
    side: Side,
    minutes: &[Minute],
    t: u32,
    step: i64,
) {
    let r1 = r1_at(minutes, t);
    let map = |x: Option<i128>, stop: bool| -> String {
        match (x, r1) {
            (Some(v), Some((z, g))) => map_level(
                v,
                z,
                g,
                step,
                if stop {
                    stop_toward(side)
                } else {
                    target_toward(side)
                },
            )
            .to_string(),
            (Some(_), None) => "no_level_ratio".to_owned(),
            (None, _) => "no_ladder_target".to_owned(),
        }
    };
    d.out(
        head,
        &[
            ("pattern_extreme", lv.pattern_extreme.to_string()),
            ("stop_milli", lv.stop_milli.to_string()),
            ("t1_milli", lv.t1_milli.to_string()),
            ("t2_milli", milli(lv.t2_milli)),
            ("t3_milli", milli(lv.t3_milli)),
            ("r1_z", o(r1.map(|x| x.0))),
            ("r1_g", o(r1.map(|x| x.1))),
            ("stop_g", map(Some(lv.stop_milli), true)),
            ("t1_g", map(Some(lv.t1_milli), false)),
            ("t2_g", map(lv.t2_milli, false)),
            ("t3_g", map(lv.t3_milli, false)),
        ],
    );
}

fn side_name(s: Side) -> String {
    s.name().to_owned()
}

/// Bars `(o, h, l, c)` in paise expanded to `tf` minutes each from minute
/// `first`: the first minute opens at the bar's open, one minute prints its
/// high and one its low, the last closes at its close.
pub fn expand(
    rng: &mut SplitMix64,
    bars: &[(i64, i64, i64, i64)],
    tf: u32,
    step: i64,
    first: u32,
) -> Vec<Minute> {
    let mut out = Vec::new();
    let mut m = first;
    for &(bo, bh, bl, bc) in bars {
        let k = tf.max(1) as usize;
        let hi_at = usize::try_from(rng.below(k as u64)).unwrap_or(0);
        let mut lo_at = usize::try_from(rng.below(k as u64)).unwrap_or(0);
        if k > 1 && lo_at == hi_at {
            lo_at = (hi_at + 1) % k;
        }
        let ticks = ((bh - bl) / step.max(1)).max(0);
        let mut prev = bo;
        for j in 0..k {
            let close = if j + 1 == k {
                bc
            } else {
                bl + step * rng.range(0, ticks)
            };
            let mut h = prev.max(close);
            let mut l = prev.min(close);
            if j == hi_at {
                h = bh;
            }
            if j == lo_at {
                l = bl;
            }
            out.push(Minute {
                m,
                o: prev,
                h,
                l,
                c: close,
                v: rng.range(500, 5_000),
                held: true,
                padded: false,
                g: Some(close),
            });
            prev = close;
            m += 1;
        }
    }
    out
}

/// Random bars from `start` in paise on the tick grid.
fn rand_bars(
    rng: &mut SplitMix64,
    start: i64,
    n: usize,
    step: i64,
    drift: i64,
    vol: i64,
) -> Vec<(i64, i64, i64, i64)> {
    let mut px = start;
    (0..n)
        .map(|_| {
            let o = px;
            let c = o + step * (drift + rng.range(-vol, vol));
            let h = o.max(c) + step * rng.range(0, 2);
            let l = o.min(c) - step * rng.range(0, 2);
            px = c;
            (o, h, l, c)
        })
        .collect()
}

/// A script of tick offsets laid at `anchor` (mirrored for a short).
fn lay(
    script: &[(i64, i64, i64, i64)],
    anchor: i64,
    step: i64,
    side: Side,
) -> Vec<(i64, i64, i64, i64)> {
    script
        .iter()
        .map(|&(so, sh, sl, sc)| {
            if side.long() {
                (
                    anchor + so * step,
                    anchor + sh * step,
                    anchor + sl * step,
                    anchor + sc * step,
                )
            } else {
                (
                    anchor - so * step,
                    anchor - sl * step,
                    anchor - sh * step,
                    anchor - sc * step,
                )
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// rank
// ---------------------------------------------------------------------------

fn rank_doc(id: &str, title: &str, seed: u64, pool: &[Mover], limit: Option<u32>) -> Doc {
    let mut d = Doc::new(id, "rank", title, seed, Readings::HEADLINE);
    d.push("cols mover sym p c stale_min".to_owned());
    for m in pool {
        d.push(format!(
            "in mover {} {} {} {}",
            m.sym, m.p, m.c, m.stale_min
        ));
    }
    d.inp("stale_limit", &[("minutes", o(limit))]);
    let f = rank::frame(pool, limit);
    for r in &f {
        let tops = |sign: MoverSign| -> String {
            [1, 3, 5, 10, 25, 50]
                .iter()
                .map(|&n| {
                    format!(
                        "{}{}",
                        u8::from(r.in_top_long(n, sign)),
                        u8::from(r.in_top_short(n, sign))
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        d.out(
            "rank",
            &[
                ("sym", r.sym.to_string()),
                ("gainer", r.gainer.to_string()),
                ("loser", r.loser.to_string()),
                ("sign", r.sign.to_string()),
                ("stale", bit(r.stale)),
                (
                    "top_n1_3_5_10_25_50_long_short_rank_only",
                    tops(MoverSign::RankOnly),
                ),
                (
                    "top_n1_3_5_10_25_50_long_short_signed_only",
                    tops(MoverSign::SignedOnly),
                ),
            ],
        );
    }
    d
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn rank_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    for k in 0..10u64 {
        let seed = 1_000 + k;
        let mut rng = SplitMix64::new(seed);
        let n = usize::try_from(rng.range(5, 40)).unwrap_or(5);
        let mut pool = Vec::new();
        for sym in 0..n {
            let c = rng.range(2_000, 400_000);
            let mv = rng.range(-500, 500); // basis points
            let p = c + c * mv / 10_000;
            pool.push(Mover {
                sym: u32::try_from(sym).unwrap_or(0),
                p,
                c,
                stale_min: u32::try_from(rng.range(0, 3)).unwrap_or(0) * 10,
            });
        }
        // a tie by construction: the same move on two different prices
        // (every seeded pool holds at least five stocks)
        pool[1].c = 200_000;
        pool[1].p = 210_000;
        pool[2].c = 100_000;
        pool[2].p = 105_000;
        let limit = if k % 3 == 0 { Some(15) } else { None };
        out.push(rank_doc(
            &format!("R{:03}", k + 1),
            "competition ranks on a seeded pool with an exact tie",
            seed,
            &pool,
            limit,
        ));
    }
    let falling: Vec<Mover> = (0..12)
        .map(|s| Mover {
            sym: s,
            p: 100_000 - i64::from(s) * 100,
            c: 100_000,
            stale_min: 0,
        })
        .collect();
    out.push(rank_doc(
        "R011",
        "a falling market: the sign readings differ for the long pool (A01)",
        1_011,
        &falling,
        None,
    ));
    let equal: Vec<Mover> = (0..8)
        .map(|s| Mover {
            sym: s,
            p: 50_000 * (i64::from(s) + 1) * 102 / 100,
            c: 50_000 * (i64::from(s) + 1),
            stale_min: 0,
        })
        .collect();
    out.push(rank_doc(
        "R012",
        "eight equal moves share rank 1 both ways",
        1_012,
        &equal,
        None,
    ));
    out.push(rank_doc(
        "R013",
        "one stock is first both ways",
        1_013,
        &[Mover {
            sym: 4,
            p: 123_400,
            c: 120_000,
            stale_min: 0,
        }],
        None,
    ));
    let invalid = vec![
        Mover {
            sym: 1,
            p: 105_000,
            c: 100_000,
            stale_min: 0,
        },
        Mover {
            sym: 2,
            p: 99_000,
            c: 0,
            stale_min: 0,
        },
        Mover {
            sym: 3,
            p: 120_000,
            c: 100_000,
            stale_min: 40,
        },
        Mover {
            sym: 4,
            p: 101_000,
            c: 100_000,
            stale_min: 2,
        },
    ];
    out.push(rank_doc(
        "R014",
        "no previous close leaves the pool; a stale price over the limit leaves it",
        1_014,
        &invalid,
        Some(30),
    ));
    let edge: Vec<Mover> = (0..15)
        .map(|s| Mover {
            sym: s,
            p: 100_000 + if s < 12 { 1_000 } else { 500 },
            c: 100_000,
            stale_min: 0,
        })
        .collect();
    out.push(rank_doc(
        "R015",
        "a twelve-way tie at the edge of top 10 includes all twelve",
        1_015,
        &edge,
        None,
    ));
    out
}

// ---------------------------------------------------------------------------
// ladder
// ---------------------------------------------------------------------------

fn ladder_doc(id: &str, title: &str, seed: u64, minutes: &[Minute], tf: u32, r: Readings) -> Doc {
    let mut d = Doc::new(id, "ladder", title, seed, r);
    d.inp("tf", &[("minutes", tf.to_string())]);
    put_minutes(&mut d, minutes);
    let candles = fold(minutes, tf);
    put_candles(&mut d, &candles);
    let ladders = ladder::running(&candles, r.ties, r.taint);
    for (i, l) in ladders.iter().enumerate() {
        put_ladder(&mut d, i, l);
    }
    d
}

fn put_ladder(d: &mut Doc, i: usize, l: &Ladder) {
    d.out(
        "ladder",
        &[
            ("before_candle", i.to_string()),
            ("hi", l.hi.to_string()),
            ("lo", l.lo.to_string()),
            ("hi_ix", l.hi_ix.to_string()),
            ("lo_ix", l.lo_ix.to_string()),
            ("leg", l.leg.name().to_owned()),
            ("tainted", bit(l.tainted)),
            ("usable", bit(l.usable())),
        ],
    );
    if l.usable() {
        let rungs: Vec<String> = RUNGS.iter().map(|&x| milli(l.rung_milli(x))).collect();
        d.out(
            "rungs",
            &[
                ("before_candle", i.to_string()),
                (
                    "milli_0_236_382_500_618_786_1000_1272_1618_2000_2618_3000",
                    rungs.join(","),
                ),
            ],
        );
    }
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn ladder_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let tfs = [1u32, 2, 3, 5, 10, 15, 30, 60];
    for (k, &tf) in tfs.iter().enumerate() {
        let seed = 2_000 + k as u64;
        let mut rng = SplitMix64::new(seed);
        let n = if tf >= 15 { 375 } else { (24 * tf).min(375) };
        let mut ms = random_minutes(&mut rng, 150_000, 5, n, 0, 0, 6, 3);
        // an outside minute late in the day holds both extremes
        let last = ms.len() - 1;
        let (hi, lo) = ms
            .iter()
            .fold((0, i64::MAX), |(h, l), m| (h.max(m.h), l.min(m.l)));
        ms[last].h = hi + 50;
        ms[last].l = lo - 50;
        out.push(ladder_doc(
            &format!("L{:03}", k + 1),
            &format!("the running ladder on {tf} minute candles, a last candle with both extremes"),
            seed,
            &ms,
            tf,
            Readings::HEADLINE,
        ));
    }
    // both extremes in one candle, then one-sided extremes
    let mut rng = SplitMix64::new(2_100);
    let bars = vec![
        (10_000, 10_100, 9_900, 10_050),
        (10_050, 10_080, 10_000, 10_060),
        (10_060, 10_150, 10_040, 10_140),
        (10_140, 10_145, 9_850, 9_860),
        (9_860, 10_200, 9_800, 10_190),
    ];
    let ms = expand(&mut rng, &bars, 1, 5, 0);
    out.push(ladder_doc(
        "L009",
        "a candle with both extremes: Undetermined until a later one-sided extreme",
        2_100,
        &ms,
        1,
        Readings::HEADLINE,
    ));
    // equal extremes under the two tie readings
    let bars = vec![
        (10_000, 10_100, 9_900, 10_050),
        (10_050, 10_120, 10_000, 10_110),
        (10_110, 10_110, 9_900, 9_950),
        (9_950, 10_120, 9_940, 10_100),
    ];
    let ms = expand(&mut rng, &bars, 1, 5, 0);
    out.push(ladder_doc(
        "L010",
        "equal extremes, strict reading (A02 headline)",
        2_101,
        &ms,
        1,
        Readings::HEADLINE,
    ));
    let mut r = Readings::HEADLINE;
    r.ties = ExtremeTies::Latest;
    out.push(ladder_doc(
        "L011",
        "equal extremes, latest reading (A02 other)",
        2_101,
        &ms,
        1,
        r,
    ));
    // a hole inside the range under the two taint readings
    let mut rng = SplitMix64::new(2_102);
    let mut ms = random_minutes(&mut rng, 80_000, 5, 30, 0, 1, 5, 2);
    ms[12].held = false;
    out.push(ladder_doc(
        "L012",
        "a Kite hole taints the ladder for the session (A03 headline)",
        2_102,
        &ms,
        1,
        Readings::HEADLINE,
    ));
    let mut r = Readings::HEADLINE;
    r.taint = TaintScope::ExtremeCandles;
    out.push(ladder_doc(
        "L013",
        "the same hole under the extreme-candles reading (A03 other)",
        2_102,
        &ms,
        1,
        r,
    ));
    let mut ms2 = ms.clone();
    ms2[12].held = true;
    ms2[12].padded = true; // padded with a GDFL trade: a hole too
    out.push(ladder_doc(
        "L014",
        "a padded minute in which GDFL traded is a hole",
        2_103,
        &ms2,
        3,
        Readings::HEADLINE,
    ));
    ms2[12].g = None; // padded, nobody traded: held
    out.push(ladder_doc(
        "L015",
        "a padded minute with no GDFL trade is held",
        2_104,
        &ms2,
        3,
        Readings::HEADLINE,
    ));
    let mut rng = SplitMix64::new(2_105);
    let down = random_minutes(&mut rng, 120_000, 5, 40, 0, -2, 4, 2);
    out.push(ladder_doc(
        "L016",
        "a falling day: a Down leg and its extensions below the low",
        2_105,
        &down,
        2,
        Readings::HEADLINE,
    ));
    out
}

// ---------------------------------------------------------------------------
// E1
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Day {
    minutes: Vec<Minute>,
    secs: Vec<Sec>,
}

/// A day: a random prefix, a scripted pattern at the prefix's last close,
/// a random suffix, at timeframe `tf`; seconds over `sec_window` candles.
fn build_day(
    seed: u64,
    tf: u32,
    side: Side,
    prefix: usize,
    script: &[(i64, i64, i64, i64)],
    suffix: usize,
    with_secs: bool,
) -> Day {
    let mut rng = SplitMix64::new(seed);
    let step = 5;
    let mut bars = rand_bars(&mut rng, 100_000, prefix, step, 0, 3);
    let anchor = bars.last().map_or(100_000, |b| b.3);
    bars.extend(lay(script, anchor, step, side));
    let after = bars.last().map_or(anchor, |b| b.3);
    bars.extend(rand_bars(&mut rng, after, suffix, step, 0, 2));
    let minutes = expand(&mut rng, &bars, tf, step, 0);
    let secs = if with_secs {
        seconds_for_all(&mut rng, &minutes, step, 400, 400)
    } else {
        Vec::new()
    };
    Day { minutes, secs }
}

/// The operator's flag in ticks: a three-bar push to a new high (+16), a
/// box of `k` bars below it (highs at most +15, lows +12), a green break.
fn flag_script(k: usize) -> Vec<(i64, i64, i64, i64)> {
    let mut s = vec![(0, 6, -1, 5), (5, 11, 4, 10), (10, 16, 9, 15)];
    for j in 0..k {
        s.push(if j % 2 == 0 {
            (15, 15, 12, 13)
        } else {
            (13, 14, 12, 14)
        });
    }
    s.push((14, 19, 13, 18));
    s
}

#[allow(clippy::too_many_arguments, reason = "one scenario's inputs")]
fn e1_doc(
    id: &str,
    title: &str,
    seed: u64,
    day: &Day,
    tf: u32,
    top: &[bool],
    p: &E1Params,
    r: Readings,
) -> Doc {
    let mut d = Doc::new(id, "e1", title, seed, r);
    d.inp("tf", &[("minutes", tf.to_string())]);
    d.inp(
        "params",
        &[
            ("side", side_name(p.side)),
            ("k", p.k.to_string()),
            ("rung", p.rung.to_string()),
            ("holding", p.holding.name().to_owned()),
            (
                "mode",
                if p.mode == LadderMode::Running {
                    "running"
                } else {
                    "frozen"
                }
                .to_owned(),
            ),
            ("max_height_ppm", o(p.max_height_ppm)),
            ("colour", bit(p.colour)),
            ("stop_offset", p.stop_offset.to_string()),
            ("t1_offset", p.t1_offset.to_string()),
            ("step", "5".to_owned()),
            ("data_start", hms(OPEN)),
        ],
    );
    put_top(&mut d, top);
    put_minutes(&mut d, &day.minutes);
    if !day.secs.is_empty() {
        put_seconds(&mut d, &day.secs);
    }
    let candles = fold(&day.minutes, tf);
    put_candles(&mut d, &candles);
    for e in e1::e1c(&candles, top, p, &r) {
        d.out(
            "e1c",
            &[
                ("candle", e.candle.to_string()),
                ("t", hms(e.t)),
                ("ext", e.ext.to_string()),
                ("box_first", e.box_first.to_string()),
                ("box_high", e.box_high.to_string()),
                ("box_low", e.box_low.to_string()),
                ("status", e.status.name().to_owned()),
            ],
        );
        if let Some(lv) = e.levels {
            put_levels(&mut d, "e1c.levels", &lv, p.side, &day.minutes, e.t, 5);
        }
    }
    if !day.secs.is_empty() {
        let feed = SecondFeed {
            minutes: &day.minutes,
            secs: &day.secs,
            step: 5,
            data_start: OPEN,
        };
        for e in e1::e1s(&candles, top, &feed, p, &r) {
            d.out(
                "e1s",
                &[
                    ("candle", e.candle.to_string()),
                    ("armed_at", hms(e.armed_at)),
                    ("trigger", o(e.trigger)),
                    ("trigger_s", ot(e.trigger_s)),
                    ("t", ot(e.t)),
                    ("ref", o(e.ref_px)),
                    ("status", e.status.name().to_owned()),
                ],
            );
            if let (Some(lv), Some(t)) = (e.levels, e.t) {
                put_levels(&mut d, "e1s.levels", &lv, p.side, &day.minutes, t, 5);
            }
        }
    }
    d
}

fn e1_params(side: Side, k: u32) -> E1Params {
    E1Params {
        side,
        k,
        rung: 382,
        holding: Holding::AllLows,
        mode: LadderMode::Running,
        max_height_ppm: None,
        colour: true,
        stop_offset: 5,
        t1_offset: 5,
    }
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn e1_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut next = |out: &mut Vec<Doc>, d: Doc| {
        n += 1;
        let mut d = d;
        d.id = format!("E1-{n:03}");
        out.push(d);
    };
    for (k, side) in [
        (1u32, Side::Long),
        (2, Side::Long),
        (3, Side::Long),
        (4, Side::Long),
        (1, Side::Short),
        (2, Side::Short),
        (3, Side::Short),
    ] {
        let seed = 3_000 + u64::from(k) + if side.long() { 0 } else { 100 };
        let day = build_day(seed, 1, side, 12, &flag_script(k as usize), 8, false);
        let top = vec![true; day.minutes.len()];
        next(
            &mut out,
            e1_doc(
                "",
                &format!(
                    "the operator's flag, K = {k}, {}: the break at candle {} inside a seeded random day",
                    side.name(),
                    15 + k
                ),
                seed,
                &day,
                1,
                &top,
                &e1_params(side, k),
                Readings::HEADLINE,
            ),
        );
    }
    for tf in [2u32, 3, 5, 15] {
        let seed = 3_200 + u64::from(tf);
        let day = build_day(seed, tf, Side::Long, 6, &flag_script(2), 4, false);
        let candles = fold(&day.minutes, tf);
        let top = vec![true; candles.len()];
        next(
            &mut out,
            e1_doc(
                "",
                &format!("the flag on {tf} minute candles"),
                seed,
                &day,
                tf,
                &top,
                &e1_params(Side::Long, 2),
                Readings::HEADLINE,
            ),
        );
    }
    // clauses
    let seed = 3_300;
    let day = build_day(seed, 1, Side::Long, 12, &flag_script(2), 8, false);
    let mut top = vec![true; day.minutes.len()];
    for t in &mut top[14..20] {
        *t = false;
    }
    next(
        &mut out,
        e1_doc(
            "",
            "not in top N at the break: refused by the rank clause",
            seed,
            &day,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    let top = vec![true; day.minutes.len()];
    let mut red = flag_script(2);
    let last = red.len() - 1;
    red[last] = (18, 19, 13, 16); // closes above the box high, red
    let rday = build_day(3_301, 1, Side::Long, 12, &red, 8, false);
    next(
        &mut out,
        e1_doc(
            "",
            "a red break candle: refused by the colour clause",
            3_301,
            &rday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e1_doc(
            "",
            "the same red break with the colour clause off",
            3_301,
            &rday,
            1,
            &top,
            &E1Params {
                colour: false,
                ..e1_params(Side::Long, 2)
            },
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e1_doc(
            "",
            "a tight clause the box fails (height in ppm of R)",
            seed,
            &day,
            1,
            &top,
            &E1Params {
                max_height_ppm: Some(10_000),
                ..e1_params(Side::Long, 2)
            },
            Readings::HEADLINE,
        ),
    );
    let mut deep = flag_script(2);
    deep[3] = (15, 15, -40, 13); // a box low far below 0.382
    let deep_day = build_day(3_302, 1, Side::Long, 12, &deep, 8, false);
    for (holding, what) in [
        (Holding::AllLows, "all lows"),
        (Holding::AllCloses, "all closes"),
        (
            Holding::NoCloseBelowFirstLow,
            "no close below the first low",
        ),
        (Holding::Off, "none"),
    ] {
        next(
            &mut out,
            e1_doc(
                "",
                &format!("a deep box under the holding rule '{what}'"),
                3_302,
                &deep_day,
                1,
                &top,
                &E1Params {
                    holding,
                    ..e1_params(Side::Long, 2)
                },
                Readings::HEADLINE,
            ),
        );
    }
    next(
        &mut out,
        e1_doc(
            "",
            "frozen ladder mode at the box's first candle",
            seed,
            &day,
            1,
            &top,
            &E1Params {
                mode: LadderMode::Frozen,
                ..e1_params(Side::Long, 2)
            },
            Readings::HEADLINE,
        ),
    );
    // a hole in the box voids the scope; a hole before taints the ladder
    let mut hday = build_day(3_303, 1, Side::Long, 12, &flag_script(2), 8, false);
    hday.minutes[16].held = false;
    next(
        &mut out,
        e1_doc(
            "",
            "a Kite hole inside the box voids the scope (incomplete_candle)",
            3_303,
            &hday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    let mut tday = build_day(3_304, 1, Side::Long, 12, &flag_script(2), 8, false);
    tday.minutes[3].held = false;
    next(
        &mut out,
        e1_doc(
            "",
            "a hole early in the day taints the ladder: tainted_ladder (A03 headline)",
            3_304,
            &tday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.taint = TaintScope::ExtremeCandles;
    next(
        &mut out,
        e1_doc(
            "",
            "the same early hole under the extreme-candles taint (A03 other)",
            3_304,
            &tday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            r,
        ),
    );
    // A05 and A06: after a break without a new high, and the box anchor
    // the push tops at +16; a first break closes at +16 (above the box high
    // +15, equal to the session high: no new extreme); a second box (+15
    // high) breaks the same way two bars later
    let mut two = flag_script(2);
    let l = two.len() - 1;
    two[l] = (14, 16, 13, 16);
    two.push((15, 15, 13, 14));
    two.push((14, 15, 13, 15));
    two.push((15, 16, 14, 16));
    let mut aday = build_day(3_305, 1, Side::Long, 12, &two, 0, false);
    // a flat tail that makes no new high
    let last_c = aday.minutes.last().map_or(100_000, |m| m.c);
    let mut tail_rng = SplitMix64::new(3_305_000);
    let tail: Vec<(i64, i64, i64, i64)> = (0..6)
        .map(|k| {
            (
                last_c - 10,
                last_c - 5,
                last_c - 20 - 5 * (k % 2),
                last_c - 10,
            )
        })
        .collect();
    let first = u32::try_from(aday.minutes.len()).unwrap_or(0);
    aday.minutes
        .extend(expand(&mut tail_rng, &tail, 1, 5, first));
    let top = vec![true; aday.minutes.len()];
    next(
        &mut out,
        e1_doc(
            "",
            "breaks without a new high: consumed until a new extreme (A05 headline)",
            3_305,
            &aday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.after_break = AfterBreak::NextBox;
    next(
        &mut out,
        e1_doc(
            "",
            "the same breaks re-armed on the next box (A05 other)",
            3_305,
            &aday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            r,
        ),
    );
    let mut slow = flag_script(2);
    let l = slow.len() - 1;
    slow[l] = (14, 15, 12, 13);
    slow.push((13, 16, 12, 15));
    slow.push((15, 15, 12, 15));
    let sday = build_day(3_306, 1, Side::Long, 12, &slow, 6, false);
    let top = vec![true; sday.minutes.len()];
    next(
        &mut out,
        e1_doc(
            "",
            "a slow box: the sliding box (A06 headline)",
            3_306,
            &sday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.box_anchor = BoxAnchor::Anchored;
    next(
        &mut out,
        e1_doc(
            "",
            "the same slow box, anchored after the push (A06 other)",
            3_306,
            &sday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            r,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.target_ladder = TargetLadder::Judging;
    next(
        &mut out,
        e1_doc(
            "",
            "rung targets on the judging ladder (A17 other)",
            seed,
            &day,
            1,
            &top_all(&day),
            &e1_params(Side::Long, 2),
            r,
        ),
    );
    // second reading
    for (k, side) in [(2u32, Side::Long), (3, Side::Short)] {
        let seed = 3_400 + u64::from(k);
        let day = build_day(seed, 1, side, 8, &flag_script(k as usize), 6, true);
        let top = vec![true; day.minutes.len()];
        next(
            &mut out,
            e1_doc(
                "",
                &format!(
                    "E1S, the first second beyond the box, K = {k}, {}",
                    side.name()
                ),
                seed,
                &day,
                1,
                &top,
                &e1_params(side, k),
                Readings::HEADLINE,
            ),
        );
    }
    let mut qday = build_day(3_410, 1, Side::Long, 8, &flag_script(2), 6, true);
    for m in &mut qday.minutes[8..] {
        m.g = None;
    }
    let top = vec![true; qday.minutes.len()];
    next(
        &mut out,
        e1_doc(
            "",
            "E1S with no GDFL price in R1's minute: no_trigger_ratio",
            3_410,
            &qday,
            1,
            &top,
            &e1_params(Side::Long, 2),
            Readings::HEADLINE,
        ),
    );
    out
}

fn top_all(day: &Day) -> Vec<bool> {
    vec![true; day.minutes.len()]
}

// ---------------------------------------------------------------------------
// E2
// ---------------------------------------------------------------------------

/// The MAZDOCK shape in ticks: low at -10, high at +90 (R 100 ticks; 0.786
/// at +11.4, 0.618 at +28.2), a fall to a first touch, a bounce past
/// 0.618, a return to 0.786.
fn touch_script() -> Vec<(i64, i64, i64, i64)> {
    vec![
        (0, 4, -10, 2),
        (2, 40, 1, 38),
        (38, 90, 36, 85), // the high
        (85, 86, 30, 32), // clear above 0.786
        (32, 33, 15, 18), // clear
        (18, 19, 11, 14), // touches 11.4: n = 1
        (14, 22, 13, 21), // clear
        (21, 30, 20, 29), // clear, bounce 30 >= 28.2: passed
        (29, 29, 16, 17), // clear
        (17, 18, 10, 12), // touches again: the return, n = 2
        (12, 20, 12, 19),
        (19, 31, 18, 30), // passes again
        (30, 30, 12, 13),
        (13, 14, 11, 12), // touches: n = 3
        (12, 18, 12, 17),
    ]
}

fn e2_params(side: Side) -> E2Params {
    E2Params {
        side,
        rung: 786,
        tol_milli: 0,
        bounce_m: 1,
        clause: TouchClause::Eq(2),
        mode: LadderMode::Running,
        close_beyond_ends: false,
        reading: Reading::Close,
        stop_offset: 5,
        t1_offset: 5,
        xj_offset: 0,
    }
}

/// A touch day: the script from the open (no prefix: the script sets the
/// day's extremes), then a random suffix.
fn touch_day(
    seed: u64,
    tf: u32,
    side: Side,
    script: &[(i64, i64, i64, i64)],
    with_secs: bool,
) -> Day {
    let mut rng = SplitMix64::new(seed);
    let step = 5;
    let mut bars = lay(script, 100_000, step, side);
    let after = bars.last().map_or(100_000, |b| b.3);
    let mid = 100_000 + if side.long() { 60 } else { -60 } * step;
    let tail: Vec<(i64, i64, i64, i64)> = rand_bars(&mut rng, after, 4, step, 0, 1)
        .into_iter()
        .map(|(o2, h, l, c)| {
            let clamp = |x: i64| x.clamp(mid - 30 * step, mid + 30 * step);
            (clamp(o2), clamp(h), clamp(l), clamp(c))
        })
        .collect();
    bars.extend(tail);
    let minutes = expand(&mut rng, &bars, tf, step, 0);
    let secs = if with_secs {
        seconds_for_all(&mut rng, &minutes, step, 450, 300)
    } else {
        Vec::new()
    };
    Day { minutes, secs }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one scenario's inputs and its printed records"
)]
fn e2_doc(
    id: &str,
    title: &str,
    seed: u64,
    day: &Day,
    tf: u32,
    top: &[bool],
    p: &E2Params,
    r: Readings,
) -> Doc {
    let mut d = Doc::new(id, "e2", title, seed, r);
    d.inp("tf", &[("minutes", tf.to_string())]);
    d.inp(
        "params",
        &[
            ("side", side_name(p.side)),
            ("rung", p.rung.to_string()),
            ("tol_milli", p.tol_milli.to_string()),
            ("bounce_m", p.bounce_m.to_string()),
            ("clause", p.clause.name()),
            (
                "mode",
                if p.mode == LadderMode::Running {
                    "running"
                } else {
                    "frozen"
                }
                .to_owned(),
            ),
            ("close_beyond_ends", bit(p.close_beyond_ends)),
            (
                "reading",
                match p.reading {
                    Reading::Close => "close",
                    Reading::Second => "second",
                    Reading::Resting => "resting",
                }
                .to_owned(),
            ),
            ("stop_offset", p.stop_offset.to_string()),
            ("t1_offset", p.t1_offset.to_string()),
            ("xj_offset", p.xj_offset.to_string()),
            ("step", "5".to_owned()),
            ("data_start", hms(OPEN)),
        ],
    );
    put_top(&mut d, top);
    put_minutes(&mut d, &day.minutes);
    if !day.secs.is_empty() {
        put_seconds(&mut d, &day.secs);
    }
    let candles = fold(&day.minutes, tf);
    put_candles(&mut d, &candles);
    let feed = SecondFeed {
        minutes: &day.minutes,
        secs: &day.secs,
        step: 5,
        data_start: OPEN,
    };
    let res = e2::e2(&candles, top, &feed, p, &r);
    for t in &res.trace {
        d.out(
            "e2.book",
            &[
                ("candle", t.candle.to_string()),
                ("level_milli", milli(t.level_milli)),
                ("rel", o(t.rel.map(crate::ladder::Relation::name))),
                ("phase", t.phase.name().to_owned()),
                ("n", t.n.to_string()),
                ("bounce", o(t.bounce)),
                ("passed", bit(t.passed)),
                (
                    "event",
                    if t.event.is_empty() { "-" } else { t.event }.to_owned(),
                ),
                ("gap", o(t.gap)),
            ],
        );
    }
    for s in &res.signals {
        d.out(
            "e2.signal",
            &[
                (
                    "reading",
                    match s.reading {
                        Reading::Close => "close",
                        Reading::Second => "second",
                        Reading::Resting => "resting",
                    }
                    .to_owned(),
                ),
                ("n", s.n.to_string()),
                ("candle", s.candle.to_string()),
                ("armed_at", ot(s.armed_at)),
                ("trigger", o(s.trigger)),
                (
                    "end",
                    s.end
                        .map_or_else(|| "-".to_owned(), |(t, why)| format!("{}:{why}", hms(t))),
                ),
                ("trigger_s", ot(s.trigger_s)),
                ("t", ot(s.t)),
                ("ref", o(s.ref_px)),
                ("status", s.status.name().to_owned()),
            ],
        );
        if let (Some(lv), Some(t)) = (s.levels, s.t) {
            let as_e1 = Levels {
                pattern_extreme: lv.pattern_extreme,
                stop_milli: lv.stop_milli,
                t1_milli: lv.t1_milli,
                t2_milli: lv.t2_milli,
                t3_milli: lv.t3_milli,
            };
            put_levels(&mut d, "e2.levels", &as_e1, p.side, &day.minutes, t, 5);
            let xj = match (lv.xj_milli, r1_at(&day.minutes, t)) {
                (Some(x), Some((z, g))) => map_level(x, z, g, 5, target_toward(p.side)).to_string(),
                (Some(_), None) => "no_level_ratio".to_owned(),
                (None, _) => "-".to_owned(),
            };
            d.out(
                "e2.xj",
                &[
                    ("level_milli", lv.level_milli.to_string()),
                    ("xj_milli", milli(lv.xj_milli)),
                    ("xj_g", xj),
                ],
            );
        }
    }
    d
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn e2_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut next = |out: &mut Vec<Doc>, d: Doc| {
        n += 1;
        let mut d = d;
        d.id = format!("E2-{n:03}");
        out.push(d);
    };
    let base = touch_script();
    for side in [Side::Long, Side::Short] {
        let seed = 4_000 + u64::from(side == Side::Short);
        let day = touch_day(seed, 1, side, &base, false);
        let top = top_all(&day);
        next(
            &mut out,
            e2_doc(
                "",
                &format!("the second touch of 0.786, close reading, {}", side.name()),
                seed,
                &day,
                1,
                &top,
                &e2_params(side),
                Readings::HEADLINE,
            ),
        );
    }
    let day = touch_day(4_010, 1, Side::Long, &base, false);
    let top = top_all(&day);
    for (clause, what) in [
        (TouchClause::Eq(1), "= 1, the baseline"),
        (TouchClause::Eq(3), "= 3"),
        (TouchClause::AtLeast(2), ">= 2"),
        (TouchClause::AtLeast(3), ">= 3"),
    ] {
        next(
            &mut out,
            e2_doc(
                "",
                &format!("touch clause {what}"),
                4_010,
                &day,
                1,
                &top,
                &E2Params {
                    clause,
                    ..e2_params(Side::Long)
                },
                Readings::HEADLINE,
            ),
        );
    }
    next(
        &mut out,
        e2_doc(
            "",
            "bounce m = 2: 0.5 must be reached",
            4_010,
            &day,
            1,
            &top,
            &E2Params {
                bounce_m: 2,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "rung 0.618",
            4_010,
            &day,
            1,
            &top,
            &E2Params {
                rung: 618,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "a tolerance of 2 paise",
            4_010,
            &day,
            1,
            &top,
            &E2Params {
                tol_milli: 2_000,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "frozen at the first touch",
            4_010,
            &day,
            1,
            &top,
            &E2Params {
                mode: LadderMode::Frozen,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    // the first touch candle closes below 0.786 (+11.4): S29b ends the count there
    let mut below = base.clone();
    below[5] = (18, 19, 10, 11);
    let sday = touch_day(4_011, 1, Side::Long, &below, false);
    next(
        &mut out,
        e2_doc(
            "",
            "a touch candle closing below the level: the seed reading counts on",
            4_011,
            &sday,
            1,
            &top_all(&sday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "the same close below under S29b's labelled reading: the count ends",
            4_011,
            &sday,
            1,
            &top_all(&sday),
            &E2Params {
                close_beyond_ends: true,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "the resting reading is refused by name",
            4_010,
            &day,
            1,
            &top,
            &E2Params {
                reading: Reading::Resting,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    let mut half = top.clone();
    for t in &mut half[8..11] {
        *t = false;
    }
    next(
        &mut out,
        e2_doc(
            "",
            "out of top N at the return",
            4_010,
            &day,
            1,
            &half,
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    // the return's minute has no GDFL trade: its levels and XJ have no R1 price
    let mut nog_ret = day.clone();
    nog_ret.minutes[9].g = None;
    next(
        &mut out,
        e2_doc(
            "",
            "the return's minute has no GDFL trade: no_level_ratio on every mapped level",
            4_010,
            &nog_ret,
            1,
            &top,
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    // a through candle wipes the bounce: the whole candle below 0.786
    let mut thru = base.clone();
    thru[8] = (9, 10, 0, 2);
    let tday = touch_day(4_020, 1, Side::Long, &thru, false);
    next(
        &mut out,
        e2_doc(
            "",
            "a through candle wipes the bounce before the return",
            4_020,
            &tday,
            1,
            &top_all(&tday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    // a hole ends the count
    let mut hday = touch_day(4_021, 1, Side::Long, &base, false);
    hday.minutes[7].held = false;
    next(
        &mut out,
        e2_doc(
            "",
            "a Kite hole ends the count: touch_count_unknown",
            4_021,
            &hday,
            1,
            &top_all(&hday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    // a new extreme changes the signature under running, not under frozen
    let mut ext = base.clone();
    ext[7] = (21, 30, 20, 29);
    ext.insert(8, (29, 29, 8, 9)); // a new... no: keep the low; make a new high instead
    ext[8] = (29, 95, 28, 30); // a new high: the running signature changes
    let eday = touch_day(4_022, 1, Side::Long, &ext, false);
    next(
        &mut out,
        e2_doc(
            "",
            "a new high mid-count restarts the running count",
            4_022,
            &eday,
            1,
            &top_all(&eday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e2_doc(
            "",
            "the same new high under the frozen ladder keeps the count",
            4_022,
            &eday,
            1,
            &top_all(&eday),
            &E2Params {
                mode: LadderMode::Frozen,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    // A07: a re-touch before the bounce passes
    let mut re = base.clone();
    re[6] = (14, 18, 13, 17);
    re[7] = (17, 17, 11, 12); // touches again before the bounce passed
    re[8] = (12, 31, 12, 30); // passes
    let rday = touch_day(4_023, 1, Side::Long, &re, false);
    next(
        &mut out,
        e2_doc(
            "",
            "a re-touch before the bounce passes joins the touch (A07 headline)",
            4_023,
            &rday,
            1,
            &top_all(&rday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.retouch = Retouch::Ignore;
    next(
        &mut out,
        e2_doc(
            "",
            "the same re-touch ignored for the count (A07 other)",
            4_023,
            &rday,
            1,
            &top_all(&rday),
            &e2_params(Side::Long),
            r,
        ),
    );
    // the first touch must come from the favourable side
    let mut from_below = base.clone();
    from_below[3] = (85, 86, 0, 2);
    from_below[4] = (2, 9, 0, 8); // below 0.786
    from_below[5] = (8, 14, 7, 13); // touches from below: not a first touch
    let bday = touch_day(4_024, 1, Side::Long, &from_below, false);
    next(
        &mut out,
        e2_doc(
            "",
            "a touch from the unfavourable side is not a first touch",
            4_024,
            &bday,
            1,
            &top_all(&bday),
            &e2_params(Side::Long),
            Readings::HEADLINE,
        ),
    );
    // timeframes
    for tf in [2u32, 3] {
        let seed = 4_030 + u64::from(tf);
        let day = touch_day(seed, tf, Side::Long, &base, false);
        let candles = fold(&day.minutes, tf);
        next(
            &mut out,
            e2_doc(
                "",
                &format!("the touch book on {tf} minute candles"),
                seed,
                &day,
                tf,
                &vec![true; candles.len()],
                &e2_params(Side::Long),
                Readings::HEADLINE,
            ),
        );
    }
    // second reading
    for side in [Side::Long, Side::Short] {
        let seed = 4_040 + u64::from(side == Side::Short);
        let day = touch_day(seed, 1, side, &base, true);
        next(
            &mut out,
            e2_doc(
                "",
                &format!(
                    "E2S armed at the bounce, triggered by a GDFL second, {}",
                    side.name()
                ),
                seed,
                &day,
                1,
                &top_all(&day),
                &E2Params {
                    reading: Reading::Second,
                    ..e2_params(side)
                },
                Readings::HEADLINE,
            ),
        );
    }
    let mut late = touch_day(4_042, 1, Side::Long, &base, true);
    late.secs.retain(|s| s.t >= OPEN + 9 * 60);
    next(
        &mut out,
        e2_doc(
            "",
            "E2S with GDFL seconds missing until the return candle",
            4_042,
            &late,
            1,
            &top_all(&late),
            &E2Params {
                reading: Reading::Second,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    let mut nog = touch_day(4_043, 1, Side::Long, &base, true);
    nog.minutes[7].g = None;
    nog.minutes[11].g = None;
    next(
        &mut out,
        e2_doc(
            "",
            "E2S with no GDFL price in R1's minute: no_trigger_ratio",
            4_043,
            &nog,
            1,
            &top_all(&nog),
            &E2Params {
                reading: Reading::Second,
                ..e2_params(Side::Long)
            },
            Readings::HEADLINE,
        ),
    );
    out
}

// ---------------------------------------------------------------------------
// E3
// ---------------------------------------------------------------------------

/// An opening-range day in minutes: `d` minutes inside a band of `width`
/// ticks, then a break and a fall back and a second break.
fn orb_minutes(seed: u64, d: u32, width: i64, side: Side, n: u32) -> Vec<Minute> {
    let mut rng = SplitMix64::new(seed);
    let step = 5;
    let base = 100_000;
    let sgn = if side.long() { 1 } else { -1 };
    let mut out = Vec::new();
    let mut px = base;
    for m in 0..n {
        let (o2, c) = if m < d {
            let c = base + step * rng.range(0, width);
            (px, c)
        } else if m < d + 3 {
            (px, base + step * rng.range(1, width - 1))
        } else if m == d + 3 {
            (
                px,
                base + sgn * step * (width + 4) + if side.long() { 0 } else { step * width },
            )
        } else if m < d + 6 {
            (px, base + step * rng.range(1, width - 1))
        } else if m == d + 6 {
            (
                px,
                base + sgn * step * (width + 6) + if side.long() { 0 } else { step * width },
            )
        } else {
            (px, px + step * rng.range(-2, 2))
        };
        let (h, l) = if m < d {
            (base + step * width, base)
        } else {
            (
                o2.max(c) + step * rng.range(0, 1),
                o2.min(c) - step * rng.range(0, 1),
            )
        };
        let (h, l) = if m < d {
            (h.max(o2.max(c)), l.min(o2.min(c)))
        } else {
            (h, l)
        };
        out.push(Minute {
            m,
            o: o2,
            h,
            l,
            c,
            v: rng.range(500, 5_000),
            held: true,
            padded: false,
            g: Some(c),
        });
        px = c;
    }
    out
}

#[allow(clippy::too_many_arguments, reason = "one scenario's inputs")]
fn e3_doc(
    id: &str,
    title: &str,
    seed: u64,
    minutes: &[Minute],
    secs: &[Sec],
    tf: u32,
    p: &E3Params,
    r: Readings,
) -> Doc {
    e3_doc_at(id, title, seed, minutes, secs, tf, p, r, OPEN)
}

#[allow(clippy::too_many_arguments, reason = "one scenario's inputs")]
fn e3_doc_at(
    id: &str,
    title: &str,
    seed: u64,
    minutes: &[Minute],
    secs: &[Sec],
    tf: u32,
    p: &E3Params,
    r: Readings,
    data_start: u32,
) -> Doc {
    let mut d = Doc::new(id, "e3", title, seed, r);
    d.inp("tf", &[("minutes", tf.to_string())]);
    d.inp(
        "params",
        &[
            ("side", side_name(p.side)),
            ("d", p.d.to_string()),
            (
                "reading",
                if p.reading == E3Reading::Close {
                    "close"
                } else {
                    "second"
                }
                .to_owned(),
            ),
            ("clause", p.clause.name().to_owned()),
            ("stop_offset", p.stop_offset.to_string()),
            ("t1_offset", p.t1_offset.to_string()),
            ("step", "5".to_owned()),
            ("data_start", hms(data_start)),
        ],
    );
    let candles = fold(minutes, tf);
    let top = vec![true; candles.len()];
    put_top(&mut d, &top);
    put_minutes(&mut d, minutes);
    if !secs.is_empty() {
        put_seconds(&mut d, secs);
    }
    put_candles(&mut d, &candles);
    let feed = SecondFeed {
        minutes,
        secs,
        step: 5,
        data_start,
    };
    let res = e3::e3(&candles, &top, &feed, p, &r);
    d.out(
        "e3.range",
        &[
            ("d", res.range.d.to_string()),
            (
                "h",
                if res.range.h == i64::MIN {
                    "-".to_owned()
                } else {
                    res.range.h.to_string()
                },
            ),
            (
                "l",
                if res.range.l == i64::MAX {
                    "-".to_owned()
                } else {
                    res.range.l.to_string()
                },
            ),
            ("close_t", hms(res.range.close_t)),
            ("status", res.range.status.name().to_owned()),
        ],
    );
    for e in &res.events {
        d.out(
            "e3.event",
            &[
                (
                    "reading",
                    if e.reading == E3Reading::Close {
                        "close"
                    } else {
                        "second"
                    }
                    .to_owned(),
                ),
                ("candle", e.candle.to_string()),
                ("ordinal", e.ordinal.to_string()),
                ("armed_at", ot(e.armed_at)),
                ("trigger", o(e.trigger)),
                ("trigger_s", ot(e.trigger_s)),
                ("t", ot(e.t)),
                ("ref", o(e.ref_px)),
                ("status", e.status.name().to_owned()),
            ],
        );
        if let (Some(lv), Some(t)) = (e.levels, e.t) {
            put_levels(&mut d, "e3.levels", &lv, p.side, minutes, t, 5);
        }
    }
    d
}

fn e3_params(side: Side, d: u32) -> E3Params {
    E3Params {
        side,
        d,
        reading: E3Reading::Close,
        clause: BreakClause::First,
        stop_offset: 5,
        t1_offset: 5,
    }
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn e3_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut next = |out: &mut Vec<Doc>, d: Doc| {
        n += 1;
        let mut d = d;
        d.id = format!("E3-{n:03}");
        out.push(d);
    };
    for (k, &d) in e3::DURATIONS.iter().enumerate() {
        let side = if k % 2 == 0 { Side::Long } else { Side::Short };
        let seed = 5_000 + u64::from(d);
        let ms = orb_minutes(seed, d, 12, side, d + 30);
        let tf = if d >= 15 { 5 } else { 1 };
        next(
            &mut out,
            e3_doc(
                "",
                &format!(
                    "ORB {d} minutes on {tf} minute candles, close reading, {}",
                    side.name()
                ),
                seed,
                &ms,
                &[],
                tf,
                &e3_params(side, d),
                Readings::HEADLINE,
            ),
        );
    }
    let ms = orb_minutes(5_100, 5, 12, Side::Long, 40);
    for clause in [
        BreakClause::Second,
        BreakClause::SecondOrLater,
        BreakClause::Any,
    ] {
        next(
            &mut out,
            e3_doc(
                "",
                &format!("ORB 5, the break ordinal clause '{}'", clause.name()),
                5_100,
                &ms,
                &[],
                1,
                &E3Params {
                    clause,
                    ..e3_params(Side::Long, 5)
                },
                Readings::HEADLINE,
            ),
        );
    }
    let mut holed = ms.clone();
    holed[2].held = false;
    next(
        &mut out,
        e3_doc(
            "",
            "a hole inside the range: orb_tainted",
            5_101,
            &holed,
            &[],
            1,
            &e3_params(Side::Long, 5),
            Readings::HEADLINE,
        ),
    );
    let mut flat = ms.clone();
    for m in &mut flat[..1] {
        m.h = m.o;
        m.l = m.o;
        m.c = m.o;
    }
    next(
        &mut out,
        e3_doc(
            "",
            "a one-minute range with no width: orb_no_width",
            5_102,
            &flat,
            &[],
            1,
            &e3_params(Side::Long, 1),
            Readings::HEADLINE,
        ),
    );
    // second reading with seconds
    for (k, side) in [(0u64, Side::Long), (1, Side::Short)] {
        let seed = 5_200 + k;
        let ms = orb_minutes(seed, 3, 10, side, 25);
        let mut rng = SplitMix64::new(seed + 50);
        let secs = seconds_for_all(&mut rng, &ms, 5, 450, 300);
        next(
            &mut out,
            e3_doc(
                "",
                &format!("E3S, the first second beyond the range, {}", side.name()),
                seed,
                &ms,
                &secs,
                1,
                &E3Params {
                    reading: E3Reading::Second,
                    ..e3_params(side, 3)
                },
                Readings::HEADLINE,
            ),
        );
    }
    // A08: the candle closing exactly at the range's close
    let ms = orb_minutes(5_300, 5, 10, Side::Long, 30);
    let mut rng = SplitMix64::new(5_350);
    let secs = seconds_for_all(&mut rng, &ms, 5, 450, 300);
    next(
        &mut out,
        e3_doc(
            "",
            "E3S on 5 minute candles, ORB 5: the boundary candle writes no record (A08 headline)",
            5_300,
            &ms,
            &secs,
            5,
            &E3Params {
                reading: E3Reading::Second,
                ..e3_params(Side::Long, 5)
            },
            Readings::HEADLINE,
        ),
    );
    let mut r = Readings::HEADLINE;
    r.orb_boundary = OrbBoundary::Arms;
    next(
        &mut out,
        e3_doc(
            "",
            "the same, the boundary candle arms (A08 other)",
            5_300,
            &ms,
            &secs,
            5,
            &E3Params {
                reading: E3Reading::Second,
                ..e3_params(Side::Long, 5)
            },
            r,
        ),
    );
    let late: Vec<Sec> = secs
        .iter()
        .copied()
        .filter(|s| s.t >= OPEN + 20 * 60)
        .collect();
    next(
        &mut out,
        e3_doc(
            "",
            "E3S with GDFL seconds only from 09:35",
            5_301,
            &ms,
            &late,
            1,
            &E3Params {
                reading: E3Reading::Second,
                ..e3_params(Side::Long, 5)
            },
            Readings::HEADLINE,
        ),
    );
    let mut gone = orb_minutes(5_103, 1, 12, Side::Long, 20);
    gone[0].held = false;
    next(
        &mut out,
        e3_doc(
            "",
            "the range's only minute is missing: orb_tainted, with no price at all",
            5_103,
            &gone,
            &[],
            1,
            &e3_params(Side::Long, 1),
            Readings::HEADLINE,
        ),
    );
    next(
        &mut out,
        e3_doc_at(
            "",
            "E3S with the GDFL data start at 10:15: every arming refused by name, no_fill_data_window",
            5_104,
            &ms,
            &secs,
            1,
            &E3Params {
                reading: E3Reading::Second,
                ..e3_params(Side::Long, 5)
            },
            Readings::HEADLINE,
            OPEN + 3_600,
        ),
    );
    out
}

// ---------------------------------------------------------------------------
// gates
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments, reason = "one gate case's inputs")]
fn gate_doc(
    id: &str,
    title: &str,
    seed: u64,
    side: Side,
    z: i64,
    g: i64,
    refpx: i64,
    stop_milli: i128,
    target_milli: Option<i128>,
    amount: i64,
    date: Date,
    given: Option<i128>,
) -> Doc {
    let r = Readings::HEADLINE;
    let mut d = Doc::new(id, "gate", title, seed, r);
    d.inp(
        "gate",
        &[
            ("side", side_name(side)),
            ("z", z.to_string()),
            ("g", g.to_string()),
            ("ref", refpx.to_string()),
            ("stop_milli", stop_milli.to_string()),
            ("target_milli", milli(target_milli)),
            ("amount", amount.to_string()),
            ("date", date.iso()),
            ("given_charges", o(given)),
        ],
    );
    let (ds, dt) = gates::distances(side, z, g, refpx, stop_milli, target_milli);
    let qty = gates::planned_qty(amount, refpx);
    let p = gates::planned_charges(date, qty, refpx, &r);
    let w = gates::planned_charges_worst(date, qty, refpx, &r);
    let show = |x: Result<i128, Status>| match x {
        Ok(v) => v.to_string(),
        Err(e) => e.name().to_owned(),
    };
    d.out(
        "gate",
        &[
            ("d_stop", ds.to_string()),
            ("d_tgt", milli(dt)),
            ("s_stop_ppm", show(gates::s_stop(ds, refpx))),
            ("s_rr_price", show(gates::s_rr_price(ds, dt))),
            ("qty", qty.to_string()),
            ("charges_official", p.to_string()),
            ("charges_worst", w.to_string()),
            ("s_rr_net_official", show(gates::s_rr_net(ds, dt, qty, p))),
            ("s_rr_net_worst", show(gates::s_rr_net(ds, dt, qty, w))),
            (
                "s_rr_net_given",
                given.map_or_else(|| "-".to_owned(), |c| show(gates::s_rr_net(ds, dt, qty, c))),
            ),
        ],
    );
    d
}

/// One S-STOP and S-RR case: side, Z, G, REF, stop at Z, target at Z, why.
type GateRow = (Side, i64, i64, i64, i64, Option<i64>, &'static str);

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn gate_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let d0909 = Date {
        y: 2026,
        m: 9,
        d: 9,
    };
    let rows: [GateRow; 6] = [
        (
            Side::Long,
            100_000,
            100_020,
            100_020,
            99_695,
            Some(100_605),
            "SNIPER.md 1.2 long, normal",
        ),
        (
            Side::Short,
            100_000,
            99_980,
            99_980,
            100_305,
            Some(99_395),
            "SNIPER.md 1.2 short, normal",
        ),
        (
            Side::Long,
            100_000,
            100_000,
            99_200,
            99_695,
            Some(100_605),
            "SNIPER.md 1.2 long, second reading, through: sn_stop_through_at_decision",
        ),
        (
            Side::Long,
            100_000,
            100_020,
            100_020,
            99_695,
            Some(99_900),
            "SNIPER.md 1.2 long, target behind",
        ),
        (
            Side::Long,
            100_000,
            100_000,
            100_000,
            99_999,
            Some(3_100_000),
            "SNIPER.md 1.2 long, tiny stop, offset 0",
        ),
        (
            Side::Long,
            100_000,
            100_000,
            100_000,
            99_990,
            Some(100_020),
            "SNIPER.md 1.2 long, exactly 2",
        ),
    ];
    for (k, (side, z, g, r, s, t, what)) in rows.iter().enumerate() {
        out.push(gate_doc(
            &format!("G{:03}", k + 1),
            what,
            6_000 + k as u64,
            *side,
            *z,
            *g,
            *r,
            i128::from(*s) * 1000,
            t.map(|x| i128::from(x) * 1000),
            10 * LAKH,
            d0909,
            None,
        ));
    }
    // 24.13's six vectors: Z = G = REF, distances as given, their charges given
    let v: [(i64, i64, i64, i64, i128, &str); 6] = [
        (
            100_000,
            10 * LAKH,
            10,
            50,
            41_677,
            "24.13: charges decide, passes nothing",
        ),
        (
            100_000,
            10 * LAKH,
            10,
            20,
            41_677,
            "24.13: reward below charges (floor, not truncation)",
        ),
        (
            100_000,
            25 * LAKH / 10,
            10,
            50,
            15_167,
            "24.13: the same record at a smaller amount",
        ),
        (
            100_020,
            10 * LAKH,
            306,
            605,
            41_671,
            "24.13: the normal vector",
        ),
        (
            100_020,
            10 * LAKH,
            306,
            0,
            41_671,
            "24.13: target behind, a real negative value",
        ),
        (
            100_000,
            10 * LAKH,
            2_500,
            12_500,
            41_677,
            "24.13: a wide stop",
        ),
    ];
    for (k, (refpx, amount, ds, dt, ch, what)) in v.iter().enumerate() {
        let stop = i128::from(refpx - ds) * 1000;
        let tgt = i128::from(refpx + dt) * 1000;
        out.push(gate_doc(
            &format!("G{:03}", k + 7),
            what,
            6_100 + k as u64,
            Side::Long,
            *refpx,
            *refpx,
            *refpx,
            stop,
            Some(tgt),
            *amount,
            d0909,
            Some(*ch),
        ));
    }
    // lanes with no price at T, and seeded cases on both sides and clocks
    out.push(gate_doc(
        "G013",
        "XG, XH, XI have no price at T: s_rr_no_target (S-STOP still runs)",
        6_200,
        Side::Long,
        100_000,
        100_000,
        100_000,
        99_900_000,
        None,
        10 * LAKH,
        d0909,
        None,
    ));
    out.push(gate_doc(
        "G014",
        "REF above the amount: sn_no_quantity",
        6_201,
        Side::Long,
        3_000_000,
        3_000_000,
        3_000_000,
        2_990_000_000,
        Some(3_100_000_000),
        25 * LAKH / 10 / 100,
        d0909,
        None,
    ));
    for k in 0..12u64 {
        let seed = 6_300 + k;
        let mut rng = SplitMix64::new(seed);
        let side = if k % 2 == 0 { Side::Long } else { Side::Short };
        let z = rng.range(5_000, 500_000);
        let g = z + rng.range(-z / 500, z / 500);
        let refpx = g + rng.range(-20, 20);
        let sgn = if side.long() { -1 } else { 1 };
        let stop =
            i128::from(z + sgn * rng.range(1, z / 50)) * 1000 + i128::from(rng.range(0, 999));
        let tgt = i128::from(z - sgn * rng.range(0, z / 20)) * 1000;
        let amount = [
            15 * LAKH,
            10 * LAKH,
            75 * LAKH / 10,
            5 * LAKH,
            25 * LAKH / 10,
        ][usize::try_from(rng.below(5)).unwrap_or(0)];
        let date = Date::from_days(
            rng.range(
                Date {
                    y: 2018,
                    m: 9,
                    d: 3,
                }
                .days(),
                Date {
                    y: 2026,
                    m: 10,
                    d: 30,
                }
                .days(),
            ),
        );
        out.push(gate_doc(
            &format!("G{:03}", 15 + k),
            &format!("seeded gate case {}, {}", k + 1, side.name()),
            seed,
            side,
            z,
            g,
            refpx,
            stop,
            Some(tgt),
            amount,
            date,
            None,
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// walks
// ---------------------------------------------------------------------------

fn put_breakdown(d: &mut Doc, head: &str, b: &Breakdown) {
    d.out(
        head,
        &[
            ("brokerage", b.brokerage.to_string()),
            ("exchange", b.exchange.to_string()),
            ("sebi", b.sebi.to_string()),
            ("stt", b.stt.to_string()),
            ("stamp", b.stamp.to_string()),
            ("gst", b.gst.to_string()),
            ("dp", b.dp.to_string()),
            ("dp_gst", b.dp_gst.to_string()),
            ("sq_fee", b.sq_fee.to_string()),
            ("sq_gst", b.sq_gst.to_string()),
            ("total", b.total().to_string()),
            ("rupees", rs(b.total())),
        ],
    );
}

fn put_priced(d: &mut Doc, p: &Priced) {
    put_breakdown(d, "charges.dated_official", &p.dated_p);
    put_breakdown(d, "charges.today_official", &p.today_p);
    put_breakdown(d, "charges.dated_worst", &p.dated_w);
    put_breakdown(d, "charges.today_worst", &p.today_w);
    d.out(
        "charges.headline",
        &[
            ("official_dearer_clock", p.headline().to_string()),
            ("worst_stress_line", p.stress().to_string()),
        ],
    );
}

/// The orders a walk made, real prices.
pub fn orders_of(w: &WalkOut, side: Side) -> Vec<Order> {
    let (entry, exit) = if side.long() {
        (OrderSide::Buy, OrderSide::Sell)
    } else {
        (OrderSide::Sell, OrderSide::Buy)
    };
    let mut out: Vec<Order> = Vec::new();
    for (k, which, sq) in [(0u8, entry, false), (1u8, exit, false), (2u8, exit, true)] {
        let fills: Vec<(i64, i64)> = w
            .fills
            .iter()
            .filter(|f| f.order == k)
            .map(|f| (f.qty, f.px))
            .collect();
        if !fills.is_empty() {
            out.push(Order {
                side: which,
                fills,
                square_off: sq,
            });
        }
    }
    out
}

/// Net = money in - money out - headline charges.
pub fn net_of(orders: &[Order], headline: i128) -> i128 {
    let sell: i128 = orders
        .iter()
        .filter(|o| o.side == OrderSide::Sell)
        .map(Order::value)
        .sum();
    let buy: i128 = orders
        .iter()
        .filter(|o| o.side == OrderSide::Buy)
        .map(Order::value)
        .sum();
    sell - buy - headline
}

fn exit_name(e: &Exit) -> String {
    match e {
        Exit::Target(t) => format!("target:{t}"),
        Exit::MoveAtBreak {
            trigger,
            then_target,
        } => format!("move_at_break:{trigger}:{then_target}"),
        Exit::Trail { arm, distance } => format!("trail:{arm}:{distance}"),
        Exit::MarketAt(t) => format!("market_at:{}", hms(*t)),
        Exit::StopOnly => "stop_only".to_owned(),
    }
}

fn walk_doc(
    id: &str,
    title: &str,
    seed: u64,
    date: Date,
    secs: &[Sec],
    plan: &Plan,
    r: Readings,
) -> Doc {
    let mut d = Doc::new(id, "walk", title, seed, r);
    d.inp(
        "plan",
        &[
            ("date", date.iso()),
            ("side", side_name(plan.side)),
            ("decided_at", hms(plan.decided_at)),
            ("amount", plan.amount.to_string()),
            ("stop", plan.stop.to_string()),
            ("exit", exit_name(&plan.exit)),
            ("lambda", plan.lambda.to_string()),
            ("share_ppm", plan.share_ppm.to_string()),
            ("step", plan.step.to_string()),
            ("session_end", hms(plan.session_end)),
            ("data_start", hms(plan.data_start)),
        ],
    );
    put_seconds(&mut d, secs);
    let w = walk::walk(secs, plan, r.within_second, r.resting_after_cut);
    d.out(
        "walk",
        &[
            ("status", w.status.name().to_owned()),
            ("start", ot(w.start)),
            ("sizing_px", o(w.sizing_px)),
            ("qty", w.qty.to_string()),
            ("filled", w.filled().to_string()),
            ("reason", w.reason.name().to_owned()),
            ("entry_end", ot(w.entry_end)),
            ("flags", w.flag_names()),
        ],
    );
    for f in &w.fills {
        d.out(
            "fill",
            &[
                ("t", hms(f.t)),
                ("qty", f.qty.to_string()),
                ("px", f.px.to_string()),
                (
                    "order",
                    ["entry", "exit", "square_off"][usize::from(f.order)].to_owned(),
                ),
            ],
        );
    }
    for (t, s) in &w.moves {
        d.out("move", &[("t", hms(*t)), ("stop", s.to_string())]);
    }
    let orders = orders_of(&w, plan.side);
    if !orders.is_empty() {
        let p = charges::price_equity(date, &orders, &r);
        put_priced(&mut d, &p);
        let net = net_of(&orders, p.headline());
        d.out(
            "net",
            &[
                ("paise", net.to_string()),
                ("rupees", rs(net)),
                ("win", bit(net > 0)),
            ],
        );
    }
    d
}

fn sec(t: u32, l: i64, h: i64, v: i64) -> Sec {
    Sec {
        t,
        o: l,
        h,
        l,
        c: h,
        v,
    }
}

fn plan(side: Side, decided_at: u32, amount: i64, stop: i64, exit: Exit) -> Plan {
    Plan {
        side,
        decided_at,
        amount,
        stop,
        exit,
        lambda: 14,
        share_ppm: 100_000,
        step: 5,
        session_end: SESSION_END,
        data_start: OPEN,
    }
}

/// Seeded noise seconds between `from` and `to` around `px` (no print
/// below `floor` nor above `ceil`).
fn noise(rng: &mut SplitMix64, from: u32, to: u32, px: i64, floor: i64, ceil: i64) -> Vec<Sec> {
    let mut out = Vec::new();
    for t in from..to {
        if !rng.chance(400) {
            continue;
        }
        let l = (px - 5 * rng.range(0, 3)).max(floor);
        let h = (px + 5 * rng.range(0, 3)).min(ceil).max(l);
        out.push(Sec {
            t,
            o: l,
            h,
            l,
            c: h,
            v: rng.range(1, 400),
        });
    }
    out
}

fn merged(mut a: Vec<Sec>, b: Vec<Sec>) -> Vec<Sec> {
    a.extend(b);
    a.sort_by_key(|s| s.t);
    a.dedup_by_key(|s| s.t);
    a
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn walk_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut next = |out: &mut Vec<Doc>, d: Doc| {
        n += 1;
        let mut d = d;
        d.id = format!("W{n:03}");
        out.push(d);
    };
    let date = Date {
        y: 2026,
        m: 10,
        d: 5,
    };
    let t0 = OPEN + 600;
    let h = Readings::HEADLINE;
    // latency, the volume cap, a no-trade second, worst prints
    let key = vec![
        sec(t0 + 5, 9_990, 10_000, 1_000),
        sec(t0 + 15, 10_000, 10_010, 300),
        sec(t0 + 16, 10_005, 10_020, 0),
        sec(t0 + 17, 10_010, 10_015, 690),
        sec(t0 + 18, 10_020, 10_030, 50),
        sec(t0 + 40, 10_100, 10_200, 1_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "14 s latency, 10% of each second's volume, a second with no trade, worst prints, a target",
            7_000,
            date,
            &key,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::Target(10_150)),
            h,
        ),
    );
    let st = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_000, 10_000),
        sec(t0 + 20, 9_800, 10_300, 10_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "a stop and a target in one second: the stop",
            7_001,
            date,
            &st,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::Target(10_200)),
            h,
        ),
    );
    next(
        &mut out,
        walk_doc(
            "",
            "a stop and a target in one second, short: the stop",
            7_002,
            date,
            &st,
            &plan(Side::Short, t0, 1_000_000, 10_200, Exit::Target(9_900)),
            h,
        ),
    );
    let thr = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 9_850, 10_000, 2_000),
        sec(t0 + 15, 9_840, 9_870, 2_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "a stop through at the first fill: entry first (A09 headline)",
            7_003,
            date,
            &thr,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::Target(10_500)),
            h,
        ),
    );
    let mut ex = h;
    ex.within_second = WithinSecond::ExitFirst;
    next(
        &mut out,
        walk_doc(
            "",
            "the same, exit first (A09 other)",
            7_003,
            date,
            &thr,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::Target(10_500)),
            ex,
        ),
    );
    let prot = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_400, 500),
        sec(t0 + 15, 10_000, 10_010, 500),
        sec(t0 + 16, 9_800, 9_850, 500),
        sec(t0 + 17, 9_780, 9_800, 2_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "protection blocks a far print; the stop's second fills the entry first",
            7_004,
            date,
            &prot,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::StopOnly),
            h,
        ),
    );
    next(
        &mut out,
        walk_doc(
            "",
            "the same, exit first: the part-filled entry exits its part",
            7_004,
            date,
            &prot,
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::StopOnly),
            ex,
        ),
    );
    let sh = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 9_990, 10_010, 10_000),
        sec(t0 + 30, 9_700, 9_760, 10_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "a short: sells at the low, buys back at the high",
            7_005,
            date,
            &sh,
            &plan(Side::Short, t0, 1_000_000, 10_100, Exit::Target(9_750)),
            h,
        ),
    );
    let xd = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_002, 10_000),
        sec(t0 + 20, 10_090, 10_110, 10_000),
        sec(t0 + 30, 9_990, 10_000, 10_000),
        sec(t0 + 36, 9_995, 10_005, 10_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "XD: at the day-high break the stop moves to the entry after 14 s",
            7_006,
            date,
            &xd,
            &plan(
                Side::Long,
                t0,
                1_000_000,
                9_900,
                Exit::MoveAtBreak {
                    trigger: 10_100,
                    then_target: 10_400,
                },
            ),
            h,
        ),
    );
    let xe = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_000, 10_000),
        sec(t0 + 20, 10_100, 10_150, 10_000),
        sec(t0 + 25, 10_150, 10_200, 10_000),
        sec(t0 + 38, 10_120, 10_160, 10_000),
        sec(t0 + 41, 10_140, 10_160, 10_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "XE: a trailing stop armed at the break, each move 14 s late",
            7_007,
            date,
            &xe,
            &plan(
                Side::Long,
                t0,
                1_000_000,
                9_900,
                Exit::Trail {
                    arm: 10_100,
                    distance: 50,
                },
            ),
            h,
        ),
    );
    let mk = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_000, 10_000),
        sec(t0 + 60, 10_020, 10_030, 10_000),
        sec(t0 + 75, 10_040, 10_050, 10_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "XG/XH: a market exit decided at a candle close, started 14 s later",
            7_008,
            date,
            &mk,
            &plan(Side::Long, t0, 1_000_000, 9_000, Exit::MarketAt(t0 + 60)),
            h,
        ),
    );
    let none = vec![
        sec(t0 - 1, 10_000, 10_000, 10),
        sec(t0 + 14, 10_000, 10_000, 5),
        sec(t0 + 60, 10_000, 10_000, 5),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "a market exit before any fill cancels the entry: no trade",
            7_009,
            date,
            &none,
            &plan(Side::Long, t0, 1_000_000, 9_000, Exit::MarketAt(t0 + 30)),
            h,
        ),
    );
    // the session-end clocks
    let cut = crate::clock::CUT;
    let sq = crate::clock::SQUARE_OFF;
    let d = cut - 24;
    let mut rng = SplitMix64::new(7_010);
    let tail = noise(&mut rng, sq, SESSION_END + 1, 9_950, 9_900, 10_000);
    let c1 = merged(
        vec![
            sec(d, 10_000, 10_000, 10),
            sec(d + 14, 10_000, 10_000, 10_000),
            sec(cut + 4, 10_050, 10_060, 1_000),
        ],
        tail.clone(),
    );
    next(
        &mut out,
        walk_doc(
            "",
            "a market exit starting at 15:08:04 is rejected; the broker squares off at 15:10",
            7_010,
            date,
            &c1,
            &plan(Side::Long, d, 1_000_000, 9_000, Exit::MarketAt(cut - 10)),
            h,
        ),
    );
    let c2 = vec![
        sec(d, 10_000, 10_000, 10),
        sec(d + 14, 9_990, 10_000, 10_000),
        sec(cut + 4, 9_980, 10_060, 1_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "no traded second from 15:10: the worst since the entry, unclosable_in_session",
            7_011,
            date,
            &c2,
            &plan(Side::Long, d, 1_000_000, 9_000, Exit::MarketAt(cut - 10)),
            h,
        ),
    );
    let c3 = vec![
        sec(cut - 20, 10_000, 10_000, 10),
        sec(cut + 4, 10_000, 10_000, 1_000),
    ];
    next(
        &mut out,
        walk_doc(
            "",
            "an entry starting at or after 15:08:00 is cancelled before it starts",
            7_012,
            date,
            &c3,
            &plan(Side::Long, cut - 10, 1_000_000, 9_000, Exit::StopOnly),
            h,
        ),
    );
    let c4 = merged(
        vec![
            sec(d, 10_000, 10_000, 10),
            sec(d + 14, 10_000, 10_000, 10_000),
            sec(cut + 60, 9_750, 9_790, 10_000),
        ],
        vec![sec(sq + 5, 9_600, 9_650, 1_000)],
    );
    next(
        &mut out,
        walk_doc(
            "",
            "a resting stop printing at 15:09 acts (A10 headline)",
            7_013,
            date,
            &c4,
            &plan(Side::Long, d, 1_000_000, 9_800, Exit::StopOnly),
            h,
        ),
    );
    let mut rej = h;
    rej.resting_after_cut = RestingAfterCut::Rejected;
    next(
        &mut out,
        walk_doc(
            "",
            "the same stop rejected after 15:08: the broker squares off (A10 other)",
            7_013,
            date,
            &c4,
            &plan(Side::Long, d, 1_000_000, 9_800, Exit::StopOnly),
            rej,
        ),
    );
    let ent = merged(
        vec![sec(cut - 30, 10_000, 10_000, 10)],
        (0..40)
            .map(|k| sec(cut - 16 + k, 10_000, 10_005, 30))
            .collect(),
    );
    let mut tail2 = noise(
        &mut SplitMix64::new(7_014),
        sq,
        sq + 300,
        10_000,
        9_950,
        10_050,
    );
    tail2.retain(|s| s.t > sq);
    let ent = merged(ent, tail2);
    next(
        &mut out,
        walk_doc(
            "",
            "an entry still filling at 15:08:00 has its remainder cancelled",
            7_014,
            date,
            &ent,
            &plan(Side::Long, cut - 30, 1_000_000, 9_000, Exit::StopOnly),
            h,
        ),
    );
    let auction = merged(
        vec![
            sec(d, 10_000, 10_000, 10),
            sec(d + 14, 10_000, 10_000, 10_000),
        ],
        vec![
            sec(sq + 100, 9_950, 9_990, 100),
            sec(SESSION_END_AUCTION + 60, 9_000, 9_100, 100),
        ],
    );
    next(
        &mut out,
        walk_doc(
            "",
            "an auction-era stock-day: the square-off reads 15:10:00 to 15:14:59 only",
            7_015,
            date,
            &auction,
            &Plan {
                session_end: SESSION_END_AUCTION,
                ..plan(Side::Long, d, 1_000_000, 9_000, Exit::StopOnly)
            },
            h,
        ),
    );
    // refusals before the start
    next(
        &mut out,
        walk_doc(
            "",
            "an order starting before the GDFL data start: no_fill_data_window",
            7_016,
            date,
            &key,
            &Plan {
                data_start: t0 + 100,
                ..plan(Side::Long, t0, 1_000_000, 9_900, Exit::StopOnly)
            },
            h,
        ),
    );
    next(
        &mut out,
        walk_doc(
            "",
            "no print before the start to size it: no_sizing_price",
            7_017,
            date,
            &[sec(t0 + 14, 10_000, 10_000, 10)],
            &plan(Side::Long, t0, 1_000_000, 9_900, Exit::StopOnly),
            h,
        ),
    );
    next(
        &mut out,
        walk_doc(
            "",
            "the sizing price above the amount: price_above_position",
            7_018,
            date,
            &[
                sec(t0, 10_000, 10_000, 10),
                sec(t0 + 14, 10_000, 10_000, 10),
            ],
            &plan(Side::Long, t0, 9_999, 9_900, Exit::StopOnly),
            h,
        ),
    );
    // seeded walks on generated seconds
    for k in 0..16u64 {
        let seed = 7_100 + k;
        let mut rng = SplitMix64::new(seed);
        let side = if k % 2 == 0 { Side::Long } else { Side::Short };
        let start = rng.range(20_000, 400_000) / 5 * 5;
        let ms = random_minutes(&mut rng, start, 5, 25, 20, 0, 6, 3);
        let mut secs = seconds_for_all(&mut rng, &ms, 5, 450, 2_000);
        // a quiet close: a few prints from 15:10 so the broker's square-off has prices
        let last = ms.last().map_or(start, |m| m.c);
        secs.extend(noise(
            &mut rng,
            crate::clock::SQUARE_OFF,
            crate::clock::SQUARE_OFF + 120,
            last,
            last - 50,
            last + 50,
        ));
        let decided = minute_start(22);
        let px = ms[2].c;
        let gap = 5 * rng.range(3, 12);
        let sgn = if side.long() { 1 } else { -1 };
        let stop = px - sgn * gap;
        let exit = match k % 5 {
            0 => Exit::Target(px + sgn * 2 * gap),
            1 => Exit::MoveAtBreak {
                trigger: px + sgn * gap,
                then_target: px + sgn * 3 * gap,
            },
            2 => Exit::Trail {
                arm: px + sgn * gap,
                distance: gap,
            },
            3 => Exit::MarketAt(minute_start(30)),
            _ => Exit::StopOnly,
        };
        let amount = [
            15 * LAKH,
            10 * LAKH,
            75 * LAKH / 10,
            5 * LAKH,
            25 * LAKH / 10,
        ][usize::try_from(k % 5).unwrap_or(0)];
        next(
            &mut out,
            walk_doc(
                "",
                &format!(
                    "seeded walk {} on generated seconds, {}",
                    k + 1,
                    side.name()
                ),
                seed,
                date,
                &secs,
                &Plan {
                    amount,
                    ..plan(side, decided, amount, stop, exit)
                },
                h,
            ),
        );
    }
    out
}

// ---------------------------------------------------------------------------
// charges
// ---------------------------------------------------------------------------

fn charge_doc(
    id: &str,
    title: &str,
    seed: u64,
    date: Date,
    orders: &[Order],
    r: Readings,
    option: bool,
) -> Doc {
    let mut d = Doc::new(id, "charges", title, seed, r);
    d.inp(
        "trade",
        &[
            ("date", date.iso()),
            (
                "instrument",
                if option { "option" } else { "equity" }.to_owned(),
            ),
        ],
    );
    for (k, ord) in orders.iter().enumerate() {
        let fills: Vec<String> = ord.fills.iter().map(|(q, p)| format!("{q}@{p}")).collect();
        d.inp(
            "order",
            &[
                ("n", (k + 1).to_string()),
                (
                    "side",
                    if ord.side == OrderSide::Buy {
                        "buy"
                    } else {
                        "sell"
                    }
                    .to_owned(),
                ),
                ("square_off", bit(ord.square_off)),
                ("fills", fills.join(",")),
            ],
        );
    }
    if option {
        let b = charges::price_option(orders, &r);
        put_breakdown(&mut d, "charges.option_official_today", &b);
    } else {
        let p = charges::price_equity(date, orders, &r);
        put_priced(&mut d, &p);
        d.out(
            "net",
            &[("paise", net_of(orders, p.headline()).to_string())],
        );
    }
    let bv: i128 = orders
        .iter()
        .filter(|o| o.side == OrderSide::Buy)
        .map(Order::value)
        .sum();
    let sv: i128 = orders
        .iter()
        .filter(|o| o.side == OrderSide::Sell)
        .map(Order::value)
        .sum();
    if orders.len() == 2 && orders.iter().all(|o| !o.square_off) {
        let (b, s) = charges::estimator(bv, sv, option);
        let side_line = |x: (i128, i128, i128, i128, i128, i128)| -> Vec<(&'static str, String)> {
            vec![
                ("brokerage", x.0.to_string()),
                ("exchange", x.1.to_string()),
                ("gst", x.2.to_string()),
                ("sebi", x.3.to_string()),
                ("stamp_or_stt", x.4.to_string()),
                ("total", x.5.to_string()),
            ]
        };
        d.out("estimator.buy", &side_line(b));
        d.out("estimator.sell", &side_line(s));
        d.out(
            "estimator.round_trip",
            &[
                ("total", (b.5 + s.5).to_string()),
                ("rupees", rs(b.5 + s.5)),
            ],
        );
    }
    d
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn charge_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    for (k, (date, orders, _)) in charges::vectors::contract_notes().into_iter().enumerate() {
        let dt = Date::parse(date).unwrap_or(Date {
            y: 2026,
            m: 1,
            d: 1,
        });
        out.push(charge_doc(
            &format!("C{:03}", k + 1),
            &format!("COSTS.md contract note CN{:02}, {date}", k + 1),
            8_000 + k as u64,
            dt,
            &orders,
            Readings::HEADLINE,
            false,
        ));
    }
    let rt = |q: i64, b: i64, s: i64| {
        vec![
            Order {
                side: OrderSide::Buy,
                fills: vec![(q, b)],
                square_off: false,
            },
            Order {
                side: OrderSide::Sell,
                fills: vec![(q, s)],
                square_off: false,
            },
        ]
    };
    out.push(charge_doc("C025", "the Adani screen: 547 x Rs 2,742.00, official reading Rs 579.41, Dhan's estimator Rs 578.85", 8_100, Date { y: 2026, m: 11, d: 7 }, &rt(547, 274_200, 274_200), Readings::HEADLINE, false));
    out.push(charge_doc(
        "C026",
        "the HDFCBANK 770 CE screen: 11,700 x Rs 26.95, Dhan Rs 794.67; official option reading",
        8_101,
        Date {
            y: 2026,
            m: 11,
            d: 7,
        },
        &rt(11_700, 2_695, 2_695),
        Readings::HEADLINE,
        true,
    ));
    let mut m20 = Readings::HEADLINE;
    m20.option_brokerage = OptionBrokerage::Min20;
    out.push(charge_doc(
        "C027",
        "a Rs 30,000 premium: flat Rs 20 brokerage (A16 headline)",
        8_102,
        Date {
            y: 2026,
            m: 11,
            d: 7,
        },
        &rt(1_000, 3_000, 3_100),
        Readings::HEADLINE,
        true,
    ));
    out.push(charge_doc(
        "C028",
        "the same premium, min(Rs 20, 0.03%) (A16 other)",
        8_102,
        Date {
            y: 2026,
            m: 11,
            d: 7,
        },
        &rt(1_000, 3_000, 3_100),
        m20,
        true,
    ));
    let mut lit = Readings::HEADLINE;
    lit.state_stamp = StateStamp::Literal;
    out.push(charge_doc(
        "C029",
        "a Rs 2.5 lakh round trip on 2019-06-03: Telangana's stamp (A15 headline)",
        8_103,
        Date {
            y: 2019,
            m: 6,
            d: 3,
        },
        &rt(250, 99_960, 99_800),
        Readings::HEADLINE,
        false,
    ));
    out.push(charge_doc(
        "C030",
        "the same trade, every listed state: Himachal Pradesh's Rs 50 (A15 other)",
        8_103,
        Date {
            y: 2019,
            m: 6,
            d: 3,
        },
        &rt(250, 99_960, 99_800),
        lit,
        false,
    ));
    out.push(charge_doc(
        "C031",
        "a Rs 15 lakh round trip on 2019-06-03 above Rs 16.67 lakh of turnover: Tamil Nadu",
        8_104,
        Date {
            y: 2019,
            m: 6,
            d: 3,
        },
        &rt(1_500, 100_000, 101_000),
        Readings::HEADLINE,
        false,
    ));
    let broker_only = vec![
        Order {
            side: OrderSide::Buy,
            fills: vec![(500, 100_000)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(500, 99_500)],
            square_off: true,
        },
    ];
    out.push(charge_doc("C038", "an entry closed whole by the broker's 15:10 square-off: its brokerage and Rs 20 + GST on top", 8_105, Date { y: 2026, m: 10, d: 30 }, &broker_only, Readings::HEADLINE, false));
    for k in 0..6u64 {
        let seed = 8_200 + k;
        let mut rng = SplitMix64::new(seed);
        let date = Date::from_days(
            rng.range(
                Date {
                    y: 2018,
                    m: 9,
                    d: 3,
                }
                .days(),
                Date {
                    y: 2026,
                    m: 10,
                    d: 30,
                }
                .days(),
            ),
        );
        let px = rng.range(2_000, 500_000);
        let qty = (15 * LAKH / px).max(1);
        let fills = |rng: &mut SplitMix64, q: i64, p: i64| -> Vec<(i64, i64)> {
            let parts = rng.range(1, 4);
            let mut left = q;
            let mut v = Vec::new();
            for j in 0..parts {
                let take = if j + 1 == parts {
                    left
                } else {
                    (q / parts).max(1).min(left)
                };
                // at least 30 shares over at most 4 parts: every part is positive
                v.push((take, p + 5 * rng.range(-2, 2)));
                left -= take;
            }
            v
        };
        let long = k % 2 == 0;
        let entry = Order {
            side: if long {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            fills: fills(&mut rng, qty, px),
            square_off: false,
        };
        let exitpx = px + rng.range(-px / 50, px / 50) / 5 * 5;
        let half = qty / 2;
        let mut orders = vec![
            entry,
            Order {
                side: if long {
                    OrderSide::Sell
                } else {
                    OrderSide::Buy
                },
                fills: fills(&mut rng, half, exitpx),
                square_off: false,
            },
        ];
        orders.push(Order {
            side: if long {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            },
            fills: vec![(qty - half, exitpx - 5)],
            square_off: k % 3 == 0,
        });
        out.push(charge_doc(
            &format!("C{:03}", 32 + k),
            &format!("seeded multi-fill trade {} on {}", k + 1, date.iso()),
            seed,
            date,
            &orders,
            Readings::HEADLINE,
            false,
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// account
// ---------------------------------------------------------------------------

fn put_ceilings(d: &mut Doc, n: u32, c: &Ceilings, start: i128) {
    d.inp(
        "account",
        &[
            ("n", n.to_string()),
            ("day_cap", c.day_cap.to_string()),
            ("month_cap", c.month_cap.to_string()),
            ("amount", c.amount.to_string()),
            ("margin", c.margin.to_string()),
            ("start_equity", start.to_string()),
        ],
    );
}

#[allow(clippy::too_many_arguments, reason = "one scenario's inputs")]
fn account_doc(
    id: &str,
    title: &str,
    seed: u64,
    sigs: &[Signal],
    n: u32,
    c: &Ceilings,
    start: i128,
    r: Readings,
) -> Doc {
    let mut d = Doc::new(id, "account", title, seed, r);
    put_ceilings(&mut d, n, c, start);
    d.push("cols signal id sym date decided_at start lev_num lev_den".to_owned());
    d.push("cols outcome id amount status filled first_fill end net charges".to_owned());
    for s in sigs {
        d.push(format!(
            "in signal {} {} {} {} {} {} {}",
            s.id,
            s.sym,
            s.date.iso(),
            hms(s.decided_at),
            hms(s.start),
            s.lev.num,
            s.lev.den
        ));
        for (k, oc) in s.outcomes.iter().enumerate() {
            d.push(format!(
                "in outcome {} {} {} {} {} {} {} {}",
                s.id,
                account::WALKED[k],
                oc.status.name().replace(' ', "_"),
                oc.filled,
                ot(oc.first_fill),
                hms(oc.end),
                oc.net,
                oc.charges
            ));
        }
    }
    let res = account::run(sigs, n, c, &r, start);
    put_account(&mut d, &res);
    d
}

fn put_account(d: &mut Doc, res: &AccountOut) {
    for a in &res.admissions {
        d.out(
            "admission",
            &[
                ("id", a.id.to_string()),
                ("status", a.status.name().replace(' ', "_")),
                ("amount", o(a.amount)),
            ],
        );
    }
    for t in &res.trades {
        d.out(
            "trade",
            &[
                ("id", t.id.to_string()),
                ("sym", t.sym.to_string()),
                ("date", t.date.iso()),
                ("amount", t.amount.to_string()),
                ("net", t.net.to_string()),
                ("charges", t.charges.to_string()),
                ("hold_s", t.hold.to_string()),
                ("win", bit(t.net > 0)),
            ],
        );
    }
    for m in &res.months {
        d.out(
            "month",
            &[
                ("month", month_label(m.month)),
                ("trades", m.trades.to_string()),
                ("wins", m.wins.to_string()),
                ("net", m.net.to_string()),
                ("charges", m.charges.to_string()),
                ("worst", m.worst.to_string()),
                (
                    "at_bar",
                    bit(m.net >= i128::from(account::BAR)), // a listed month has a trade
                ),
            ],
        );
    }
    d.out(
        "account",
        &[
            ("max_dd", res.max_dd.to_string()),
            ("max_dd_tenths", res.max_dd_tenths.to_string()),
            ("end_equity", res.end_equity.to_string()),
        ],
    );
}

fn sig(id: u32, sym: u32, date: Date, start: u32, net: i128, end: u32) -> Signal {
    let oc = Outcome {
        status: Status::Ok,
        filled: 100,
        first_fill: Some(start + 1),
        end,
        net,
        charges: 40_000,
    };
    Signal {
        id,
        sym,
        date,
        decided_at: start - 14,
        start,
        lev: Leverage::FIVE,
        outcomes: [oc; 5],
    }
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn account_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut next = |out: &mut Vec<Doc>, d: Doc| {
        n += 1;
        let mut d = d;
        d.id = format!("A{n:03}");
        out.push(d);
    };
    let cap = i128::from(account::CAPITAL);
    let h = Readings::HEADLINE;
    let day = |d: u32| Date { y: 2026, m: 10, d };
    let six: Vec<Signal> = (0..7)
        .map(|k| {
            sig(
                k,
                k,
                day(5),
                OPEN + 600 + 60 * k,
                100_000 - i128::from(k) * 30_000,
                OPEN + 620 + 60 * k,
            )
        })
        .collect();
    next(
        &mut out,
        account_doc(
            "",
            "the 6th and 7th trades of a day: day cap",
            9_000,
            &six,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    next(
        &mut out,
        account_doc(
            "",
            "the same signals under ceiling B: ten a day at Rs 10 lakh",
            9_001,
            &six,
            10,
            &CEILING_B,
            cap,
            h,
        ),
    );
    let mut month = Vec::new();
    let mut id = 0;
    for d in 1..=25u32 {
        for k in 0..5u32 {
            month.push(sig(
                id,
                id % 40,
                Date { y: 2026, m: 7, d },
                OPEN + 600 + 60 * k,
                10_000,
                OPEN + 620 + 60 * k,
            ));
            id += 1;
        }
    }
    next(
        &mut out,
        account_doc(
            "",
            "the 101st trade of a calendar month: month cap",
            9_002,
            &month,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    let lock = vec![
        sig(1, 7, day(5), OPEN + 600, 1, OPEN + 900),
        sig(2, 7, day(5), OPEN + 700, 1, OPEN + 800),
        sig(3, 8, day(5), OPEN + 900, 1, OPEN + 950),
    ];
    next(
        &mut out,
        account_doc(
            "",
            "one position per stock (while_open) and N = 1's slot, inclusive lock (A18 headline)",
            9_003,
            &lock,
            1,
            &MAIN,
            cap,
            h,
        ),
    );
    let mut ex = h;
    ex.lock_end = LockEnd::Exclusive;
    next(
        &mut out,
        account_doc(
            "",
            "the same, free in the last fill's second (A18 other)",
            9_003,
            &lock,
            1,
            &MAIN,
            cap,
            ex,
        ),
    );
    let five: Vec<Signal> = (0..5)
        .map(|k| sig(k, k, day(6), OPEN + 600 + 60 * k, 0, OPEN + 10_000))
        .collect();
    next(
        &mut out,
        account_doc(
            "",
            "the margin check on Rs 13 lakh: one step down does not fit (margin_cap)",
            9_004,
            &five,
            5,
            &MAIN,
            130_000_000,
            h,
        ),
    );
    next(
        &mut out,
        account_doc(
            "",
            "the margin check on Rs 14.01 lakh: one step down to Rs 10 lakh fits",
            9_005,
            &five,
            5,
            &MAIN,
            140_100_000,
            h,
        ),
    );
    next(
        &mut out,
        account_doc(
            "",
            "on Rs 14 lakh the month's data charge, debited at the first fill, leaves Rs 13,99,411.18: the step down no longer fits (A14 headline)",
            9_014,
            &five,
            5,
            &MAIN,
            140_000_000,
            h,
        ),
    );
    let mut ooc = h;
    ooc.data_charge_at = DataChargeAt::OutOfCurve;
    next(
        &mut out,
        account_doc(
            "",
            "the same on Rs 14 lakh with the data charge out of the curve: it fits (A14 other)",
            9_014,
            &five,
            5,
            &MAIN,
            140_000_000,
            ooc,
        ),
    );
    let three: Vec<Signal> = (0..3)
        .map(|k| Signal {
            lev: Leverage { num: 1, den: 3 },
            ..sig(k, k, day(7), OPEN + 600 + k, 5_000, OPEN + 900)
        })
        .collect();
    next(
        &mut out,
        account_doc(
            "",
            "dated leverage 3x: the main account enters Rs 7.5 lakh",
            9_006,
            &three,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    next(
        &mut out,
        account_doc(
            "",
            "dated leverage 3x: ceiling B enters Rs 5 lakh",
            9_007,
            &three,
            10,
            &CEILING_B,
            cap,
            h,
        ),
    );
    let two: Vec<Signal> = (0..2)
        .map(|k| Signal {
            lev: Leverage { num: 1, den: 2 },
            ..sig(k, k, day(8), OPEN + 600 + k, 5_000, OPEN + 900)
        })
        .collect();
    next(
        &mut out,
        account_doc(
            "",
            "leverage 2x: Rs 5 lakh, below N = 1's half step, allowed (A12 headline)",
            9_008,
            &two,
            1,
            &MAIN,
            cap,
            h,
        ),
    );
    let mut rf = h;
    rf.below_half = BelowHalf::Refused;
    next(
        &mut out,
        account_doc(
            "",
            "the same, refused below the half step (A12 other)",
            9_008,
            &two,
            1,
            &MAIN,
            cap,
            rf,
        ),
    );
    let var: Vec<Signal> = (0..4)
        .map(|k| Signal {
            lev: Leverage::from_var(2_000 + 1_500 * i64::from(k)),
            ..sig(k, k, day(9), OPEN + 600 + k, 5_000, OPEN + 900)
        })
        .collect();
    next(
        &mut out,
        account_doc(
            "",
            "C_VAR1 margins 20%, 35%, 50%, 65%: max(20, M) quantises each",
            9_009,
            &var,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    let dead = Outcome {
        status: Status::Ok,
        filled: 0,
        first_fill: None,
        end: OPEN + 650,
        net: 0,
        charges: 0,
    };
    let mut zf: Vec<Signal> = (0..5)
        .map(|k| Signal {
            outcomes: [dead; 5],
            ..sig(k, k, day(12), OPEN + 600 + k, 0, OPEN + 650)
        })
        .collect();
    zf.push(sig(9, 9, day(12), OPEN + 700, 5_000, OPEN + 760));
    next(
        &mut out,
        account_doc(
            "",
            "five zero-fill entries release the day's count (A11 headline)",
            9_010,
            &zf,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    let mut kept = h;
    kept.zero_fill_count = ZeroFillCount::Kept;
    next(
        &mut out,
        account_doc(
            "",
            "the same, counted for good (A11 other)",
            9_010,
            &zf,
            5,
            &MAIN,
            cap,
            kept,
        ),
    );
    let curve = vec![
        sig(
            1,
            1,
            Date {
                y: 2026,
                m: 1,
                d: 5,
            },
            OPEN + 600,
            300_000,
            OPEN + 700,
        ),
        sig(
            2,
            2,
            Date {
                y: 2026,
                m: 1,
                d: 6,
            },
            OPEN + 600,
            -500_000,
            OPEN + 700,
        ),
        sig(
            3,
            3,
            Date {
                y: 2026,
                m: 2,
                d: 2,
            },
            OPEN + 600,
            100_000,
            OPEN + 700,
        ),
    ];
    next(
        &mut out,
        account_doc(
            "",
            "months and drawdown, the data charge in the curve, floor tenths (A13, A14 headline)",
            9_011,
            &curve,
            5,
            &MAIN,
            cap,
            h,
        ),
    );
    let mut ceil = h;
    ceil.dd_round = DdRound::Ceil;
    next(
        &mut out,
        account_doc(
            "",
            "the same, ceil tenths (A13 other)",
            9_011,
            &curve,
            5,
            &MAIN,
            cap,
            ceil,
        ),
    );
    let mut oc = h;
    oc.data_charge_at = DataChargeAt::OutOfCurve;
    next(
        &mut out,
        account_doc(
            "",
            "the same, the data charge out of the curve (A14 other)",
            9_011,
            &curve,
            5,
            &MAIN,
            cap,
            oc,
        ),
    );
    let refused = Signal {
        outcomes: [Outcome {
            status: Status::PriceAbovePosition,
            filled: 0,
            first_fill: None,
            end: OPEN + 700,
            net: 0,
            charges: 0,
        }; 5],
        ..sig(50, 50, day(8), OPEN + 600, 0, OPEN + 700)
    };
    next(
        &mut out,
        account_doc(
            "",
            "a walk refusal (price_above_position) admits nothing",
            9_012,
            &[refused, sig(51, 51, day(8), OPEN + 650, 10, OPEN + 900)],
            1,
            &MAIN,
            cap,
            h,
        ),
    );
    let tiny = [Signal {
        lev: Leverage { num: 1, den: 1 },
        ..sig(9, 9, day(6), OPEN + 600, 0, OPEN + 700)
    }];
    next(
        &mut out,
        account_doc(
            "",
            "N = 20 at 1x on Rs 1,000 of equity: size_refused",
            9_013,
            &tiny,
            20,
            &MAIN,
            100_000,
            h,
        ),
    );
    for k in 0..6u64 {
        let seed = 9_100 + k;
        let mut rng = SplitMix64::new(seed);
        let mut sigs = Vec::new();
        for id in 0..u32::try_from(rng.range(20, 60)).unwrap_or(20) {
            let date = Date::from_days(
                Date {
                    y: 2026,
                    m: 3,
                    d: 2,
                }
                .days()
                    + rng.range(0, 45),
            );
            if date.weekday() >= 5 {
                continue;
            }
            let start = OPEN + u32::try_from(rng.range(60, 20_000)).unwrap_or(60);
            let end = start + u32::try_from(rng.range(30, 3_000)).unwrap_or(30);
            let trade_net = i128::from(rng.range(-30_000, 40_000)) * 10;
            let mut s = sig(
                id,
                u32::try_from(rng.range(0, 12)).unwrap_or(0),
                date,
                start,
                trade_net,
                end,
            );
            s.lev = Leverage {
                num: 1,
                den: rng.range(2, 5),
            };
            for (j, oc) in s.outcomes.iter_mut().enumerate() {
                oc.net = trade_net * i128::from(account::WALKED[j]) / i128::from(15 * LAKH);
                oc.charges = 20_000 + i128::from(account::WALKED[j] / 5_000);
                if rng.chance(50) {
                    oc.filled = 0;
                    oc.first_fill = None;
                }
            }
            sigs.push(s);
        }
        let nn = [1u32, 3, 5, 10, 20, 2][usize::try_from(k).unwrap_or(0)];
        next(
            &mut out,
            account_doc(
                "",
                &format!("seeded account {} at N = {nn}", k + 1),
                seed,
                &sigs,
                nn,
                &MAIN,
                cap,
                h,
            ),
        );
    }
    out
}

// ---------------------------------------------------------------------------
// ranking, walk-forward, the gate, luck, verdict, lots
// ---------------------------------------------------------------------------

fn put_key(d: &mut Doc, head: &str, extra: &[(&str, String)], k: &Key) {
    let mut v: Vec<(&str, String)> = extra.to_vec();
    v.extend([
        ("id", k.id.to_string()),
        ("passer", bit(k.passer)),
        ("dd", k.dd.to_string()),
        ("dd_tenths", k.dd_tenths.to_string()),
        ("net", k.net.to_string()),
        ("wins", k.wins.to_string()),
        ("trades", k.trades.to_string()),
        ("hold", k.hold.to_string()),
        ("shortfall", k.shortfall.to_string()),
        ("worst", k.worst.to_string()),
        ("at_bar", k.at_bar.to_string()),
        ("months", k.months.to_string()),
    ]);
    d.out(head, &v);
}

fn put_series(d: &mut Doc, cands: &[Series]) {
    d.push("cols trade series month net hold".to_owned());
    for s in cands {
        for t in &s.trades {
            d.push(format!(
                "in trade {} {} {} {}",
                s.id,
                month_label(t.month),
                t.net,
                t.hold
            ));
        }
    }
}

fn wf_doc(id: &str, title: &str, seed: u64, cands: &[Series], r: Readings) -> Doc {
    let mut d = Doc::new(id, "walkforward", title, seed, r);
    put_series(&mut d, cands);
    for w in wf::windows() {
        for s in cands {
            let k = wf::key(s, w.learn_first, w.learn_last, &r);
            put_key(&mut d, "learn.key", &[("window", w.year.to_string())], &k);
        }
        d.out(
            "pick",
            &[
                ("window", w.year.to_string()),
                ("id", o(wf::pick(cands, &w, &r))),
            ],
        );
    }
    let (_, months, joined) = wf::chain(cands, &r);
    for m in &months {
        d.out(
            "chain.month",
            &[
                ("month", month_label(m.month)),
                ("trades", m.trades.to_string()),
                ("net", m.net.to_string()),
                (
                    "at_bar",
                    bit(m.trades > 0 && m.net >= i128::from(account::BAR)),
                ),
            ],
        );
    }
    let w = wf::windows();
    let ck = wf::key(&joined, w[0].unseen_first, w[6].unseen_last, &r);
    put_key(&mut d, "chain.key", &[], &ck);
    d
}

fn series_seeded(rng: &mut SplitMix64, id: u32, mean: i64, spread: i64, per_month: i64) -> Series {
    let w = wf::windows();
    let mut trades = Vec::new();
    for m in w[0].learn_first..=w[6].unseen_last {
        let k = rng.range(0, per_month);
        for _ in 0..k {
            trades.push(SeriesTrade {
                month: m,
                net: i128::from(mean + rng.range(-spread, spread)) * 100,
                hold: u32::try_from(rng.range(60, 6_000)).unwrap_or(60),
            });
        }
    }
    Series { id, trades }
}

fn verdict_doc(id: &str, title: &str, v: &VerdictIn, r: Readings) -> Doc {
    let mut d = Doc::new(id, "verdict", title, 0, r);
    d.inp(
        "verdict",
        &[
            ("months", o(v.months.as_ref().map(Vec::len))),
            ("luck", bit(v.luck)),
            ("interim", bit(v.interim)),
        ],
    );
    if let Some(ms) = &v.months {
        let below = ms
            .iter()
            .filter(|m| m.trades == 0 || m.net < i128::from(account::BAR))
            .count();
        d.inp(
            "months_summary",
            &[
                ("count", ms.len().to_string()),
                ("below_bar_or_empty", below.to_string()),
                (
                    "first_below",
                    o(ms.iter()
                        .find(|m| m.trades == 0 || m.net < i128::from(account::BAR))
                        .map(|m| month_label(m.month))),
                ),
            ],
        );
    }
    put_key(&mut d, "in.headline_key", &[], &v.key);
    for (k, t) in v.twins.iter().enumerate() {
        match t {
            Some(t) => put_key(
                &mut d,
                "in.twin_key",
                &[("seed", (k + 1).to_string()), ("whole", bit(t.whole))],
                &t.key,
            ),
            None => d.inp(
                "twin",
                &[("seed", (k + 1).to_string()), ("ran", "0".to_owned())],
            ),
        }
    }
    d.out(
        "line2",
        &[("verdict", wf::verdict(v, &r).line().replace(' ', "_"))],
    );
    d
}

#[allow(clippy::too_many_lines, reason = "one list of scenarios")]
fn wf_docs() -> Vec<Doc> {
    let mut out = Vec::new();
    let h = Readings::HEADLINE;
    for k in 0..6u64 {
        let seed = 10_000 + k;
        let mut rng = SplitMix64::new(seed);
        let cands: Vec<Series> = (0..4)
            .map(|id| series_seeded(&mut rng, id, 60_000 + 40_000 * i64::from(id), 300_000, 3))
            .collect();
        out.push(wf_doc(
            &format!("K{:03}", k + 1),
            &format!(
                "THE RANKING's picks over seven windows, seeded candidates {}",
                k + 1
            ),
            seed,
            &cands,
            h,
        ));
    }
    // a passer against misses, and the drawdown key
    let w = wf::windows();
    let every = |id: u32, net: i128| Series {
        id,
        trades: (w[0].learn_first..=w[6].unseen_last)
            .map(|m| SeriesTrade {
                month: m,
                net,
                hold: 600,
            })
            .collect(),
    };
    let pass = every(1, 3_200_000_000 / 100);
    let big_miss = Series {
        id: 2,
        trades: vec![SeriesTrade {
            month: w[0].learn_first,
            net: 100_000_000_000,
            hold: 60,
        }],
    };
    out.push(wf_doc(
        "K007",
        "a passer in every month beats a miss with far more net",
        10_100,
        &[pass.clone(), big_miss],
        h,
    ));
    let mut dd = every(3, 3_200_000_000 / 100);
    dd.trades.insert(
        5,
        SeriesTrade {
            month: w[0].learn_first + 2,
            net: -200_000_000,
            hold: 60,
        },
    );
    dd.trades.insert(
        6,
        SeriesTrade {
            month: w[0].learn_first + 2,
            net: 210_000_000,
            hold: 60,
        },
    );
    out.push(wf_doc(
        "K008",
        "two passers: the lower max drawdown wins before more net",
        10_101,
        &[pass.clone(), dd.clone()],
        h,
    ));
    let mut hc = h;
    hc.dd_round = DdRound::Ceil;
    out.push(wf_doc(
        "K009",
        "the same two passers, drawdown tenths ceiled (A13 other)",
        10_101,
        &[pass.clone(), dd],
        hc,
    ));
    let a = Series {
        id: 4,
        trades: vec![
            SeriesTrade {
                month: w[0].learn_first,
                net: 100_000,
                hold: 10,
            },
            SeriesTrade {
                month: w[0].learn_first + 1,
                net: 100_000,
                hold: 10,
            },
        ],
    };
    let b = Series {
        id: 5,
        trades: vec![SeriesTrade {
            month: w[0].learn_first,
            net: 200_000,
            hold: 10,
        }],
    };
    out.push(wf_doc(
        "K010",
        "two misses: the smaller total shortfall wins (more net in all)",
        10_102,
        &[a, b],
        h,
    ));
    let mut oc = h;
    oc.data_charge_at = DataChargeAt::OutOfCurve;
    out.push(wf_doc(
        "K011",
        "the passer with the data charge out of the curve (A14 other)",
        10_103,
        &[pass, every(6, 3_000_000_000 / 100)],
        oc,
    ));
    // the gate, luck, lots
    let mut g = Doc::new(
        "K012",
        "gate",
        "the high-probability gate's boundaries (R4) on the three seeds",
        0,
        h,
    );
    for (wins, losses) in [
        (3, 0),
        (2, 0),
        (6, 0),
        (5, 0),
        (10, 0),
        (9, 0),
        (677_853, 675_147),
        (677_852, 675_148),
        (0, 0),
        (5, 5),
        (4, 5),
        (u32::MAX, u32::MAX - 1),
        (20, 10),
        (40, 25),
        (100, 80),
        (1_000, 950),
    ] {
        for z in wf::ZSQ {
            d_gate(&mut g, wins, losses, z);
        }
    }
    out.push(g);
    let mut l = Doc::new(
        "K013",
        "luck",
        "the luck rule clears with at most 49 of 1,000 at or above O",
        0,
        h,
    );
    for (k, at) in [0usize, 1, 48, 49, 50, 51, 999, 1000].iter().enumerate() {
        let best: Vec<u32> = (0..1000).map(|b| if b < *at { 90 } else { 10 }).collect();
        l.inp(
            "luck",
            &[
                ("case", (k + 1).to_string()),
                ("o", "70".to_owned()),
                ("best_at_or_above", at.to_string()),
                ("of", "1000".to_owned()),
            ],
        );
        l.out(
            "luck",
            &[
                ("case", (k + 1).to_string()),
                ("clears", bit(wf::luck_clears(70, &best))),
            ],
        );
    }
    out.push(l);
    let mut lots = Doc::new(
        "K014",
        "lots",
        "whole option lots at or under the premium, rounding down",
        0,
        h,
    );
    for (amount, lot, prem) in [
        (20_000_000, 650, 3_000),
        (19_499_999, 650, 3_000),
        (1_950_000, 650, 3_000),
        (1_949_999, 650, 3_000),
        (3_000_000, 650, 3_000),
        (5_000_000, 75, 12_345),
        (10_000_000, 1_800, 2_695),
        (20_000_000, 550, 40_000),
    ] {
        lots.inp(
            "lots",
            &[
                ("amount", amount.to_string()),
                ("lot", lot.to_string()),
                ("premium", prem.to_string()),
            ],
        );
        let v = wf::option_lots(amount, lot, prem);
        lots.out(
            "lots",
            &[
                ("amount", amount.to_string()),
                (
                    "lots",
                    v.map_or_else(|e| e.name().to_owned(), |x| x.to_string()),
                ),
            ],
        );
    }
    out.push(lots);
    // the verdict, rule by rule
    let good = Key {
        passer: true,
        dd_tenths: 0,
        dd: 0,
        net: 1,
        wins: 1,
        trades: 1,
        hold: 1,
        shortfall: 0,
        worst: 0,
        at_bar: 82,
        months: 82,
        id: 1,
    };
    let months: Vec<ChainMonth> = (0..82)
        .map(|m| ChainMonth {
            month: 2020 * 12 + m,
            trades: 1,
            net: i128::from(account::BAR),
        })
        .collect();
    let below = Twin {
        key: Key {
            passer: false,
            id: 9,
            ..good
        },
        whole: true,
    };
    let above = Twin {
        key: Key { id: 0, ..good },
        whole: true,
    };
    let base = VerdictIn {
        months: Some(months.clone()),
        key: good,
        luck: true,
        twins: [Some(below); 3],
        interim: false,
    };
    let mut vs: Vec<(String, VerdictIn, Readings)> = Vec::new();
    vs.push((
        "rule 1: no headline chain formed".to_owned(),
        VerdictIn {
            months: None,
            ..base.clone()
        },
        h,
    ));
    let mut low = months.clone();
    low[40].net -= 1;
    vs.push((
        "rule 2: one paisa below the bar in one month".to_owned(),
        VerdictIn {
            months: Some(low),
            ..base.clone()
        },
        h,
    ));
    let mut empty = months;
    empty[3].trades = 0;
    vs.push((
        "rule 2: a no-trade month".to_owned(),
        VerdictIn {
            months: Some(empty),
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 3: the sign test does not clear".to_owned(),
        VerdictIn {
            luck: false,
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 4: twin seed 1 at or above the headline".to_owned(),
        VerdictIn {
            twins: [Some(above), None, None],
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 4: a partial twin 1 at or above still says NO".to_owned(),
        VerdictIn {
            twins: [
                Some(Twin {
                    whole: false,
                    ..above
                }),
                None,
                None,
            ],
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 4: a partial twin 1 below clears nothing".to_owned(),
        VerdictIn {
            twins: [
                Some(Twin {
                    whole: false,
                    ..below
                }),
                None,
                None,
            ],
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 5: fewer than three twin seeds have run".to_owned(),
        VerdictIn {
            twins: [Some(below), None, None],
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 5 before rule 6: twin 2 above, twin 3 missing reads PENDING".to_owned(),
        VerdictIn {
            twins: [Some(below), Some(above), None],
            ..base.clone()
        },
        h,
    ));
    vs.push((
        "rule 6: twin seed 2 at or above".to_owned(),
        VerdictIn {
            twins: [Some(below), Some(above), Some(below)],
            ..base.clone()
        },
        h,
    ));
    vs.push(("rule 7: YES".to_owned(), base.clone(), h));
    vs.push((
        "an interim never prints YES (A19 headline)".to_owned(),
        VerdictIn {
            interim: true,
            ..base.clone()
        },
        h,
    ));
    let mut nc = h;
    nc.interim = InterimLine::NoCanPrint;
    vs.push((
        "an interim's NO prints under A19's other reading".to_owned(),
        VerdictIn {
            interim: true,
            luck: false,
            ..base.clone()
        },
        nc,
    ));
    vs.push((
        "an interim's NO stays PENDING under A19's headline".to_owned(),
        VerdictIn {
            interim: true,
            luck: false,
            ..base
        },
        h,
    ));
    for (k, (title, v, r)) in vs.into_iter().enumerate() {
        out.push(verdict_doc(&format!("V{:03}", k + 1), &title, &v, r));
    }
    out
}

fn d_gate(d: &mut Doc, wins: u32, losses: u32, z: u32) {
    d.out(
        "gate",
        &[
            ("wins", wins.to_string()),
            ("losses", losses.to_string()),
            ("zsq", z.to_string()),
            ("clears", bit(wf::clears(wins, losses, z))),
        ],
    );
}

// ---------------------------------------------------------------------------
// end to end
// ---------------------------------------------------------------------------

/// One day end to end: E1C, its levels mapped, each lane walked, priced.
fn e2e_doc(id: &str, title: &str, seed: u64, side: Side, k: u32) -> Doc {
    e2e_doc_gap(id, title, seed, side, k, false)
}

/// As [`e2e_doc`]; `no_g_at_break` removes the GDFL trade of the pattern's
/// break minute, so R1 has no price there.
#[allow(
    clippy::too_many_lines,
    reason = "one day built, signalled, walked and priced in order"
)]
fn e2e_doc_gap(id: &str, title: &str, seed: u64, side: Side, k: u32, no_g_at_break: bool) -> Doc {
    let r = Readings::HEADLINE;
    let date = Date {
        y: 2026,
        m: 10,
        d: 15,
    };
    let mut day = build_day(seed, 1, side, 12, &flag_script(k as usize), 40, true);
    if no_g_at_break {
        day.minutes[15 + k as usize].g = None;
    }
    let last = day.minutes.last().map_or(100_000, |m| m.c);
    let mut tail_rng = SplitMix64::new(seed + 7);
    day.secs.extend(noise(
        &mut tail_rng,
        crate::clock::SQUARE_OFF,
        crate::clock::SQUARE_OFF + 120,
        last,
        last - 50,
        last + 50,
    ));
    let mut d = Doc::new(id, "e2e", title, seed, r);
    let p = e1_params(side, k);
    let candles = fold(&day.minutes, 1);
    let top = vec![true; candles.len()];
    d.inp(
        "e1",
        &[
            ("side", side_name(side)),
            ("k", k.to_string()),
            ("date", date.iso()),
            ("amount", (15 * LAKH).to_string()),
            ("step", "5".to_owned()),
        ],
    );
    put_top(&mut d, &top);
    put_minutes(&mut d, &day.minutes);
    put_seconds(&mut d, &day.secs);
    // a break always carries its levels (only a void has none)
    for (e, lv) in e1::e1c(&candles, &top, &p, &r)
        .iter()
        .filter(|e| e.status == Status::Ok)
        .filter_map(|e| e.levels.map(|lv| (e, lv)))
    {
        let Some((z, g)) = r1_at(&day.minutes, e.t) else {
            d.out(
                "e2e.signal",
                &[
                    ("candle", e.candle.to_string()),
                    ("status", "no_level_ratio".to_owned()),
                ],
            );
            continue;
        };
        let st = map_level(lv.stop_milli, z, g, 5, stop_toward(side));
        let tg = |x: i128| map_level(x, z, g, 5, target_toward(side));
        d.out(
            "e2e.signal",
            &[
                ("candle", e.candle.to_string()),
                ("t", hms(e.t)),
                ("stop_g", st.to_string()),
                ("t1_g", tg(lv.t1_milli).to_string()),
                ("t2_g", o(lv.t2_milli.map(tg))),
                ("t3_g", o(lv.t3_milli.map(tg))),
            ],
        );
        // a rung lane walks only when its rung has a usable ladder (no_ladder_target otherwise)
        let t1 = tg(lv.t1_milli);
        let rung = |t: Option<i128>| t.map(tg);
        let mut lanes: Vec<(&str, Exit)> = vec![("XA", Exit::Target(t1))];
        lanes.extend(rung(lv.t2_milli).map(|t| ("XB", Exit::Target(t))));
        lanes.extend(rung(lv.t3_milli).map(|t| ("XC", Exit::Target(t))));
        lanes.extend(rung(lv.t2_milli).map(|t| {
            (
                "XD2",
                Exit::MoveAtBreak {
                    trigger: t1,
                    then_target: t,
                },
            )
        }));
        lanes.extend(rung(lv.t3_milli).map(|t| {
            (
                "XD3",
                Exit::MoveAtBreak {
                    trigger: t1,
                    then_target: t,
                },
            )
        }));
        lanes.push(("XG", Exit::MarketAt(e.t + 10 * 60)));
        for (name, exit) in lanes {
            let pl = Plan {
                side,
                decided_at: e.t,
                amount: 15 * LAKH,
                stop: st,
                exit,
                lambda: 14,
                share_ppm: 100_000,
                step: 5,
                session_end: SESSION_END,
                data_start: OPEN,
            };
            let w = walk::walk(&day.secs, &pl, r.within_second, r.resting_after_cut);
            let orders = orders_of(&w, side);
            let pr = charges::price_equity(date, &orders, &r);
            let net = net_of(&orders, pr.headline()); // zero on an empty order list
            d.out(
                "e2e.lane",
                &[
                    ("lane", name.to_owned()),
                    ("status", w.status.name().to_owned()),
                    ("start", ot(w.start)),
                    ("qty", w.qty.to_string()),
                    ("filled", w.filled().to_string()),
                    ("first_fill", ot(w.first_fill())),
                    ("last_fill", ot(w.last_fill())),
                    ("reason", w.reason.name().to_owned()),
                    ("flags", w.flag_names()),
                    ("charges", pr.headline().to_string()),
                    ("net", net.to_string()),
                ],
            );
        }
    }
    d
}

fn e2e_docs() -> Vec<Doc> {
    vec![
        e2e_doc(
            "X001",
            "a flag day end to end, long, K = 2: every E1C signal walked alone on lanes XA, XB, XC, XD2, XD3, XG (no account)",
            11_000,
            Side::Long,
            2,
        ),
        e2e_doc(
            "X002",
            "one flag day end to end, short, K = 2",
            11_001,
            Side::Short,
            2,
        ),
        e2e_doc(
            "X003",
            "one flag day end to end, long, K = 3",
            11_002,
            Side::Long,
            3,
        ),
        e2e_doc(
            "X004",
            "one flag day end to end, short, K = 1",
            11_003,
            Side::Short,
            1,
        ),
        e2e_doc_gap(
            "X005",
            "the flag's break minute has no GDFL trade: no_level_ratio, no lane walked for it",
            11_004,
            Side::Long,
            2,
            true,
        ),
    ]
}

/// Every scenario, in a fixed order.
pub fn all() -> Vec<Doc> {
    let mut v = Vec::new();
    v.extend(rank_docs());
    v.extend(ladder_docs());
    v.extend(e1_docs());
    v.extend(e2_docs());
    v.extend(e3_docs());
    v.extend(gate_docs());
    v.extend(walk_docs());
    v.extend(charge_docs());
    v.extend(account_docs());
    v.extend(wf_docs());
    v.extend(e2e_docs());
    v
}

// ---------------------------------------------------------------------------
// SHA-256 and the manifest
// ---------------------------------------------------------------------------

/// SHA-256 of `data` (FIPS 180-4), for the manifest.
#[allow(
    clippy::too_many_lines,
    reason = "FIPS 180-4's padding and compression in one place"
)]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut msg = data.to_vec();
    let bits = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lower-case hex of a digest.
pub fn hex(d: &[u8]) -> String {
    d.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// `MANIFEST.md` for a set of `(file name, text)`.
pub fn manifest(files: &[(String, String)]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# MANIFEST: FIB-PREVIEW-REF golden scenarios");
    let _ = writeln!(s);
    let _ = writeln!(s, "{}", crate::PREVIEW);
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "One line per file: its SHA-256, two spaces, its name (the `sha256sum` form). {} files.",
        files.len()
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "```text");
    for (name, text) in files {
        let _ = writeln!(s, "{}  {name}", hex(&sha256(text.as_bytes())));
    }
    let _ = writeln!(s, "```");
    s
}

/// Every golden file `(name, text)`: the scenarios, the format and the
/// index; the manifest is made from these.
pub fn files() -> Vec<(String, String)> {
    let docs = all();
    let mut v: Vec<(String, String)> = docs.iter().map(|d| (d.file_name(), d.render())).collect();
    v.push((
        "FORMAT.md".to_owned(),
        crate::golden_format::FORMAT_MD.to_owned(),
    ));
    v.push(("INDEX.md".to_owned(), index(&docs)));
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn index(docs: &[Doc]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# INDEX: FIB-PREVIEW-REF golden scenarios");
    let _ = writeln!(s);
    let _ = writeln!(s, "{}", crate::PREVIEW);
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "{} scenarios. Readings other than the headline are named (report section 4).",
        docs.len()
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "| Id | Kind | Readings | Outputs | Title |");
    let _ = writeln!(s, "|---|---|---|---:|---|");
    for d in docs {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {} |",
            d.id,
            d.kind,
            d.readings.label(),
            d.outputs(),
            d.title
        );
    }
    s
}

/// Writes every golden file and the manifest into `dir`.
///
/// # Errors
/// Any I/O error from creating the directory or writing a file.
pub fn write_all(dir: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dir)?;
    let files = files();
    for (name, text) in &files {
        std::fs::write(dir.join(name), text)?;
    }
    std::fs::write(dir.join("MANIFEST.md"), manifest(&files))?;
    Ok(files.len() + 1)
}

/// The golden writer, whole, for `src/bin/fib_ref_golden.rs`: the first
/// argument after the program's name is the directory. Returns the exit
/// code: 0 written, 1 refused by the file system (named on `err`), 2 no
/// directory named (the usage on `err`).
pub fn writer_main(
    args: &[std::ffi::OsString],
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> u8 {
    let Some(dir) = args.get(1).map(std::path::PathBuf::from) else {
        let _ = writeln!(err, "usage: fib_ref_golden <directory>");
        return 2;
    };
    match write_all(&dir) {
        Ok(n) => {
            let _ = writeln!(out, "wrote {n} files into {}", dir.display());
            0
        }
        Err(e) => {
            let _ = writeln!(err, "writing {}: {e}", dir.display());
            1
        }
    }
}

#[cfg(test)]
#[path = "golden_tests.rs"]
mod tests;
