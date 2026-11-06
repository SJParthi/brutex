#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::needless_range_loop,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::market::Bar;
use crate::rng::SplitMix64;
use crate::time::{CLOSE_FULL, OPEN};

fn sec(at: u32, high: i64, low: i64) -> Second {
    Second {
        at,
        bar: Bar::new(low, high, low, low, 1),
    }
}

/// A day whose Zerodha minutes and GDFL seconds agree at `price` (ratio 1).
fn flat_tape(price: i64, until: u32) -> (Vec<Minute>, Vec<Second>) {
    let minutes: Vec<Minute> = (0..375)
        .map(|i| Minute {
            start: OPEN + 60 * i,
            bar: Bar::flat(price, 10),
        })
        .collect();
    let seconds: Vec<Second> = (0..375)
        .map(|i| OPEN + 60 * i + 1)
        .filter(|at| *at < until)
        .map(|at| sec(at, price, price))
        .collect();
    (minutes, seconds)
}

fn tape<'a>(m: &'a [Minute], s: &'a [Second]) -> Tape<'a> {
    Tape {
        minutes: m,
        seconds: s,
        fallback: false,
        close: CLOSE_FULL,
    }
}

fn filled(o: Outcome) -> Simulated {
    match o {
        Outcome::Filled(s) => s,
        other => panic!("not filled: {other:?}"),
    }
}

const T: u32 = OPEN + 30 * 60;

#[test]
fn t_f39_a_quiet_long_reaches_the_time_exit() {
    let (m, s) = flat_tape(100_000, CLOSE_FULL);
    let sim = filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &s)));
    assert_eq!(
        (sim.entry.second, sim.entry.price, sim.qty),
        (T + 1, 100_000, 500)
    );
    assert_eq!(
        (sim.kind, sim.exit.second, sim.exit.price),
        (ExitKind::Time, OPEN + 60 * 353 + 1, 100_000)
    );
    assert_eq!(
        (sim.first_stop, sim.last_stop, sim.late, sim.fallback),
        (99_000, 99_000, false, false)
    );
    assert!(
        ExitKind::SquareOff.by_broker()
            && ExitKind::Unclosable.by_broker()
            && !ExitKind::Flip.by_broker()
    );
    assert_eq!(
        ExitKind::ALL.map(ExitKind::label),
        ["stop", "flip", "time", "square-off", "unclosable"]
    );
}

