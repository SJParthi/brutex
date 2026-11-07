//! FIB-PREVIEW-REF benches: p50 and p99 per step at sizes 10^3 .. 10^7.
//! PREVIEW: synthetic inputs; timings are of this reference build only.
//!
//! Each row times one hot step after `n` items of prior state (or within
//! inputs of size `n`), as 2,000 samples of a batch, and prints p50 and p99
//! in nanoseconds a step. A step is called flat when p99 at 10^7 is at most
//! 1.5 x p99 at 10^3 and p99 is at most 3 x p50 at every size (PREVIEW-SPEC
//! 17.7's bar); the verdict prints as measured, never adjusted. Steps that
//! are not O(1) by construction are named on their own rows.

#![allow(
    clippy::cast_precision_loss,
    reason = "timings and sizes print as floats; no decision reads them"
)]

use std::hint::black_box;
use std::time::Instant;

use fib_ref::account::{Account, Leverage, MAIN, Outcome, Signal, WALKED};
use fib_ref::bars::{Candle, Sec};
use fib_ref::charges::{Order, OrderSide, price_equity};
use fib_ref::clock::{Date, OPEN, SESSION_END};
use fib_ref::e1::{E1Params, Holding, LadderMode, SecondFeed, e1c};
use fib_ref::e2::{E2Params, Reading, TouchClause, e2};
use fib_ref::gates;
use fib_ref::ladder::{ExtremeTies, Ladder, TaintScope};
use fib_ref::model::{Readings, RestingAfterCut, Side, Status, WithinSecond};
use fib_ref::rank::{Mover, frame};
use fib_ref::synth::{SplitMix64, random_minutes};
use fib_ref::walk::{Exit, Plan, walk};
use fib_ref::wf::{Key, Series, SeriesTrade, cmp_ranking, key};

const SIZES: [u64; 5] = [1_000, 10_000, 100_000, 1_000_000, 10_000_000];
const SAMPLES: usize = 2_000;

fn pct(v: &mut [f64], per_mille: usize) -> f64 {
    v.sort_by(f64::total_cmp);
    // nearest rank, half up, in integers
    let ix = ((v.len() - 1) * per_mille + 500) / 1000;
    v[ix.min(v.len() - 1)]
}

/// Times `SAMPLES` batches of `batch` calls of `f`, returning (p50, p99)
/// nanoseconds per call.
fn time<F: FnMut()>(batch: u32, f: F) -> (f64, f64) {
    time_n(SAMPLES, batch, f)
}

/// As [`time`] with a chosen number of samples.
fn time_n<F: FnMut()>(samples: usize, batch: u32, mut f: F) -> (f64, f64) {
    let mut v = Vec::with_capacity(samples);
    for _ in 0..samples {
        let t = Instant::now();
        for _ in 0..batch {
            f();
        }
        v.push(t.elapsed().as_nanos() as f64 / f64::from(batch));
    }
    let p50 = pct(&mut v, 500);
    let p99 = pct(&mut v, 990);
    (p50, p99)
}

struct Row {
    id: &'static str,
    what: &'static str,
    o1: &'static str,
    points: Vec<(u64, f64, f64)>,
}

impl Row {
    fn print(&self) {
        for (n, p50, p99) in &self.points {
            println!(
                "{} {:<34} n={:>10} p50={:>10.1} ns p99={:>10.1} ns",
                self.id, self.what, n, p50, p99
            );
        }
        let first = self.points.first().map_or(0.0, |p| p.2);
        let last = self.points.last().map_or(0.0, |p| p.2);
        let tail_ok = last <= 1.5 * first;
        let spread_ok = self
            .points
            .iter()
            .all(|(_, p50, p99)| *p99 <= 3.0 * p50.max(1.0));
        println!(
            "{} {:<34} verdict: {} (p99 10^7 / 10^3 = {:.2}; p99 <= 3 x p50 at every size: {}); {}",
            self.id,
            self.what,
            if tail_ok && spread_ok {
                "FLAT"
            } else {
                "NOT FLAT"
            },
            if first > 0.0 { last / first } else { 0.0 },
            if spread_ok { "yes" } else { "no" },
            self.o1
        );
    }
}

