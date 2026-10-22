//! The universe one day's pull plans over (R4-26, COR-08 kept part; FIB-15
//! RQ-13): a PURE function of what its caller read.
//!
//! # The rule
//!
//! `day_universe(day, members_on_day, frozen_surface, series_with_own_past)`
//! is the union of
//!
//! - the F&O members on `day`, as PIT-1's dated membership answers them (the
//!   caller reads PIT-1; this module builds no membership of its own and
//!   needs no build edge to it),
//! - the caller's frozen surface (the names a sweep is pinned to), and
//! - every series the store already holds a past for,
//!
//! each name once, each with the reasons it is there. It NAMES the entrants
//! (a member on `day` the store holds no past of: its series starts here) and
//! the leavers (a series with a past that is neither a member on `day` nor on
//! the frozen surface). A leaver stays in the union: its own past keeps
//! accruing, so a share that left F&O is still pulled (FIB-15 §4.9), and
//! nothing about it is silently dropped.
//!
//! # Who calls it
//!
//! Nobody registers it with a scheduler (the one-command cut, 20 Oct 2026):
//! whoever plans a day's pull calls it — Claude Code, the operator, or
//! FIB-15's forward lane.
//!
//! # Cost
//!
//! O(members + frozen + series) per day: one hash insert per name given.
//! The answer keeps the order given (members, then the frozen surface, then
//! the series with a past), so it is deterministic for deterministic input
//! and costs no sort. Named for `docs/06-limits.md`.

use std::collections::HashMap;

use crate::session::Day;

/// Why a name is in a day's universe. A name may hold several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Why {
    /// A member of F&O on the day.
    pub member: bool,
    /// On the caller's frozen surface.
    pub frozen: bool,
    /// The store holds a past of its series.
    pub own_past: bool,
}

/// One day's universe: every name, in the order given, with its reasons;
/// and the entrants and leavers, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayUniverse {
    /// The day.
    pub day: Day,
    /// Every name once, with why it is there.
    pub series: Vec<(String, Why)>,
    /// Members on the day the store holds no past of.
    pub entrants: Vec<String>,
    /// Series with a past that are neither members on the day nor frozen:
    /// kept, so their own past keeps accruing.
    pub leavers: Vec<String>,
}

impl DayUniverse {
    /// The names alone, in order.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.series.iter().map(|(name, _)| name.as_str()).collect()
    }
}

/// The union of `members_on_day`, `frozen_surface` and
/// `series_with_own_past`, each name once, with its entrants and leavers
/// named. O(inputs).
#[must_use]
pub fn day_universe<M, F, P>(
    day: Day,
    members_on_day: M,
    frozen_surface: F,
    series_with_own_past: P,
) -> DayUniverse
where
    M: IntoIterator,
    M::Item: AsRef<str>,
    F: IntoIterator,
    F::Item: AsRef<str>,
    P: IntoIterator,
    P::Item: AsRef<str>,
{
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut series: Vec<(String, Why)> = Vec::new();
    let mut mark = |name: &str, set: fn(&mut Why)| {
        let slot = *at.entry(name.to_owned()).or_insert_with(|| {
            series.push((name.to_owned(), Why::default()));
            series.len().saturating_sub(1)
        });
        if let Some((_, why)) = series.get_mut(slot) {
            set(why);
        }
    };
    for name in members_on_day {
        mark(name.as_ref(), |why| why.member = true);
    }
    for name in frozen_surface {
        mark(name.as_ref(), |why| why.frozen = true);
    }
    for name in series_with_own_past {
        mark(name.as_ref(), |why| why.own_past = true);
    }
    let entrants = series
        .iter()
        .filter(|(_, why)| why.member && !why.own_past)
        .map(|(name, _)| name.clone())
        .collect();
    let leavers = series
        .iter()
        .filter(|(_, why)| why.own_past && !why.member && !why.frozen)
        .map(|(name, _)| name.clone())
        .collect();
    DayUniverse {
        day,
        series,
        entrants,
        leavers,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real day")
    }

    // Every name below is INVENTED (`ENTRANTCO`, `LEAVERCO`, ...).
    #[test]
    fn the_daily_universe_is_the_members_on_that_date() {
        let on = day(2023, 7, 3);
        let got = day_universe(
            on,
            ["STAYCO", "ENTRANTCO"],
            ["FROZENCO", "STAYCO"],
            ["STAYCO", "LEAVERCO", "FROZENCO"],
        );
        assert_eq!(got.day, on);
        // THE WHOLE UNION, in the order given, each name once.
        assert_eq!(
            got.names(),
            ["STAYCO", "ENTRANTCO", "FROZENCO", "LEAVERCO"]
        );
        // The entrant is named: a member the store holds no past of.
        assert_eq!(got.entrants, ["ENTRANTCO"]);
        // The leaver is named AND kept: its own past keeps accruing.
        assert_eq!(got.leavers, ["LEAVERCO"]);
        let why = |name: &str| got.series.iter().find(|(n, _)| n == name).unwrap().1;
        assert_eq!(
            why("LEAVERCO"),
            Why {
                member: false,
                frozen: false,
                own_past: true
            }
        );
        assert_eq!(
            why("STAYCO"),
            Why {
                member: true,
                frozen: true,
                own_past: true
            }
        );
        // A frozen name with a past is not a leaver: the surface holds it.
        assert!(!got.leavers.contains(&"FROZENCO".to_owned()));
    }

    #[test]
    fn an_empty_day_is_an_empty_universe_and_duplicates_count_once() {
        let empty = day_universe(day(2020, 3, 23), [""; 0], [""; 0], [""; 0]);
        assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
        let twice = day_universe(day(2020, 3, 23), ["ACO", "ACO"], ["ACO"], ["ACO", "ACO"]);
        assert_eq!(twice.names(), ["ACO"]);
        assert!(twice.entrants.is_empty() && twice.leavers.is_empty());
    }

    #[test]
    fn a_name_that_left_and_came_back_is_a_member_again_not_a_leaver() {
        // The caller reads PIT-1's two spans; on a day inside the second the
        // name is a member with a past: neither entrant nor leaver.
        let got = day_universe(day(2024, 1, 2), ["BACKCO"], [""; 0], ["BACKCO"]);
        assert_eq!(got.names(), ["BACKCO"]);
        assert!(got.entrants.is_empty() && got.leavers.is_empty());
    }

    #[test]
    fn the_universe_costs_one_insert_per_name_measured() {
        // 1x, 10x and 100x names: the per-name cost stays flat (O(inputs)).
        let names = |n: usize, tag: &str| -> Vec<String> {
            (0..n).map(|i| format!("{tag}{i:06}")).collect()
        };
        let mut per_name = Vec::new();
        for scale in [200_usize, 2_000, 20_000] {
            let members = names(scale, "M");
            let past = names(scale, "P");
            let mut samples = Vec::new();
            for _ in 0..21 {
                let started = std::time::Instant::now();
                let got = day_universe(day(2025, 1, 1), &members, [""; 0], &past);
                assert_eq!(got.series.len(), scale * 2);
                samples.push(started.elapsed().as_nanos() / (u128::try_from(scale).unwrap() * 2));
            }
            samples.sort_unstable();
            per_name.push(samples[10]);
        }
        // The median nanoseconds per name at 100x stays within 8x of 1x:
        // linear, not quadratic (a quadratic walk would be 100x).
        assert!(
            per_name[2] <= per_name[0].max(1) * 8,
            "ns per name at 1x, 10x, 100x: {per_name:?}"
        );
    }
}