/// X-F30..X-F39: the walk at its edges.
#[test]
fn x_f39_trade_walk_extremes() {
    let (m, s) = flat_tape(100_000, CLOSE_FULL);
    // gap through: a second far below the stop fills there in full
    let mut gap = s.clone();
    gap[40] = sec(gap[40].at, 100_000, 90_000);
    let sim = filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &gap)));
    assert_eq!((sim.kind, sim.exit.price), (ExitKind::Stop, 90_000));
    // the entry second reaching the stop does not fire (the stop is live after it)
    let mut first = s.clone();
    first[30] = sec(first[30].at, 100_000, 50_000);
    assert_eq!(
        filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &first))).kind,
        ExitKind::Time
    );
    // a flip: the candle closes under its band; the fill is the next second's low
    let later = [Later {
        t: T + 600,
        close: 98_000,
        band: 99_500,
        hole: false,
    }];
    let sim = filled(simulate(Side::Long, T, 99_000, &later, &tape(&m, &s)));
    assert_eq!((sim.kind, sim.exit.second), (ExitKind::Flip, T + 601));
    // combo: the flip's second also reaches the stop: one exit, a flip
    let mut both = s.clone();
    both[40] = sec(T + 601, 100_000, 95_000);
    assert_eq!(
        filled(simulate(Side::Long, T, 99_000, &later, &tape(&m, &both))).kind,
        ExitKind::Flip
    );
    // a hole candle moves nothing and fires no flip
    let hole = [Later {
        t: T + 600,
        close: 1,
        band: 1_000_000,
        hole: true,
    }];
    assert_eq!(
        filled(simulate(Side::Long, T, 99_000, &hole, &tape(&m, &s))).kind,
        ExitKind::Time
    );
    // a tighter band moves the stop, a looser one does not
    let moves = [
        Later {
            t: T + 600,
            close: 100_000,
            band: 99_800,
            hole: false,
        },
        Later {
            t: T + 900,
            close: 100_000,
            band: 99_100,
            hole: false,
        },
    ];
    assert_eq!(
        filled(simulate(Side::Long, T, 99_000, &moves, &tape(&m, &s))).last_stop,
        99_800
    );
    // Q3: the 15:07:59 second reaches the stop: the stop
    let mut q3 = s.clone();
    q3.push(sec(TIME_EXIT, 100_000, 98_000));
    q3.sort_by_key(|x| x.at);
    let sim = filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &q3)));
    assert_eq!((sim.kind, sim.exit.second), (ExitKind::Stop, TIME_EXIT));
    // no second in [15:07:59, 15:10): the broker's square-off at 15:10:00
    let mut sq: Vec<Second> = s
        .iter()
        .copied()
        .filter(|x| x.at < TIME_EXIT || x.at >= SQUARE_OFF)
        .collect();
    sq.push(sec(SQUARE_OFF, 100_100, 99_900));
    sq.sort_by_key(|x| x.at);
    let sim = filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &sq)));
    assert_eq!(
        (sim.kind, sim.exit.second, sim.exit.price),
        (ExitKind::SquareOff, SQUARE_OFF, 99_900)
    );
    // Q4: a flip whose first second is 15:10:00: the square-off
    let flip_late = [Later {
        t: OPEN + 60 * 345,
        close: 98_000,
        band: 99_500,
        hole: false,
    }];
    let mut gapped: Vec<Second> = s
        .iter()
        .copied()
        .filter(|x| x.at <= OPEN + 60 * 345 || x.at >= SQUARE_OFF)
        .collect();
    gapped.push(sec(SQUARE_OFF, 100_000, 99_950));
    gapped.sort_by_key(|x| x.at);
    let sim = filled(simulate(
        Side::Long,
        T,
        99_000,
        &flip_late,
        &tape(&m, &gapped),
    ));
    assert_eq!(
        (sim.kind, sim.exit.second),
        (ExitKind::SquareOff, SQUARE_OFF)
    );
    // oob: halted from 11:00: unclosable at the lowest low since the entry, at the close
    let (hm, hs) = flat_tape(100_000, 11 * 3_600);
    let mut halted = hs;
    halted[35] = sec(halted[35].at, 100_000, 99_500);
    let sim = filled(simulate(Side::Long, T, 99_000, &[], &tape(&hm, &halted)));
    assert_eq!(
        (sim.kind, sim.exit.second, sim.exit.price),
        (ExitKind::Unclosable, CLOSE_FULL, 99_500)
    );
    // a flip with no second left: unclosable
    let flip_halt = [Later {
        t: OPEN + 60 * 120,
        close: 98_000,
        band: 99_500,
        hole: false,
    }];
    assert_eq!(
        filled(simulate(
            Side::Short,
            T,
            101_000,
            &flip_halt,
            &tape(&hm, &halted)
        ))
        .kind,
        ExitKind::Unclosable
    );
}

#[test]
fn x_f30_entry_refusals_and_late_fills() {
    let (m, s) = flat_tape(100_000, CLOSE_FULL);
    assert_eq!(
        simulate(Side::Long, CLOSE_FULL, 99_000, &[], &tape(&m, &s)),
        Outcome::NotFilled
    );
    let early: Vec<Second> = s.iter().copied().filter(|x| x.at > T).collect();
    assert_eq!(
        simulate(Side::Long, T, 99_000, &[], &tape(&m, &early)),
        Outcome::NoRatio
    );
    assert_eq!(
        simulate(Side::Long, T, -5, &[], &tape(&m, &s)),
        Outcome::NoRatio
    );
    let (bm, bs) = flat_tape(60_000_000, CLOSE_FULL);
    assert_eq!(
        simulate(Side::Long, T, 59_000_000, &[], &tape(&bm, &bs)),
        Outcome::ZeroQuantity
    );
    // 61 s late is late; 60 s is not
    let late: Vec<Second> = s
        .iter()
        .copied()
        .filter(|x| x.at < T || x.at > T + 60)
        .collect();
    assert!(filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &late))).late);
    let mut on_time: Vec<Second> = late.clone();
    on_time.push(sec(T + 60, 100_000, 100_000));
    on_time.sort_by_key(|x| x.at);
    assert!(!filled(simulate(Side::Long, T, 99_000, &[], &tape(&m, &on_time))).late);
    // an entry at or after 15:07:59 goes straight to the square-off rule (no time order)
    let mut tail: Vec<Second> = s
        .iter()
        .copied()
        .filter(|x| x.at < OPEN + 60 * 352)
        .collect();
    tail.push(sec(TIME_EXIT + 10, 100_000, 100_000));
    tail.push(sec(SQUARE_OFF + 5, 100_000, 100_000));
    let sim = filled(simulate(
        Side::Long,
        OPEN + 60 * 352,
        99_000,
        &[],
        &tape(&m, &tail),
    ));
    assert_eq!(
        (sim.entry.second, sim.kind, sim.exit.second),
        (TIME_EXIT + 10, ExitKind::SquareOff, SQUARE_OFF + 5)
    );
}

