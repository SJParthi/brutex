//! Test-only: how the `#[ignore]`d latency measurements of this crate time a
//! request and report it. D-4434.
//!
//! A measurement here asserts nothing about time. A shared host's numbers are
//! not a gate, so each one is run on purpose and its printed line, with the
//! load average beside it, is what `docs/06-limits.md` records:
//!
//! ```text
//! cargo test -p api --lib -- --ignored --nocapture latency
//! ```

use std::time::Instant;

/// `n` timings of one call, sorted, in nanoseconds.
pub(crate) struct Timed {
    ns: Vec<u128>,
}

impl Timed {
    /// Times `n` calls of `call`, refusing the first one that fails.
    pub(crate) fn run(
        n: usize,
        mut call: impl FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut ns = Vec::with_capacity(n);
        for _ in 0..n {
            let start = Instant::now();
            call()?;
            ns.push(start.elapsed().as_nanos());
        }
        ns.sort_unstable();
        if ns.is_empty() {
            return Err("a measurement of no calls".to_owned());
        }
        Ok(Self { ns })
    }

    /// The `per_mille` quantile, nearest rank.
    pub(crate) fn at(&self, per_mille: usize) -> u128 {
        let last = self.ns.len().saturating_sub(1);
        self.ns
            .get((self.ns.len() * per_mille / 1_000).min(last))
            .copied()
            .unwrap_or_default()
    }

    /// The slowest call.
    pub(crate) fn max(&self) -> u128 {
        self.ns.last().copied().unwrap_or_default()
    }

    /// One printable line: what, p50, p99, max, n and the load average.
    pub(crate) fn line(&self, what: &str) -> String {
        format!(
            "{what}: p50 {} ns  p99 {} ns  max {} ns  n {}  load {}",
            self.at(500),
            self.at(990),
            self.max(),
            self.ns.len(),
            load()
        )
    }
}

/// The 1-, 5- and 15-minute load averages, or why there are none.
fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").map_or_else(
        |why| format!("unread ({why})"),
        |text| {
            text.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::Timed;

    #[test]
    fn quantiles_are_nearest_rank_over_the_sorted_timings() -> Result<(), String> {
        let timed = Timed {
            ns: (1..=1_000).collect(),
        };
        assert_eq!(timed.at(500), 501);
        assert_eq!(timed.at(990), 991);
        assert_eq!(timed.at(1_000), 1_000, "clamped to the last");
        assert_eq!(timed.max(), 1_000);
        assert!(
            timed
                .line("x")
                .starts_with("x: p50 501 ns  p99 991 ns  max 1000 ns  n 1000")
        );
        assert!(Timed::run(0, || Ok(())).is_err(), "no calls is refused");
        assert!(Timed::run(3, || Err("no".to_owned())).is_err());
        assert_eq!(Timed::run(3, || Ok(()))?.ns.len(), 3);
        Ok(())
    }
}
