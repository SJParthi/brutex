//! CRC-32 (IEEE 802.3, the zlib polynomial 0xEDB88320, reflected), the tick store's checksum (FORMAT.md section 3).
//! Checked against the published check value of `b"123456789"` (0xCBF43926).

#[allow(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    reason = "a const table build: i < 256, so the cast is exact and the index is in bounds"
)]
const fn table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

static TABLE: [u32; 256] = table();

/// CRC-32 of `bytes`.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        let i = usize::from(u8::try_from((c ^ u32::from(b)) & 0xFF).unwrap_or(0));
        c = TABLE.get(i).copied().unwrap_or(0) ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_value() {
        assert_eq!(super::crc32(b"123456789"), 0xCBF4_3926);
    }
}
