//! Gate G0 (design v2 9.2): crate v2 against its frozen vectors (`vectors/COMMON-FILL-VECTORS-v2.tsv`, design 6 E1-E9 incl. E1 / E2 on
//! the REAL seconds, the out-of-box cases) and the extreme-case matrix (`matrix/MATRIX-v2.tsv`, design 10), both written by the
//! independent node reference (`vectors/reference.mjs`). Every field is compared byte for byte through one canonical string. Under
//! `--features red` (the old rule) the matrix test counts the cells whose output differs from the standard, checks that each equals the
//! node reference's red reading, prints the count and FAILS (the red round); the vectors fail there too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::too_many_arguments, clippy::needless_range_loop, clippy::type_complexity, clippy::cast_lossless, reason = "tests")]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::legs::walk_pair;
use crate::*;

fn sec(us: Us) -> String {
    if us == 0 { "0".to_owned() } else { (us / US_S).to_string() }
}

/// The node reference's canonical string of one level (reference.mjs `canon`).
pub(crate) fn canon(f: &LevelFill, fills: &[BlockFill]) -> String {
    let mut fl = String::new();
    for (i, b) in fills.iter().enumerate() {
        if i > 0 {
            fl.push(';');
        }
        let _ = write!(fl, "{}~{}~{}~{}~{}", sec(b.block.at), sec(b.block.to), b.units, b.px, sec(b.block.known));
    }
    format!("{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/[{fl}]", f.units, f.value, f.blocks_filled, sec(f.e0), sec(f.e1), sec(f.known_at), f.rest_at_stop, f.unpaid, f.past_limit, f.clipped)
}

fn refused(e: FillErr) -> String {
    format!("refused:{e:?}")
}

pub(crate) fn parse_rows(s: &str) -> Vec<SecIn> {
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split(',')
        .map(|r| {
            let v: Vec<i64> = r.split(':').map(|x| x.parse().unwrap()).collect();
            SecIn { ts: v[0] * US_S, high: v[1], low: v[2], vol: u64::try_from(v[3]).unwrap() }
        })
        .collect()
}

pub(crate) fn parse_segs(s: &str) -> Vec<Seg> {
    if s == "-" {
        return Vec::new();
    }
    s.split(',')
        .map(|g| {
            let v: Vec<i64> = g.split(':').map(|x| x.parse().unwrap()).collect();
            Seg { open: v[0] * US_S, last_sec: v[1] * US_S, auction_open: v[2] == 1 }
        })
        .collect()
}

/// The day index of `rows` covering every instant the case reads.
pub(crate) fn index_for(rows: &[SecIn], lo: Us, hi: Us) -> Result<SecIndex, FillErr> {
    let t0 = rows.first().map_or(lo, |r| r.ts.min(lo));
    let t1 = rows.last().map_or(hi, |r| r.ts.max(hi));
    SecIndex::new(rows, t0 - t0.rem_euclid(US_S), t1 + US_S)
}

fn kv(spec: &str) -> BTreeMap<&str, &str> {
    spec.split(' ').filter_map(|p| p.split_once('=')).collect()
}

fn side_of(s: &str) -> Side {
    if s == "B" { Side::Buy } else { Side::Sell }
}

fn opt_i(s: &str) -> Option<i64> {
    if s == "-" { None } else { Some(s.parse().unwrap()) }
}

/// One walk of every level of `dial` (one pass), canonical per level.
fn walk_levels(tape: &Tape<'_>, o: &Order, dial: &[u16], want: Units, pay: Option<Paisa>) -> Vec<Result<String, FillErr>> {
    let d = Dial::new(dial).unwrap();
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(dial.len()) {
        *x = want;
    }
    let mut f = [LevelFill::default(); MAXL];
    let mut fills: Vec<Vec<BlockFill>> = vec![Vec::new(); MAXL];
    let money = pay.map(|p| [Some(p); MAXL]);
    let got = {
        let mut rec = |l: usize, b: &BlockFill| {
            if b.why == Why::Filled {
                fills[l].push(*b);
            }
        };
        walk(tape, o, &d, LevelMask::all(dial.len()), &mut w, money.as_ref(), None, &mut f, Some(&mut rec))
    };
    (0..dial.len()).map(|l| got.map(|()| canon(&f[l], &fills[l]))).collect()
}

