//! The outer dials' kept sets as bitsets over the candidate rows (g7's sparse-bitset AND, `o5s_search.rs:401-417`; plan 8.3:
//! "the kept set S of TT-learning rows ... outer dial bitsets AND-ed").

/// A fixed-length bitset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bits {
    w: Vec<u64>,
    n: usize,
}

impl Bits {
    /// `n` bits, all clear.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self { w: vec![0; n.div_ceil(64)], n }
    }

    /// `n` bits, all set.
    #[must_use]
    pub fn full(n: usize) -> Self {
        let mut b = Self::new(n);
        for i in 0..n {
            b.set(i);
        }
        b
    }

    /// The length in bits.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.n
    }

    /// No bits at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Sets bit `i` (an index past the end is ignored).
    pub fn set(&mut self, i: usize) {
        if i < self.n
            && let Some(x) = self.w.get_mut(i / 64)
        {
            *x |= 1_u64 << (i % 64);
        }
    }

    /// Bit `i`.
    #[must_use]
    pub fn get(&self, i: usize) -> bool {
        i < self.n && self.w.get(i / 64).is_some_and(|x| (x >> (i % 64)) & 1 == 1)
    }

    /// `self AND other` into `self` (lengths must agree; a shorter `other` clears the rest).
    pub fn and_with(&mut self, other: &Self) {
        for (i, x) in self.w.iter_mut().enumerate() {
            *x &= other.w.get(i).copied().unwrap_or(0);
        }
    }

    /// The set bits, ascending.
    pub fn ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.w.iter().enumerate().flat_map(|(wi, &word)| {
            let mut x = word;
            std::iter::from_fn(move || {
                if x == 0 {
                    None
                } else {
                    let t = x.trailing_zeros() as usize;
                    x &= x - 1;
                    Some(wi * 64 + t)
                }
            })
        })
    }

    /// The number of set bits.
    #[must_use]
    pub fn count(&self) -> usize {
        self.w.iter().map(|x| x.count_ones() as usize).sum()
    }
}