fn candle(i: u32, o: i64, h: i64, l: i64, c: i64) -> Candle {
    Candle {
        i,
        first_minute: i,
        last_minute: i,
        close_t: OPEN + 60 * (i % 375 + 1),
        o,
        h,
        l,
        c,
        v: 1,
        incomplete: false,
    }
}

fn bench_ladder() -> Row {
    let mut points = Vec::new();
    for &n in &SIZES {
        let mut rng = SplitMix64::new(n);
        let mut l = Ladder::EMPTY;
        let mut px = 100_000i64;
        for i in 0..n {
            px += 5 * rng.range(-3, 3);
            let c = candle(u32::try_from(i).unwrap_or(0), px, px + 10, px - 10, px + 5);
            l.fold(&c, ExtremeTies::Strict, TaintScope::AnyFolded);
        }
        let mut i = u32::try_from(n).unwrap_or(0);
        let (p50, p99) = time(64, || {
            px += 5 * rng.range(-3, 3);
            let c = candle(i, px, px + 10, px - 10, px + 5);
            l.fold(black_box(&c), ExtremeTies::Strict, TaintScope::AnyFolded);
            i = i.wrapping_add(1);
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B01",
        what: "ladder fold, one candle",
        o1: "O(1): two compares and a state",
        points,
    }
}

fn long_candles(seed: u64, n: u64) -> Vec<Candle> {
    let mut rng = SplitMix64::new(seed);
    let mut px = 100_000i64;
    (0..n)
        .map(|i| {
            let o = px;
            px += 5 * rng.range(-4, 4);
            let c = px;
            candle(
                u32::try_from(i).unwrap_or(0),
                o,
                o.max(c) + 5 * rng.range(0, 2),
                o.min(c) - 5 * rng.range(0, 2),
                c,
            )
        })
        .collect()
}

/// Samples for a whole-sequence timing of `n` items: fewer for longer runs.
fn samples_for(n: u64) -> usize {
    usize::try_from((20_000_000 / n).clamp(5, 2_000)).unwrap_or(5)
}

fn bench_e1() -> Row {
    let p = E1Params {
        side: Side::Long,
        k: 16,
        rung: 382,
        holding: Holding::AllLows,
        mode: LadderMode::Running,
        max_height_ppm: None,
        colour: true,
        stop_offset: 5,
        t1_offset: 5,
    };
    let r = Readings::HEADLINE;
    let mut points = Vec::new();
    for &n in &SIZES {
        let candles = long_candles(n, n);
        let top = vec![true; candles.len()];
        let (p50, p99) = time_n(samples_for(n), 1, || {
            black_box(e1c(black_box(&candles), &top, &p, &r));
        });
        let per = n as f64;
        points.push((n, p50 / per, p99 / per));
    }
    Row {
        id: "FIBREF-B02",
        what: "E1C box machine, per candle (K=16)",
        o1: "per candle O(K), K at most 16 (the box is re-read); a sequence of n candles in one call",
        points,
    }
}

fn bench_e2() -> Row {
    let p = E2Params {
        side: Side::Long,
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
    };
    let r = Readings::HEADLINE;
    let mut rng = SplitMix64::new(9);
    let minutes = random_minutes(&mut rng, 100_000, 5, 375, 0, 0, 6, 2);
    let feed = SecondFeed {
        minutes: &minutes,
        secs: &[],
        step: 5,
        data_start: OPEN,
    };
    let mut points = Vec::new();
    for &n in &SIZES {
        let candles = long_candles(n + 1, n);
        let top = vec![true; candles.len()];
        let (p50, p99) = time_n(samples_for(n), 1, || {
            black_box(e2(black_box(&candles), &top, &feed, &p, &r));
        });
        let per = n as f64;
        points.push((n, p50 / per, p99 / per));
    }
    Row {
        id: "FIBREF-B03",
        what: "E2 touch book, per candle",
        o1: "per candle O(1) with a constant 12-rung sort for the bounce rung; a sequence of n candles in one call",
        points,
    }
}

fn bench_walk() -> Row {
    let decided = OPEN + 3_600;
    let mut points = Vec::new();
    for &n in &SIZES {
        // n - 2,000 untraded seconds before the decision (the binary searches
        // grow with them), then 2,000 traded seconds the walk reads
        let pre = n.saturating_sub(2_000);
        let mut rng = SplitMix64::new(n);
        let mut secs: Vec<Sec> = (0..pre)
            .map(|k| Sec {
                t: u32::try_from(k * u64::from(decided - 1) / pre.max(1)).unwrap_or(0),
                o: 100_000,
                h: 100_000,
                l: 100_000,
                c: 100_000,
                v: 0,
            })
            .collect();
        let mut px = 100_000i64;
        secs.extend((0..2_000u32).map(|k| {
            px += 5 * rng.range(-1, 1);
            Sec {
                t: decided + k,
                o: px,
                h: px + 5,
                l: px - 5,
                c: px,
                v: rng.range(100, 500),
            }
        }));
        let plan = Plan {
            side: Side::Long,
            decided_at: decided,
            amount: 1_000_000,
            stop: 1,
            exit: Exit::MarketAt(decided + 300),
            lambda: 14,
            share_ppm: 100_000,
            step: 5,
            session_end: SESSION_END,
            data_start: OPEN,
        };
        let (p50, p99) = time(16, || {
            black_box(walk(
                black_box(&secs),
                &plan,
                WithinSecond::EntryFirst,
                RestingAfterCut::Acts,
            ));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B04",
        what: "walk, one 5-minute round trip",
        o1: "NOT O(1) in the day's seconds: the start and each LTP are binary searches, O(log n); per walked second O(1)",
        points,
    }
}

fn bench_account() -> Row {
    let r = Readings::HEADLINE;
    let mut points = Vec::new();
    for &n in &SIZES {
        let mut acct = Account::new(5, &MAIN, &r, 1_000_000_000_000);
        let oc = Outcome {
            status: Status::Ok,
            filled: 10,
            first_fill: Some(OPEN + 100),
            end: OPEN + 200,
            net: 1,
            charges: 1,
        };
        let mk = |k: u64| -> Signal {
            let day = Date::from_days(10_000 + i64::try_from(k / 7).unwrap_or(0));
            Signal {
                id: u32::try_from(k % u64::from(u32::MAX)).unwrap_or(0),
                sym: u32::try_from(k % 50).unwrap_or(0),
                date: day,
                decided_at: OPEN + 50 + u32::try_from(k % 7).unwrap_or(0) * 300,
                start: OPEN + 64 + u32::try_from(k % 7).unwrap_or(0) * 300,
                lev: Leverage::FIVE,
                outcomes: [oc; 5],
            }
        };
        for k in 0..n {
            black_box(acct.offer(&mk(k)));
        }
        let mut k = n;
        let (p50, p99) = time(64, || {
            black_box(acct.offer(&mk(k)));
            k += 1;
        });
        points.push((n, p50, p99));
        drop(acct);
    }
    let _ = WALKED;
    Row {
        id: "FIBREF-B05",
        what: "account admission, one offer",
        o1: "O(1) amortised: hash-map day and month counts, an open list of at most N, a heap of at most N + 1 events (O(log N))",
        points,
    }
}

fn bench_charges() -> Row {
    let r = Readings::HEADLINE;
    let d = Date {
        y: 2026,
        m: 10,
        d: 30,
    };
    let orders = [
        Order {
            side: OrderSide::Buy,
            fills: vec![(547, 274_200)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(547, 274_300)],
            square_off: false,
        },
    ];
    let mut points = Vec::new();
    for &n in &SIZES {
        let (p50, p99) = time(64, || {
            black_box(price_equity(black_box(d), black_box(&orders), &r));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B06",
        what: "charges, one round trip, both clocks",
        o1: "O(fills): one fill a side here",
        points,
    }
}

fn bench_gate() -> Row {
    let r = Readings::HEADLINE;
    let d = Date {
        y: 2026,
        m: 9,
        d: 9,
    };
    let mut points = Vec::new();
    for &n in &SIZES {
        let (p50, p99) = time(64, || {
            let (ds, dt) = gates::distances(
                Side::Long,
                100_000,
                100_020,
                100_020,
                99_695_000,
                Some(100_605_000),
            );
            let q = gates::planned_qty(black_box(100_000_000), 100_020);
            let ch = gates::planned_charges(d, q, 100_020, &r);
            let _ = black_box((gates::s_stop(ds, 100_020), gates::s_rr_net(ds, dt, q, ch)));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B07",
        what: "S-STOP and S-RR, one record",
        o1: "O(1)",
        points,
    }
}

fn bench_ranking() -> Row {
    let a = Key {
        passer: false,
        dd_tenths: 3,
        dd: 6,
        net: 100,
        wins: 3,
        trades: 5,
        hold: 50,
        shortfall: 10,
        worst: -5,
        at_bar: 0,
        months: 16,
        id: 1,
    };
    let b = Key { id: 2, ..a };
    let mut points = Vec::new();
    for &n in &SIZES {
        let (p50, p99) = time(64, || {
            black_box(cmp_ranking(black_box(&a), black_box(&b)));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B08",
        what: "THE RANKING, one comparison",
        o1: "O(1)",
        points,
    }
}

fn bench_key() -> Row {
    let r = Readings::HEADLINE;
    let mut points = Vec::new();
    for &n in &SIZES[..4] {
        let trades: Vec<SeriesTrade> = (0..n)
            .map(|k| SeriesTrade {
                month: 24_200 + i32::try_from(k % 98).unwrap_or(0),
                net: 1_000,
                hold: 60,
            })
            .collect();
        let s = Series { id: 1, trades };
        let (p50, p99) = time(1, || {
            black_box(key(black_box(&s), 24_200, 24_297, &r));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B09",
        what: "a series' key over its months",
        o1: "NOT O(1): linear in the series' trades (once per series and window; sizes to 10^6)",
        points,
    }
}

fn bench_rank() -> Row {
    let mut points = Vec::new();
    for &n in &[218u64, 1_000, 10_000, 100_000] {
        let mut rng = SplitMix64::new(n);
        let pool: Vec<Mover> = (0..n)
            .map(|s| {
                let c = rng.range(10_000, 400_000);
                Mover {
                    sym: u32::try_from(s).unwrap_or(0),
                    p: c + rng.range(-c / 20, c / 20),
                    c,
                    stale_min: 0,
                }
            })
            .collect();
        let (p50, p99) = time(1, || {
            black_box(frame(black_box(&pool), None));
        });
        points.push((n, p50, p99));
    }
    Row {
        id: "FIBREF-B10",
        what: "rank frame of a pool of P",
        o1: "NOT O(1): a sort, O(P log P) per frame (P at most 218 in use); a lookup after it is O(1)",
        points,
    }
}

fn main() {
    println!("FIB-PREVIEW-REF benches (PREVIEW: synthetic inputs; this build's timings only)");
    for row in [
        bench_ladder(),
        bench_e1(),
        bench_e2(),
        bench_walk(),
        bench_account(),
        bench_charges(),
        bench_gate(),
        bench_ranking(),
        bench_key(),
        bench_rank(),
    ] {
        row.print();
    }
}
