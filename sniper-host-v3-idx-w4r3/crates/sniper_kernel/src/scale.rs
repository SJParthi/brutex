//! The 1x / 10x / 100x data of the HARD gate (SNIPER-ADANIENT-PLAN 8.4): "the TT-learning rows of every session replicated 10
//! and 100 times as synthetic candles inside the same calendar (outcomes drawn from the real ones by a seeded permutation), so
//! per-event bounds must not move while totals grow linearly".

use crate::line::Row;

/// splitmix64: the seeded stream every draw of the host uses (no float).
#[must_use]
pub const fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// `times` copies of every row inside its own session (POSITIONAL v2: its own GROUP; `Row.sess` holds the group index there): copy j of a
/// row of candle k gets candle `k * times + j` (computed in u32, B5) (the session's
/// candle order is kept, so FIRST mode reads the same first candle), and the OUTCOME source of every synthetic row is a seeded
/// permutation of the real rows of the same session. Returns (rows, outcome source index per row).
#[must_use]
pub fn replicate(rows: &[Row], times: u16, seed: u64) -> (Vec<Row>, Vec<u32>) {
    let t = times.max(1);
    let mut out = Vec::with_capacity(rows.len() * usize::from(t));
    let mut src = Vec::with_capacity(rows.len() * usize::from(t));
    // Rows of one session are contiguous in the host's row order (time order); find the runs.
    let mut start = 0_usize;
    while start < rows.len() {
        let sess = rows.get(start).map_or(0, |r| r.sess);
        let mut end = start;
        while rows.get(end).is_some_and(|r| r.sess == sess) {
            end += 1;
        }
        let n = end - start;
        let mut st = splitmix(seed ^ u64::from(sess));
        for i in start..end {
            let Some(r) = rows.get(i) else { continue };
            for j in 0..t {
                let mut c = *r;
                // POSITIONAL v2 (B5): in u32 (Row.k is u32): a 700-candle group x 100 = 70,099 at most here, far below u32::MAX.
                c.k = r.k.saturating_mul(u32::from(t)).saturating_add(u32::from(j));
                out.push(c);
                let pick = if j == 0 {
                    i
                } else {
                    st = splitmix(st);
                    start + usize::try_from(st % (n as u64)).unwrap_or(0)
                };
                src.push(u32::try_from(pick).unwrap_or(u32::MAX));
            }
        }
        start = end;
    }
    (out, src)
}