#[test]
fn x_f38_fallback_days_fill_on_minutes() {
    let (m, _) = flat_tape(100_000, CLOSE_FULL);
    let fb = Tape {
        minutes: &m,
        seconds: &[],
        fallback: true,
        close: CLOSE_FULL,
    };
    let sim = filled(simulate(Side::Short, T, 101_000, &[], &fb));
    assert_eq!(
        (sim.entry.second, sim.fallback, sim.kind),
        (T + 59, true, ExitKind::Time)
    );
    assert_eq!(sim.exit.second, OPEN + 60 * 353 + 59);
    // the 15:07 minute entry: slot 15:07:59, then the time order on the 15:08 minute
    let sim = filled(simulate(Side::Long, OPEN + 60 * 352, 99_000, &[], &fb));
    assert_eq!(
        (sim.entry.second, sim.kind, sim.exit.second),
        (TIME_EXIT, ExitKind::Time, OPEN + 60 * 353 + 59)
    );
    // a level that is not positive is refused on a fallback day too
    assert_eq!(simulate(Side::Long, T, 0, &[], &fb), Outcome::NoRatio);
    let moved = [
        Later {
            t: T + 600,
            close: 100_000,
            band: 99_900,
            hole: false,
        },
        Later {
            t: T + 660,
            close: 100_000,
            band: -1,
            hole: false,
        },
    ];
    let sim = filled(simulate(Side::Long, T, 99_000, &moved, &fb));
    assert_eq!((sim.last_stop, sim.no_ratio_trail), (99_900, 1));
}

/// The independent replay of P-F39-1: SPEC section 3.5's rules as one forward loop over
/// prints, the candles applied at their instants, written apart from `walk`.
fn replay(side: Side, t: u32, band: i128, later: &[Later], tape: &Tape<'_>) -> Outcome {
    let prints = if tape.fallback {
        crate::fills::minute_prints(tape.minutes, tape.close)
    } else {
        crate::fills::second_prints(tape.seconds)
    };
    let level = |b: i128, at: u32| -> Option<i64> {
        if tape.fallback {
            return i64::try_from(b).ok().filter(|v| *v > 0);
        }
        let r = crate::pricemap::ratio_source(tape.minutes, tape.seconds, at)?;
        crate::pricemap::map_level(b, &r, side).ok()
    };
    let Some(e) = prints.iter().position(|p| p.start >= t) else {
        return Outcome::NotFilled;
    };
    let long = side == Side::Long;
    let entry_price = if long { prints[e].high } else { prints[e].low };
    let Some(mut stop) = level(band, t) else {
        return Outcome::NoRatio;
    };
    let qty = crate::sizing::qty(entry_price);
    if qty == 0 {
        return Outcome::ZeroQuantity;
    }
    let hit = |stop: i64, i: usize| {
        if long {
            prints[i].low <= stop
        } else {
            prints[i].high >= stop
        }
    };
    let price = |i: usize| if long { prints[i].low } else { prints[i].high };
    let time_rule = prints[e].start < TIME_EXIT;
    let mut candles = later
        .iter()
        .filter(|c| c.t > prints[e].slot && c.t <= TIME_EXIT)
        .peekable();
    let mut flip = false;
    let mut moves = 0u32;
    let mut out = None;
    for i in e + 1..prints.len() {
        while let Some(c) = candles.peek().copied() {
            if c.t > prints[i].start || flip {
                break;
            }
            candles.next();
            if c.hole {
                continue;
            }
            let crossed = if long {
                i128::from(c.close) < c.band
            } else {
                i128::from(c.close) > c.band
            };
            if crossed {
                flip = true;
                continue;
            }
            match level(c.band, c.t) {
                Some(l) if (long && l > stop) || (!long && l < stop) => stop = l,
                Some(_) => {}
                None => moves += 1,
            }
        }
        let p = prints[i];
        let kind = if flip {
            Some(if p.start >= SQUARE_OFF {
                ExitKind::SquareOff
            } else {
                ExitKind::Flip
            })
        } else if hit(stop, i) {
            Some(ExitKind::Stop)
        } else if time_rule && p.start >= TIME_EXIT && p.start < SQUARE_OFF {
            Some(ExitKind::Time)
        } else if p.start >= SQUARE_OFF {
            Some(ExitKind::SquareOff)
        } else {
            None
        };
        if let Some(kind) = kind {
            out = Some((
                kind,
                Fill {
                    second: p.slot,
                    price: price(i),
                },
            ));
            break;
        }
    }
    // No print left: the candles still trail the stop (nothing can fill it).
    if out.is_none() && !flip {
        for c in candles.filter(|c| !c.hole) {
            let crossed = if long {
                i128::from(c.close) < c.band
            } else {
                i128::from(c.close) > c.band
            };
            if crossed {
                break;
            }
            match level(c.band, c.t) {
                Some(l) if (long && l > stop) || (!long && l < stop) => stop = l,
                Some(_) => {}
                None => moves += 1,
            }
        }
    }
    let (kind, exit) = out.unwrap_or_else(|| {
        let worst = if long {
            prints[e..].iter().map(|p| p.low).min()
        } else {
            prints[e..].iter().map(|p| p.high).max()
        };
        (
            ExitKind::Unclosable,
            Fill {
                second: tape.close,
                price: worst.unwrap(),
            },
        )
    });
    Outcome::Filled(Simulated {
        entry: Fill {
            second: prints[e].slot,
            price: entry_price,
        },
        qty,
        exit,
        kind,
        fallback: tape.fallback,
        late: prints[e].slot > t + 60,
        first_stop: level(band, t).unwrap(),
        last_stop: stop,
        no_ratio_trail: moves,
    })
}