/// One vector row through the crate (its kind's executor), as the node reference's canonical string.
fn run_vector(kind: &str, tape_s: &str, segs_s: &str, spec: &str) -> String {
    let p = kv(spec);
    match kind {
        "walk" => {
            let rows = parse_rows(tape_s);
            let segs = parse_segs(segs_s);
            let anchor = p["anchor"].parse::<i64>().unwrap() * US_S;
            let stop = p["stop"].parse::<i64>().unwrap() * US_S;
            let ix = match index_for(&rows, anchor, stop.min(anchor + 86_400 * US_S)) {
                Ok(ix) => ix,
                Err(e) => return refused(e),
            };
            let mm = RangeMm::new(&rows);
            let tape = Tape::new(&rows, &ix, Some(&mm)).unwrap().with_segments(&segs).unwrap();
            let target = opt_i(p["target"]);
            let o = Order { side: side_of(p["side"]), anchor, stop, unit: p["unit"].parse().unwrap(), limit: opt_i(p["limit"]), kind: target.map_or(Kind::Entry, |px| Kind::TargetLimit { px }) };
            let dial: Vec<u16> = p["dial"].split(',').map(|x| x.parse().unwrap()).collect();
            let want: Units = p["want"].parse().unwrap();
            let pay = opt_i(p["pay"]);
            walk_levels(&tape, &o, &dial, want, pay).into_iter().zip(&dial).map(|(r, bp)| format!("l{bp}={}", r.unwrap_or_else(refused))).collect::<Vec<_>>().join("|")
        }
        "seq" => {
            let rows = parse_rows(tape_s);
            let segs = parse_segs(segs_s);
            let ix = index_for(&rows, rows[0].ts, rows[rows.len() - 1].ts).unwrap();
            let tape = Tape::new(&rows, &ix, None).unwrap().with_segments(&segs).unwrap();
            let bp: u16 = p["bp"].parse().unwrap();
            let d = Dial::new(&[bp]).unwrap();
            let mut led = Ledger::new();
            let mut out = Vec::new();
            for o in p["orders"].split(';') {
                let v: Vec<&str> = o.split(',').collect();
                let ord = Order { side: side_of(v[0]), anchor: v[1].parse::<i64>().unwrap() * US_S, stop: v[2].parse::<i64>().unwrap() * US_S, unit: v[3].parse().unwrap(), limit: None, kind: Kind::Entry };
                let mut w = [v[4].parse::<u64>().unwrap(), 0, 0, 0, 0, 0, 0, 0];
                let mut f = [LevelFill::default(); MAXL];
                let mut fills = Vec::new();
                {
                    let mut rec = |_l: usize, b: &BlockFill| {
                        if b.why == Why::Filled {
                            fills.push(*b);
                        }
                    };
                    walk_ledger(&tape, &ord, &d, LevelMask::all(1), &mut w, None, None, &mut led, &mut f, Some(&mut rec)).unwrap();
                }
                out.push(canon(&f[0], &fills));
            }
            out.join("|")
        }
        "pair" => {
            let (a, b) = tape_s.split_once('#').unwrap();
            let (rl, rf) = (parse_rows(a), parse_rows(b));
            let anchor = p["anchor"].parse::<i64>().unwrap() * US_S;
            let (il, ifo) = (index_for(&rl, anchor, anchor).unwrap(), index_for(&rf, anchor, anchor).unwrap());
            let (tl, tf) = (Tape::new(&rl, &il, None).unwrap(), Tape::new(&rf, &ifo, None).unwrap());
            let unit: Units = p["unit"].parse().unwrap();
            let leg = |s: &str| {
                let (sd, st) = s.split_once(',').unwrap();
                Order { side: side_of(sd), anchor, stop: st.parse::<i64>().unwrap() * US_S, unit, limit: None, kind: Kind::Leg { leg: 0, open: true } }
            };
            let (ol, of) = (leg(p["lead"]), leg(p["follow"]));
            let dial: Vec<u16> = p["dial"].split(',').map(|x| x.parse().unwrap()).collect();
            let want: Units = p["want"].parse().unwrap();
            pair_canon(&tl, &ol, &tf, &of, &dial, want)
        }
        "size" => {
            let rows = parse_rows(tape_s);
            let segs = parse_segs(segs_s);
            let t = p["t"].parse::<i64>().unwrap() * US_S;
            let ix = index_for(&rows, t, t).unwrap();
            let tape = Tape::new(&rows, &ix, None).unwrap().with_segments(&segs).unwrap();
            match limits::size_ref(&tape, t, side_of(p["side"])) {
                None => "none".to_owned(),
                Some(r) => format!("{r}/{}", limits::size_units(p["slot"].parse().unwrap(), p["lev"].parse().unwrap(), 1, r, p["unit"].parse().unwrap())),
            }
        }
        "trig" => {
            let rows = parse_rows(tape_s);
            let segs = parse_segs(segs_s);
            let anchor = p["anchor"].parse::<i64>().unwrap() * US_S;
            let stop = p["stop"].parse::<i64>().unwrap() * US_S;
            let to = p["to"].parse::<i64>().unwrap() * US_S;
            let ix = index_for(&rows, anchor, to).unwrap();
            let mm = RangeMm::new(&rows);
            let tape = Tape::new(&rows, &ix, Some(&mm)).unwrap().with_segments(&segs).unwrap();
            let long = p["side"] == "B";
            let o = Order { side: side_of(p["side"]), anchor, stop, unit: p["unit"].parse().unwrap(), limit: None, kind: Kind::Entry };
            let bp: u16 = p["bp"].parse().unwrap();
            let d = Dial::new(&[bp]).unwrap();
            let mut w = [p["want"].parse::<u64>().unwrap(), 0, 0, 0, 0, 0, 0, 0];
            let mut f = [LevelFill::default(); MAXL];
            let mut fills = Vec::new();
            {
                let mut rec = |_l: usize, b: &BlockFill| {
                    if b.why == Why::Filled {
                        fills.push(*b);
                    }
                };
                walk(&tape, &o, &d, LevelMask::all(1), &mut w, None, None, &mut f, Some(&mut rec)).unwrap();
            }
            let rules: Vec<path::Rule> = p["rules"]
                .split(',')
                .map(|r| {
                    let (k, v) = r.split_once(':').unwrap();
                    match k {
                        "stop" => path::Rule::Stop { x: v.parse().unwrap() },
                        "target" => path::Rule::Target { x: v.parse().unwrap() },
                        "time" => path::Rule::TimeExit { secs: v.parse().unwrap() },
                        "gopos" => path::Rule::GoPositive { secs: v.parse().unwrap() },
                        "trail" => {
                            let (a, b) = v.split_once('/').unwrap();
                            path::Rule::Trail { act: a.parse().unwrap(), dist: b.parse().unwrap() }
                        }
                        _ => panic!("rule {k}"),
                    }
                })
                .collect();
            let e = path::EntryView { e0: f[0].e0, e1: f[0].e1, fills: &fills };
            let mut st = 0;
            let (tg, _) = path::scan_one(&tape, &e, to, long, &rules, &[], &mut st);
            let t = tg.iter().map(|x| x.map_or_else(|| "none".to_owned(), |x| format!("{}~{}~{}", x.sec / US_S, x.anchor / US_S, x.level))).collect::<Vec<_>>().join("|");
            format!("{}#{t}", canon(&f[0], &fills))
        }
        "holes" => {
            let rows = parse_rows(tape_s);
            let segs = parse_segs(segs_s);
            let ix = index_for(&rows, rows[0].ts, rows[rows.len() - 1].ts).unwrap();
            let tape = Tape::new(&rows, &ix, None).unwrap().with_segments(&segs).unwrap();
            let h = find_holes(&tape);
            if h.is_empty() { "none".to_owned() } else { h.iter().map(|(a, b)| format!("{}~{}", a / US_S, b / US_S)).collect::<Vec<_>>().join(";") }
        }
        _ => panic!("unknown vector kind {kind}"),
    }
}

