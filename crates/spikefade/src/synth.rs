//! v3.4 E12 (build:shard; cloud-kit/CLOUD-KIT-DESIGN.md E12): a SEEDED SYNTHETIC day set in the engine's own cube and seconds-tape formats,
//! for the data-free canary (C2): no market data, no vendor data. Every number comes from a splitmix64 stream of the seed and the day, so the
//! same seed gives the same bytes on every machine (named test `synthetic_is_reproducible`; the sha256 of every file is printed).
//!
//! A day holds index underlyings (sorted first, as the cube builder sorts them) and stock underlyings in the main root, and one more index in
//! an extra root (`--cube-extra`, critic F1's second range), with weekly / monthly expiries, strikes around a random-walk spot, CE and PE,
//! minute bars and second prints whose premium paths carry seeded SPIKES (+80% .. +600% inside a few minutes, then a fade), so the screener,
//! the events, the fills and the batch walk all run. Names are generic market names (the C_VAR1 proxy margin finds a row); the files are
//! labelled SYNTHETIC in their manifests. Nothing here is a trading result.

use std::fmt::Write as _;
use std::path::Path;

use crate::cal::{NM, OPEN_S};
use crate::cube::{Contract, Cube, Sec, SecTape, Und};

/// splitmix64 (public-domain constants): one deterministic stream per (seed, day, salt).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A stream for a seed and a salt.
    #[must_use]
    pub const fn new(seed: u64, salt: u64) -> Self {
        Self(seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }
    /// The next 64 bits.
    pub const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in [0, n) (n > 0).
    pub const fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

/// (name, kind, spot paisa, strike step paisa, lot, spot sid)
const MAIN_UNDS: [(&str, u8, i64, i64, u32, u32); 5] = [("NIFTY", 0, 1_700_000, 5_000, 50, 13), ("BANKNIFTY", 0, 4_000_000, 10_000, 25, 25), ("RELIANCE", 1, 240_000, 2_000, 250, 2885), ("INFY", 1, 150_000, 2_000, 300, 1594), ("TCS", 1, 330_000, 5_000, 150, 11536)];
const EXTRA_UNDS: [(&str, u8, i64, i64, u32, u32); 1] = [("FINNIFTY", 0, 1_800_000, 5_000, 40, 27)];

fn next_weekday(day: i64, wd: i64) -> i64 {
    // 1970-01-01 was a Thursday (weekday 3 with Monday = 0)
    let cur = (day + 3).rem_euclid(7);
    day + (wd - cur).rem_euclid(7)
}

fn month_last_thursday(day: i64) -> i64 {
    let (y, m, _) = crate::cal::civil(day);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let last = crate::cal::days_from_civil(ny, nm, 1) - 1;
    last - (last + 3 - 3).rem_euclid(7)
}

/// One synthetic day of one root.
#[must_use]
pub fn day(seed: u64, d: i64, extra: bool) -> (Cube, SecTape) {
    let ymd = crate::cal::ymd_of(d);
    let unds_def: &[(&str, u8, i64, i64, u32, u32)] = if extra { &EXTRA_UNDS } else { &MAIN_UNDS };
    let mut rng = Rng::new(seed, u64::try_from(d).unwrap_or(0) * 2 + u64::from(extra));
    let weekly = next_weekday(d, 3);
    let monthly = { let m = month_last_thursday(d); if m < d { month_last_thursday(m + 7) } else { m } };
    let mut c = Cube { ymd, day: d, ..Cube::default() };
    let mut spots: Vec<Vec<i32>> = Vec::new();
    for &(name, kind, s0, _, _, sid) in unds_def {
        let (near, next) = if kind == 0 { (weekly, weekly + 7) } else { (monthly, month_last_thursday(monthly + 7)) };
        c.unds.push(Und { name: name.to_owned(), kind, spot_seg: kind, spot_sid: sid, nearest: crate::cal::ymd_of(near), next: crate::cal::ymd_of(next) });
        let mut s = s0 + i64::try_from(rng.below(2_001)).unwrap_or(0) * (s0 / 100_000).max(1) - (s0 / 100);
        let mut v = Vec::with_capacity(NM);
        for _ in 0..NM {
            s += (i64::try_from(rng.below(201)).unwrap_or(100) - 100) * (s0 / 200_000).max(1);
            v.push(i32::try_from(s).unwrap_or(i32::MAX));
        }
        spots.push(v);
    }
    // contracts: index first (the builder's order), 8 strikes x CE / PE on the nearest expiry, 4 strikes x CE / PE on the next
    let mut order: Vec<usize> = (0..unds_def.len()).collect();
    order.sort_by_key(|&u| unds_def.get(u).map_or(1, |x| x.1));
    let mut sid = if extra { 900_000_u32 } else { 100_000_u32 };
    let mut paths: Vec<(usize, i64)> = Vec::new();
    for &u in &order {
        let Some(&(name, kind, _, step, lot, _)) = unds_def.get(u) else { continue };
        let und = c.unds.get(u).cloned().unwrap_or_default();
        let atm = i64::from(*spots.get(u).and_then(|v| v.first()).unwrap_or(&0)) / step * step;
        for (exp, ns) in [(und.nearest, 8_i64), (und.next, 4_i64)] {
            for k in -ns / 2..ns / 2 {
                for right in 0..2_u8 {
                    let strike = atm + k * step;
                    sid += 1;
                    let mon = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"].get(usize::try_from((exp / 100) % 100).unwrap_or(1).saturating_sub(1)).copied().unwrap_or("JAN");
                    let symbol = format!("{name}{:02}{mon}{}{}", (exp / 10_000) % 100, strike / 100, if right == 0 { "CE" } else { "PE" });
                    c.cons.push(Contract { sid, und: u16::try_from(u).unwrap_or(0), kind, right, expiry: exp, lot, strike, first_min: -1, first_open: 0, symbol, n_sec: 0 });
                    // the premium's start: cheaper further out of the money
                    let otm = if right == 0 { k } else { -k };
                    paths.push((u, (step * 3 / (2 + otm.max(-1) + 2)).max(500) + i64::try_from(rng.below(400)).unwrap_or(0)));
                }
            }
        }
    }
    let nc = c.cons.len();
    c.spot = spots.concat();
    c.close = vec![0; nc * NM];
    c.flags = vec![0; nc * NM];
    c.high = vec![0; nc * NM];
    c.low = vec![0; nc * NM];
    c.vol = vec![0; nc * NM];
    c.oi = vec![0; nc * NM];
    c.cumvol = vec![0; nc * (NM + 1)];
    c.cumval = vec![0; nc * (NM + 1)];
    let mut t = SecTape { off: vec![0], rows: Vec::new(), lim: None };
    for ci in 0..nc {
        let (_, p0) = paths.get(ci).copied().unwrap_or((0, 1_000));
        let lot = c.cons.get(ci).map_or(1, |k| k.lot);
        let busy = 40 + rng.below(55);
        // up to two seeded spikes a day: (start minute, peak factor in percent, minutes to the peak)
        let spikes: Vec<(usize, i64, usize)> = (0..rng.below(3)).map(|_| (usize::try_from(5 + rng.below(330)).unwrap_or(5), 80 + i64::try_from(rng.below(520)).unwrap_or(0), usize::try_from(1 + rng.below(6)).unwrap_or(1))).collect();
        let mut px = p0;
        let mut last_close = 0_i32;
        let (mut cv, mut cl) = (0_u64, 0_u64);
        let mut nsec = 0_u32;
        for m in 0..NM {
            let ix = Cube::ix(ci, m);
            // the path: a small random walk, plus a spike rising to its peak and fading back
            px += (i64::try_from(rng.below(41)).unwrap_or(20) - 20) * (px / 400).max(1);
            let mut f = 100_i64;
            for &(s0, pk, up) in &spikes {
                if m >= s0 && m < s0 + up {
                    f = f.max(100 + pk * i64::try_from(m - s0 + 1).unwrap_or(1) / i64::try_from(up).unwrap_or(1));
                } else if m >= s0 + up && m < s0 + up + 30 {
                    f = f.max(100 + pk * i64::try_from(s0 + up + 30 - m).unwrap_or(0) / 30);
                }
            }
            px = px.clamp(5, 5_000_000);
            let p = (px * f / 100).clamp(5, 50_000_000);
            let present = rng.below(100) < busy || f > 100;
            if let Some(x) = c.cumvol.get_mut(Cube::px(ci, m)) {
                *x = cv;
            }
            if let Some(x) = c.cumval.get_mut(Cube::px(ci, m)) {
                *x = cl;
            }
            if !present || m + 1 >= NM - 9 {
                if let Some(x) = c.close.get_mut(ix) {
                    *x = last_close;
                }
                if let Some(x) = c.flags.get_mut(ix) {
                    *x = if last_close > 0 { 2 } else { 0 };
                }
                continue;
            }
            let (mut hi, mut lo, mut v) = (0_i32, i32::MAX, 0_u32);
            let nprint = 1 + rng.below(if f > 100 { 40 } else { 12 });
            let mut secs: Vec<u32> = (0..nprint).map(|_| u32::try_from(rng.below(60)).unwrap_or(0)).collect();
            secs.sort_unstable();
            secs.dedup();
            let mut close = 0_i32;
            for s in secs {
                let q = i32::try_from(p + i64::try_from(rng.below(21)).unwrap_or(10) * (p / 200).max(1) - 10 * (p / 200).max(1)).unwrap_or(i32::MAX).max(5);
                let h = q + i32::try_from(rng.below(3)).unwrap_or(0) * 5;
                let qty = lot * u32::try_from(1 + rng.below(if f > 100 { 60 } else { 15 })).unwrap_or(1);
                t.rows.push(Sec { sod: u32::try_from(OPEN_S).unwrap_or(0) + u32::try_from(m).unwrap_or(0) * 60 + s, high: h, low: q, vol: qty });
                nsec += 1;
                hi = hi.max(h);
                lo = lo.min(q);
                v += qty;
                close = q;
            }
            if let Some(k) = c.cons.get_mut(ci)
                && k.first_min < 0
            {
                k.first_min = i32::try_from(m).unwrap_or(0);
                k.first_open = close;
            }
            for (arr, val) in [(&mut c.close, close), (&mut c.high, hi), (&mut c.low, lo)] {
                if let Some(x) = arr.get_mut(ix) {
                    *x = val;
                }
            }
            if let Some(x) = c.vol.get_mut(ix) {
                *x = v;
            }
            if let Some(x) = c.oi.get_mut(ix) {
                *x = lot * 1_000;
            }
            if let Some(x) = c.flags.get_mut(ix) {
                *x = 1 | 4;
            }
            last_close = close;
            cv += u64::from(v);
            cl += u64::from(v) * u64::try_from(close).unwrap_or(0);
        }
        if let Some(x) = c.cumvol.get_mut(Cube::px(ci, NM)) {
            *x = cv;
        }
        if let Some(x) = c.cumval.get_mut(Cube::px(ci, NM)) {
            *x = cl;
        }
        if let Some(k) = c.cons.get_mut(ci) {
            k.n_sec = nsec;
        }
        t.off.push(t.rows.len() as u64);
    }
    (c, t)
}

/// `spikefade synth <main out dir> <extra out dir> --seed N <days...>`: writes each day's synthetic cube, seconds tape and manifest in both
/// roots (never over an existing file) and prints every file's sha256 (the canary compares them across machines).
///
/// # Errors
/// A bad argument or an existing file.
pub fn cmd_synth(a: &[String]) -> Result<String, String> {
    let (Some(mo), Some(xo)) = (a.get(2), a.get(3)) else { return Err("synth <main out dir> <extra out dir> --seed N <days...>".to_owned()) };
    let seed: u64 = a.iter().position(|x| x == "--seed").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).ok_or("--seed N")?;
    let days: Vec<&String> = a.iter().skip(4).filter(|x| x.len() == 10 && x.starts_with("20")).collect();
    let mut out = String::new();
    let mut all = crate::kit::Sha256::new();
    for (root, extra) in [(Path::new(mo), false), (Path::new(xo), true)] {
        std::fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
        for d in &days {
            let di = crate::cal::parse_day(d).ok_or_else(|| format!("day {d}"))?;
            let (c, t) = day(seed, di, extra);
            let files = [(root.join(format!("{d}.bin")), crate::cube::encode_with(&c, None)), (root.join(format!("{d}.sec.bin")), crate::cube::encode_sec_with(&t, c.ymd, None)), (root.join(format!("{d}.manifest.tsv")), format!("builder\tSYNTHETIC spikefade synth v1 (E12; seed {seed}; no market data)\nday\t{d}\ncontracts\t{}\nrows_1s\t{}\n", c.cons.len(), t.rows.len()).into_bytes())];
            for (p, b) in &files {
                if p.exists() {
                    return Err(format!("{}: exists (never overwritten): refused", p.display()));
                }
                std::fs::write(p, b).map_err(|e| format!("{}: {e}", p.display()))?;
                let h = crate::kit::sha256_bytes(b);
                all.update(h.as_bytes());
                let _ = writeln!(out, "{}\t{}\t{h}", p.display(), b.len());
            }
        }
    }
    let _ = write!(out, "SYNTHETIC SET sha256 (over every file's sha256 in order) {}", all.hex());
    Ok(out)
}
