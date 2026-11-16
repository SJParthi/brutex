//! idx_f1's library half: the C11 vs host `size.rs` cross-test (S2; MAIN-SIGNED-IDXOPT C11 line: "the ADANIENT host's interim
//! closest function must equal C11 by a cross-test and is replaced by C11 when v2.1 passes its check").
//! `host_size` is a verbatim copy (`cp -p -n`) of `scratch/sniper-host-v1-xcdb/crates/sniper_host/src/size.rs` (sha16
//! e91c905212d1ee72), compiled against crate v2.1 (its `size_units` / `money_in` are crate v2's byte for byte, G-CHG).

#[allow(missing_docs, clippy::pedantic, clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, reason = "a verbatim copy of the host file; its own tests use unwrap and indexing as the host allows them")]
pub mod host_size;

#[cfg(test)]
mod c11_cross {
    use crate::host_size::{QtyRule, size};
    use common_fill::limits::{SizeWhy, exposure, size_units_closest};

    /// A small deterministic generator (xorshift64*), so the cases are the same on every run.
    struct G(u64);
    impl G {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n.max(1)
        }
    }

    /// One cash shape: C11 with target = exposure(slot, L), free = exposure(free, L), against the host's `size(Closest, ..)`.
    /// Returns a difference text, or None.
    fn one(slot: i64, free: i64, lev: (u32, u32), refpx: i64, unit: u64) -> Option<String> {
        let h = size(QtyRule::Closest, slot, free, lev, refpx, unit);
        let target = exposure(slot, lev.0, lev.1)?;
        let fc = exposure(free, lev.0, lev.1)?;
        let c = size_units_closest(target, Some(refpx), unit, Some(fc));
        let (q, why) = match c {
            Ok(x) => x,
            Err(e) => return Some(format!("C11 error {e:?} slot {slot} free {free} lev {lev:?} ref {refpx} unit {unit}")),
        };
        let c_capped = matches!(why, SizeWhy::CashShort { .. } | SizeWhy::CashShortZero { .. });
        let c_rule = match why {
            SizeWhy::CashShort { closest } | SizeWhy::CashShortZero { closest } => closest,
            _ => q,
        };
        if q != h.qty || c_rule != h.rule_qty || c_capped != h.capped {
            return Some(format!("slot {slot} free {free} lev {lev:?} ref {refpx} unit {unit}: host (qty {}, rule {}, capped {}) C11 (qty {q}, rule {c_rule}, {})", h.qty, h.rule_qty, h.capped, why.code()));
        }
        None
    }

    /// 2,000,000 random cash shapes at the sniper's leverages (5x the headline, 1x, 4x) and slots Rs 20 L / N for N 1..=100, plus
    /// random slots; reference prices Rs 1 to Rs 50,000; free cash from 0 to above the slot's exposure. 0 differences.
    #[test]
    fn c11_equals_the_host_closest_on_cash_shapes() {
        let mut g = G(0x9E37_79B9_7F4A_7C15);
        let mut bad = Vec::new();
        let levs = [(5u32, 1u32), (1, 1), (4, 1), (3, 1), (2, 1)];
        for k in 0..2_000_000u64 {
            let n = 1 + g.below(100);
            let slot = if k % 3 == 0 { i64::try_from(1 + g.below(5_000_000_000)).unwrap_or(1) } else { 200_000_000 / i64::try_from(n).unwrap_or(1) };
            let lev = levs.get(usize::try_from(g.below(5)).unwrap_or(0)).copied().unwrap_or((5, 1));
            let refpx = i64::try_from(100 + g.below(5_000_000)).unwrap_or(100);
            let cap = i64::try_from(u64::try_from(slot).unwrap_or(0).saturating_mul(2)).unwrap_or(i64::MAX);
            let free = match g.below(4) {
                0 => slot,
                1 => i64::try_from(g.below(u64::try_from(cap).unwrap_or(1))).unwrap_or(0),
                2 => 200_000_000,
                _ => i64::try_from(g.below(1_000_000)).unwrap_or(0),
            };
            if let Some(d) = one(slot, free, lev, refpx, 1) {
                bad.push(d);
            }
        }
        assert!(bad.is_empty(), "{} differences, first: {:?}", bad.len(), bad.iter().take(5).collect::<Vec<_>>());
    }

    /// The exhaustive small grid: slot 1..=400 paisa x ref 1..=40 x leverage {1, 5} x free {slot x lev, 0..=420 step 7}.
    #[test]
    fn c11_equals_the_host_closest_on_an_exhaustive_grid() {
        let mut bad = 0usize;
        let mut first = None;
        for slot in 1..=400i64 {
            for refpx in 1..=40i64 {
                for lev in [(1u32, 1u32), (5, 1)] {
                    let mut frees: Vec<i64> = (0..=420).step_by(7).collect();
                    frees.push(slot * i64::from(lev.0));
                    for free in frees {
                        if let Some(d) = one(slot, free, lev, refpx, 1) {
                            bad += 1;
                            first.get_or_insert(d);
                        }
                    }
                }
            }
        }
        assert_eq!(bad, 0, "first: {first:?}");
    }
}
