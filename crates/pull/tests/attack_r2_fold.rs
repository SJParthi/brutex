//! ATTACK ROUND 2, FOLD: the round-1 refusals (`Bucket::of_secs` on a width
//! that does not divide a day, `fold_from_bars`' repeated / off-grid /
//! misaligned-day refusals) against every caller-shaped input: every width,
//! every pair of store rungs, random holed sessions.
//!
//! splitmix64, fixed seed, so reruns are byte-identical.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use pull::fold::{Bucket, FoldError, fold, fold_from_bars};
use pull::session::Day;
use store::format::Bar;
use store::path::Timeframe;

struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// **EVERY WIDTH IS ACCEPTED EXACTLY WHEN IT DIVIDES A DAY**, and every store
/// rung a caller derives a bucket from (`ingest::fold_in_place`,
/// `ingest::derive`, `cli::fold_audit::store_bucket`) still gets one.
#[test]
fn of_secs_accepts_exactly_the_divisors_of_a_day_and_every_store_rung() {
    let mut accepted = 0u32;
    for secs in 0..=200_000u32 {
        let want = secs != 0 && secs <= 86_400 && 86_400 % secs == 0;
        assert_eq!(Bucket::of_secs(secs).is_some(), want, "{secs}");
        if want {
            accepted += 1;
            assert_eq!(Bucket::of_secs(secs).unwrap().secs(), secs);
        }
    }
    // 86,400 = 2^7 * 3^3 * 5^2 has (7+1)(3+1)(2+1) = 96 divisors.
    assert_eq!(accepted, 96);
    for secs in [u32::MAX, 86_401, 172_800] {
        assert!(Bucket::of_secs(secs).is_none(), "{secs}");
    }
    for &tf in Timeframe::KNOWN {
        assert!(
            Bucket::of_secs(tf.secs()).is_some(),
            "store rung {} lost its bucket",
            tf.as_str()
        );
    }
}

/// Minutes of random sessions, with random holes, on the 09:15 grid.
fn minutes(mix: &mut Mix, days: &[Day]) -> Vec<Bar> {
    let mut out = Vec::new();
    let mut level: i64 = 2_400_000;
    for &d in days {
        let open = i64::from(d.days_from_epoch()) * 86_400 - 19_800 + 9 * 3_600 + 15 * 60;
        for m in 0..375 {
            if mix.below(20) == 0 {
                continue;
            }
            level += mix.below(2_001) as i64 - 1_000;
            let o = level;
            let c = level + mix.below(401) as i64 - 200;
            out.push(Bar {
                ts_micros: (open + m * 60) * 1_000_000,
                open: o,
                high: o.max(c) + mix.below(100) as i64,
                low: o.min(c) - mix.below(100) as i64,
                close: c,
                volume: mix.below(10_000) as i64,
                open_interest: store::format::OI_NULL,
            });
        }
    }
    out
}

/// **FOLDING THROUGH ANY LEGAL INTERMEDIATE RUNG IS FOLDING DIRECTLY.** For
/// every ordered pair of store rungs, on random holed sessions: a target that
/// is a whole multiple of the source composes to exactly the bars a direct
/// fold of the minutes gives; a target that is not is `NarrowerThanSource`;
/// a day from a source whose grid misses IST midnight is `GridMisaligned` —
/// and nothing else is ever refused.
#[test]
fn folding_through_any_legal_intermediate_rung_is_folding_directly() {
    let mut mix = Mix(0x3180_F01D_0000_0001);
    let days = [
        Day::new(2025, 7, 1).unwrap(),
        Day::new(2025, 7, 2).unwrap(),
        Day::new(2025, 7, 31).unwrap(),
        Day::new(2025, 8, 1).unwrap(),
    ];
    let rungs: Vec<Bucket> = Timeframe::KNOWN
        .iter()
        .filter(|tf| tf.secs() >= 60)
        .map(|tf| Bucket::of_secs(tf.secs()).unwrap())
        .collect();
    let mut composed = 0usize;
    let mut misaligned = 0usize;
    for _ in 0..20 {
        let raw = minutes(&mut mix, &days);
        for &source in &rungs {
            let mid = fold(&raw, source).expect("minutes fold to any rung");
            for &target in &rungs {
                let got = fold_from_bars(&mid, target, source);
                if target.secs() % source.secs() != 0 {
                    assert!(
                        matches!(got, Err(FoldError::NarrowerThanSource { .. })),
                        "{} from {}: {got:?}",
                        target.secs(),
                        source.secs()
                    );
                    continue;
                }
                if target.secs() == 86_400 && source.secs() < 86_400 && 33_300 % source.secs() != 0
                {
                    assert!(
                        matches!(got, Err(FoldError::GridMisaligned { .. })),
                        "{} from {}: {got:?}",
                        target.secs(),
                        source.secs()
                    );
                    misaligned += 1;
                    continue;
                }
                let direct = fold(&raw, target).unwrap();
                assert_eq!(
                    got.unwrap_or_else(|why| panic!(
                        "{} from {}: {why}",
                        target.secs(),
                        source.secs()
                    )),
                    direct,
                    "{} from {}",
                    target.secs(),
                    source.secs()
                );
                composed += 1;
            }
        }
    }
    assert!(composed > 400, "{composed}");
    assert_eq!(misaligned, 20 * 4, "2, 10, 30 and 60 minutes miss midnight");
}

/// **A BAR SHIFTED OFF ITS GRID, OR REPEATED, IS REFUSED BY POSITION.** Every
/// single bar of a folded series moved by every offset in a small set, and
/// every single bar duplicated in place.
#[test]
fn every_single_off_grid_or_repeated_bar_is_refused_at_its_position() {
    let mut mix = Mix(0x3180_F01D_0000_0002);
    let raw = minutes(&mut mix, &[Day::new(2025, 7, 1).unwrap()]);
    let five = Bucket::of_secs(300).unwrap();
    let fifteen = Bucket::of_secs(900).unwrap();
    let bars = fold(&raw, five).unwrap();
    let mut tried = 0usize;
    for at in 0..bars.len() {
        for shift in [1_i64, 1_000_000, 60_000_000, 299_999_999] {
            let mut moved = bars.clone();
            moved[at].ts_micros += shift;
            // keep the order strict so only the grid can object
            if at + 1 < moved.len() && moved[at].ts_micros >= moved[at + 1].ts_micros {
                continue;
            }
            tried += 1;
            let got = fold_from_bars(&moved, fifteen, five);
            assert!(
                matches!(got, Err(FoldError::OffSourceGrid { at: a, .. }) if a == at),
                "bar {at} +{shift}: {got:?}"
            );
        }
        let mut twice = bars.clone();
        twice.insert(at + 1, bars[at]);
        tried += 1;
        assert!(
            matches!(
                fold_from_bars(&twice, fifteen, five),
                Err(FoldError::RepeatedBar { at: a, .. }) if a == at + 1
            ),
            "bar {at} repeated"
        );
    }
    assert!(tried > 300, "{tried}");
}
