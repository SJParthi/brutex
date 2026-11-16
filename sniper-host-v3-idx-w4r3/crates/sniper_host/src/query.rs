//! `sniper-q` (plan 13): every lookup reads a fixed-width offset index written with its partition, O(1) a lookup: a partition of
//! fixed-width records is addressed by record number (offset = number x width); the (session, candle) index is a dense table of
//! u32 record numbers (sessions x candles), `u32::MAX` for none.

/// No record.
pub const NONE: u32 = u32::MAX;

/// Builds the dense (session, candle) -> record index.
#[must_use]
pub fn build_index(keys: &[(u32, u16)], sessions: usize, kmax: u16) -> Vec<u8> {
    let w = usize::from(kmax) + 1;
    let mut ix = vec![NONE; sessions * w];
    for (rec, &(s, k)) in keys.iter().enumerate() {
        if let Some(x) = ix.get_mut(s as usize * w + usize::from(k)) {
            *x = u32::try_from(rec).unwrap_or(NONE);
        }
    }
    let mut b = Vec::with_capacity(ix.len() * 4 + 8);
    b.extend_from_slice(&u32::try_from(sessions).unwrap_or(0).to_le_bytes());
    b.extend_from_slice(&u32::from(kmax).to_le_bytes());
    for x in ix {
        b.extend_from_slice(&x.to_le_bytes());
    }
    b
}

/// The record number of (session, candle), O(1).
#[must_use]
pub fn lookup(index: &[u8], sess: u32, k: u16) -> Option<u32> {
    let rd = |o: usize| index.get(o..o + 4).and_then(|s| s.try_into().ok()).map(u32::from_le_bytes);
    let (n, kmax) = (rd(0)?, rd(4)?);
    if sess >= n || u32::from(k) > kmax {
        return None;
    }
    let w = kmax as usize + 1;
    rd(8 + (sess as usize * w + usize::from(k)) * 4).filter(|&x| x != NONE)
}

/// Record `rec` of a fixed-width partition, O(1).
#[must_use]
pub fn record(part: &[u8], width: usize, rec: u32) -> Option<&[u8]> {
    let o = rec as usize * width;
    part.get(o..o + width)
}