/// P-F39-1: 10,000 generated stock-days against the independent replay.
#[test]
fn p_f39_simulate_vs_independent_replay() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let fallback = r.chance(1, 4);
            let base = 10_000 + i64::try_from(r.below(1_000)).unwrap();
            let mut minutes = Vec::new();
            let mut seconds = Vec::new();
            let mut price = base;
            for i in 0..375u32 {
                price = (price + i64::try_from(r.below(41)).unwrap() - 20).max(100);
                let v = if r.chance(1, 15) { 0 } else { 5 };
                if r.chance(19, 20) {
                    minutes.push(Minute {
                        start: OPEN + 60 * i,
                        bar: Bar::new(price, price + 5, price - 5, price, v),
                    });
                }
                for _ in 0..u32::try_from(r.below(3)).unwrap() {
                    let at = OPEN + 60 * i + u32::try_from(r.below(60)).unwrap();
                    seconds.push(sec(
                        at,
                        price + i64::try_from(r.below(8)).unwrap(),
                        price - i64::try_from(r.below(8)).unwrap(),
                    ));
                }
            }
            seconds.sort_by_key(|s| s.at);
            seconds.dedup_by_key(|s| s.at);
            if r.chance(1, 10) {
                let halt = OPEN + 60 * u32::try_from(r.below(375)).unwrap();
                seconds.retain(|s| s.at < halt);
            }
            let side = if r.chance(1, 2) {
                Side::Long
            } else {
                Side::Short
            };
            let t = OPEN + 60 * (u32::try_from(r.below(374)).unwrap() + 1);
            let off = i128::from(r.below(200));
            let band = if side == Side::Long {
                i128::from(base) - off
            } else {
                i128::from(base) + off
            };
            let mut later = Vec::new();
            let mut lt = t;
            while lt < CLOSE_FULL && later.len() < 40 {
                lt += 60 * (u32::try_from(r.below(5)).unwrap() + 1);
                let close = base + i64::try_from(r.below(400)).unwrap() - 200;
                let b = i128::from(base) + i128::from(r.below(400)) - 200;
                later.push(Later {
                    t: lt,
                    close,
                    band: b,
                    hole: r.chance(1, 8),
                });
            }
            let tp = Tape {
                minutes: &minutes,
                seconds: &seconds,
                fallback,
                close: CLOSE_FULL,
            };
            assert_eq!(
                simulate(side, t, band, &later, &tp),
                replay(side, t, band, &later, &tp),
                "seed {seed:#x}"
            );
        }
    }
}
