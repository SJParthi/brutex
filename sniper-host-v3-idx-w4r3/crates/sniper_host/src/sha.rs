//! SHA-256 (FIPS 180-4), std only: the audit chain and the identity lines.

const K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01,
    0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc,
    0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147,
    0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85,
    0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116, 0x1e37_6c08,
    0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208,
    0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

/// The SHA-256 of `data`.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (i, wi) in w.iter_mut().take(16).enumerate() {
            let b = chunk.get(i * 4..i * 4 + 4).unwrap_or(&[0, 0, 0, 0]);
            *wi = u32::from_be_bytes([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0), b.get(2).copied().unwrap_or(0), b.get(3).copied().unwrap_or(0)]);
        }
        for i in 16..64 {
            let g = |j: usize| w.get(j).copied().unwrap_or(0);
            let s0 = g(i - 15).rotate_right(7) ^ g(i - 15).rotate_right(18) ^ (g(i - 15) >> 3);
            let s1 = g(i - 2).rotate_right(17) ^ g(i - 2).rotate_right(19) ^ (g(i - 2) >> 10);
            let v = g(i - 16).wrapping_add(s0).wrapping_add(g(i - 7)).wrapping_add(s1);
            if let Some(x) = w.get_mut(i) {
                *x = v;
            }
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K.get(i).copied().unwrap_or(0)).wrapping_add(w.get(i).copied().unwrap_or(0));
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0_u8; 32];
    for (i, v) in h.iter().enumerate() {
        if let Some(s) = out.get_mut(i * 4..i * 4 + 4) {
            s.copy_from_slice(&v.to_be_bytes());
        }
    }
    out
}

/// The STREAMING SHA-256 (SECONDS BUILDER, 2026-11-14): the seconds families' outcome stores run to gigabytes, so they are hashed as
/// they are written and verified by streaming reads, never held whole ([`sha256`] copies its input). Test `streaming_equals_one_shot`
/// holds it to [`sha256`] on every length 0..300 and random splits.
#[derive(Clone, Debug)]
pub struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

fn block(h: &mut [u32; 8], chunk: &[u8]) {
    let mut w = [0_u32; 64];
    for (i, wi) in w.iter_mut().take(16).enumerate() {
        let b = chunk.get(i * 4..i * 4 + 4).unwrap_or(&[0, 0, 0, 0]);
        *wi = u32::from_be_bytes([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0), b.get(2).copied().unwrap_or(0), b.get(3).copied().unwrap_or(0)]);
    }
    for i in 16..64 {
        let g = |j: usize| w.get(j).copied().unwrap_or(0);
        let s0 = g(i - 15).rotate_right(7) ^ g(i - 15).rotate_right(18) ^ (g(i - 15) >> 3);
        let s1 = g(i - 2).rotate_right(17) ^ g(i - 2).rotate_right(19) ^ (g(i - 2) >> 10);
        let v = g(i - 16).wrapping_add(s0).wrapping_add(g(i - 7)).wrapping_add(s1);
        if let Some(x) = w.get_mut(i) {
            *x = v;
        }
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K.get(i).copied().unwrap_or(0)).wrapping_add(w.get(i).copied().unwrap_or(0));
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *x = x.wrapping_add(y);
    }
}

impl Sha256 {
    /// A fresh hasher.
    #[must_use]
    pub fn new() -> Self {
        Self { h: [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19], buf: Vec::with_capacity(64), len: 0 }
    }

    /// Feeds bytes.
    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if !self.buf.is_empty() {
            let need = 64 - self.buf.len();
            let (a, b) = data.split_at(need.min(data.len()));
            self.buf.extend_from_slice(a);
            data = b;
            if self.buf.len() == 64 {
                let blk = std::mem::take(&mut self.buf);
                block(&mut self.h, &blk);
                self.buf = blk;
                self.buf.clear();
            }
        }
        let mut it = data.chunks_exact(64);
        for c in &mut it {
            block(&mut self.h, c);
        }
        self.buf.extend_from_slice(it.remainder());
    }

    /// The digest.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        let mut tail = std::mem::take(&mut self.buf);
        tail.push(0x80);
        while tail.len() % 64 != 56 {
            tail.push(0);
        }
        tail.extend_from_slice(&bits.to_be_bytes());
        for c in tail.chunks_exact(64) {
            block(&mut self.h, c);
        }
        let mut out = [0_u8; 32];
        for (i, v) in self.h.iter().enumerate() {
            if let Some(s) = out.get_mut(i * 4..i * 4 + 4) {
                s.copy_from_slice(&v.to_be_bytes());
            }
        }
        out
    }
}

/// The SHA-256 of a file, read in 8 MiB pieces.
///
/// # Errors
/// An I/O error.
pub fn sha256_file(path: &std::path::Path) -> Result<[u8; 32], String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0_u8; 8 << 20];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(buf.get(..n).unwrap_or(&[]));
    }
    Ok(h.finish())
}

/// Lower-case hex.
#[must_use]
pub fn hex(b: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn known_vectors() {
        assert_eq!(super::hex(&super::sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(super::hex(&super::sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn streaming_equals_one_shot() {
        let data: Vec<u8> = (0..300_u32).map(|i| u8::try_from((i * 131 + 7) % 251).unwrap()).collect();
        for n in 0..data.len() {
            let d = &data[..n];
            for split in [0, 1, n / 3, n / 2, n.saturating_sub(1), n] {
                let mut h = super::Sha256::new();
                h.update(&d[..split.min(n)]);
                h.update(&d[split.min(n)..]);
                assert_eq!(h.finish(), super::sha256(d), "length {n} split {split}");
            }
            let mut h = super::Sha256::new();
            for b in d.chunks(7) {
                h.update(b);
            }
            assert_eq!(h.finish(), super::sha256(d), "length {n} in 7-byte pieces");
        }
    }
}