fn pair_canon(tl: &Tape<'_>, ol: &Order, tf: &Tape<'_>, of: &Order, dial: &[u16], want: Units) -> String {
    let d = Dial::new(dial).unwrap();
    let mut w = [0_u64; MAXL];
    for x in w.iter_mut().take(dial.len()) {
        *x = want;
    }
    let (mut fl, mut ff) = ([LevelFill::default(); MAXL], [LevelFill::default(); MAXL]);
    let mut rows: [Vec<BlockFill>; 2 * MAXL] = Default::default();
    let got = {
        let mut rec = |l: usize, leg: u8, b: &BlockFill| {
            if b.why == Why::Filled {
                rows[2 * l + usize::from(leg)].push(*b);
            }
        };
        walk_pair((tl, ol), (tf, of), &d, LevelMask::all(dial.len()), &mut w, &mut fl, &mut ff, Some(&mut rec))
    };
    (0..dial.len())
        .map(|l| {
            let s = match got {
                Ok(()) => format!("{}&{}", canon(&fl[l], &rows[2 * l]), canon(&ff[l], &rows[2 * l + 1])),
                Err(e) => format!("{}&{}", refused(e), refused(e)),
            };
            format!("l{}={s}", dial[l])
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn g0_every_vector_of_common_fill_vectors_v2_is_reproduced_byte_for_byte() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/vectors/COMMON-FILL-VECTORS-v2.tsv")).unwrap();
    let mut n = 0;
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.starts_with("id\t")) {
        let c: Vec<&str> = line.split('\t').collect();
        let got = run_vector(c[1], c[3], c[4], c[5]);
        if got != c[6] {
            bad.push(format!("{}: got {} want {}", c[0], got.chars().take(300).collect::<String>(), c[6].chars().take(300).collect::<String>()));
        }
        n += 1;
    }
    assert!(n >= 28, "{n} vectors");
    assert!(bad.is_empty(), "{} of {n} vectors differ:\n{}", bad.len(), bad.join("\n"));
}

/// One matrix cell through the crate.
fn run_cell(c: &[&str], tapes: &BTreeMap<String, (Vec<SecIn>, Vec<Seg>)>) -> String {
    let (leg, size, acct, opt) = (c[2], c[4], c[6], c[7]);
    let bp: u16 = c[5].parse().unwrap();
    let anchor = c[11].parse::<i64>().unwrap() * US_S;
    let stop = c[12].parse::<i64>().unwrap() * US_S;
    let side = side_of(c[13]);
    let unit: Units = c[14].parse().unwrap();
    let limit = opt_i(c[15]);
    let want: Units = c[16].parse().unwrap();
    let pay = opt_i(c[17]);
    let _ = size;
    if opt == "stock-own-expiry" {
        return limits::expiry_guard(true, 20_389, 20_389).map_or_else(refused, |()| "ok".to_owned());
    }
    let (rows, segs) = &tapes[c[9]];
    let ix = index_for(rows, anchor, stop).unwrap();
    let mm = RangeMm::new(rows);
    let tape = Tape::new(rows, &ix, Some(&mm)).unwrap().with_segments(segs).unwrap();
    let o = Order { side, anchor, stop, unit, limit, kind: Kind::Entry };
    if leg == "sold-leg" || leg == "hedge-leg" {
        let (prow, psegs) = &tapes[c[10]];
        let pix = index_for(prow, anchor, stop).unwrap();
        let ptape = Tape::new(prow, &pix, None).unwrap().with_segments(psegs).unwrap();
        let po = Order { side: side.other(), anchor, stop, unit, limit: None, kind: Kind::Entry };
        let s = if c[18] == "cell" { pair_canon(&tape, &o, &ptape, &po, &[bp], want) } else { pair_canon(&ptape, &po, &tape, &o, &[bp], want) };
        return s.split_once('=').map_or(s.clone(), |x| x.1.to_owned());
    }
    let r = walk_levels(&tape, &o, &[bp], want, pay).remove(0);
    let Ok(base) = r else { return r.unwrap_err().pipe(refused) };
    // The canonical string's fields (units/value/.../rest) for the extras.
    let f: Vec<&str> = base.split('/').collect();
    let (units, value, rest): (u64, i64, u64) = (f[0].parse().unwrap(), f[1].parse().unwrap(), f[6].parse().unwrap());
    let mut s = base.clone();
    if acct == "excess-over-reserve" {
        let _ = write!(s, "|excess={}", (money::money_in(value, 5, 1) - value / 10).max(0));
    }
    if leg == "index-sliced" {
        let _ = write!(s, "|slices={}", legs::freeze_slices(units, 1_800));
    }
    if opt == "index-expiry-settle" {
        let _ = write!(s, "|settle={}", if rest == 0 { 0 } else { limits::settle_intrinsic(true, 2_000_000, 2_010_000).unwrap() * i64::try_from(rest).unwrap() });
    }
    s
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl Pipe for FillErr {}

#[test]
fn g0_every_cell_of_the_extreme_case_matrix_equals_the_node_reference_and_the_red_round_counts_its_cells() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/matrix/MATRIX-v2.tsv")).unwrap();
    let mut tapes: BTreeMap<String, (Vec<SecIn>, Vec<Seg>)> = BTreeMap::new();
    let mut lines = text.lines();
    let head = lines.next().unwrap();
    assert!(head.contains("1128960 raw cells"), "{head}");
    let nt: usize = lines.next().unwrap().split('\t').nth(1).unwrap().parse().unwrap();
    for _ in 0..nt {
        let c: Vec<&str> = lines.next().unwrap().split('\t').collect();
        tapes.insert(c[0].to_owned(), (parse_rows(c[1]), parse_segs(c[2])));
    }
    let ncells: usize = lines.next().unwrap().split('\t').nth(1).unwrap().parse().unwrap();
    let _header = lines.next().unwrap();
    let (mut n, mut std_bad, mut red_cells, mut red_seen, mut red_mismatch) = (0_usize, Vec::new(), 0_usize, 0_usize, Vec::new());
    for line in lines {
        let c: Vec<&str> = line.split('\t').collect();
        let got = run_cell(&c, &tapes);
        let (std, red, rd) = (c[19], if c[20] == "=" { c[19] } else { c[20] }, c[21] == "1");
        red_cells += usize::from(rd);
        if RED {
            if got != std {
                red_seen += 1;
            }
            if got != red {
                red_mismatch.push(format!("cell {} {}/{}/{}/{}: red got {} want {}", c[0], c[2], c[3], c[4], c[6], got.chars().take(200).collect::<String>(), red.chars().take(200).collect::<String>()));
            }
        } else if got != std {
            std_bad.push(format!("cell {} {} {} {} {} {} {} {}: got {} want {}", c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], got.chars().take(240).collect::<String>(), std.chars().take(240).collect::<String>()));
        }
        n += 1;
    }
    assert_eq!(n, ncells);
    if RED {
        // THE RED ROUND (design 10.4): every cell whose standard output differs from the old rule fails; each such cell must also be
        // exactly the node reference's red reading.
        println!("RED ROUND: {red_seen} of {n} matrix cells differ from the standard (the node reference counts {red_cells}); red-reading mismatches {}", red_mismatch.len());
        assert!(red_mismatch.is_empty(), "the red build differs from the reference's red reading:\n{}", red_mismatch.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
        assert_eq!(red_seen, red_cells);
        panic!("red round: {red_seen} of {n} cells differ from the standard");
    }
    println!("MATRIX: {n} cells (1,128,960 raw, validity rules V1-V5), every one equal to the node reference; {red_cells} differ under the red reading");
    assert!(std_bad.is_empty(), "{} of {n} cells differ:\n{}", std_bad.len(), std_bad.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}
