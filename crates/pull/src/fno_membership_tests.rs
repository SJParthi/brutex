//! `fno_membership`: the day rule, the refusals, the file, the reader and the
//! resumed build. D-5160 to D-5165 (PIT-1).
//!
//! Every fixture is invented: the share names (`ZZ...`) are not NSE symbols
//! and their strikes are arbitrary. The real symbols used, `ZOMATO` and
//! `ETERNAL`, are there only because the sourced rename row naming them
//! (`brutex_core::alias::SYMBOL_CHANGES`, NSE/CML/67420) and the 2026-08-01
//! snapshot holding `ETERNAL` are what is under test; their strike (99,999.5)
//! is not a price either ever traded at, and no tick row is read.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{
    Built, DAY_ROW_LEN, DIGEST_LEN, DayInput, DayStatus, ENTRY_LEN, HEADER_LEN, Installed,
    Membership, MembershipError, MonthBuilt, MonthTable, SPELLING_LEN, State, UnknownWhy, Window,
    build, file_name, generations, install, parse_file_name, read_day,
};
use crate::calendar::{DayKind, sourced_kind_of};
use crate::gdfl_cm::{CmRefusal, ListedFile};
use crate::gdfl_nfo::{NfoDay, NfoSource, day_folder_name};
use crate::session::Day;

fn d(year: u16, month: u8, day: u8) -> Day {
    Day::new(year, month, day).unwrap()
}

/// An invented options source: the tickers of each listed day, and the days
/// it refuses.
#[derive(Default, Clone)]
struct Fake {
    days: HashMap<Day, Vec<String>>,
    refused: Vec<Day>,
}

impl Fake {
    fn list(mut self, day: Day, tickers: &[&str]) -> Self {
        self.days
            .insert(day, tickers.iter().map(|t| (*t).to_owned()).collect());
        self
    }

    fn refuse(mut self, day: Day) -> Self {
        self.refused.push(day);
        self
    }
}

impl NfoSource for Fake {
    type Locator = ();

    fn day(&self, day: Day) -> Result<Option<NfoDay<()>>, CmRefusal> {
        if self.refused.contains(&day) {
            return Err(CmRefusal::FolderUnknown);
        }
        Ok(self.days.get(&day).map(|tickers| {
            let folder = day_folder_name(day);
            let mut listing = NfoDay::new(day, tickers.len() + 1);
            // A non-option entry is skipped by the listing, never a share.
            listing.push(&format!("{folder}/readme.txt"), 1, 0, ());
            for ticker in tickers {
                listing.push(&format!("{folder}/Options/{ticker}.NFO.csv"), 1, 0, ());
            }
            listing
        }))
    }

    fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
        Err(CmRefusal::FolderUnknown)
    }
}

/// A fresh scratch directory under the system's temporary folder: never the
/// store.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-fno-membership-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Builds `dir` from `source` over a typed window; returns what the build
/// said and every month it reported.
fn build_into(
    source: &Fake,
    dir: &Path,
    from: Option<Day>,
    to: Day,
) -> Result<(Built, Vec<MonthBuilt>), MembershipError> {
    let mut months = Vec::new();
    let built = build(source, dir, Window::Typed { from, to }, 3, &mut |month| {
        months.push(month.clone());
    })?;
    Ok((built, months))
}

/// Builds March 2024 of `source` into a new directory and opens it.
fn march(source: &Fake, name: &str) -> (Membership, Vec<MonthBuilt>, PathBuf) {
    let dir = scratch(name);
    let (_, months) = build_into(source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 31)).unwrap();
    (Membership::open(&dir).unwrap(), months, dir)
}

/// The names of `day`'s members, in series order.
fn names(held: &Membership, day: Day) -> Vec<String> {
    held.members(day)
        .unwrap()
        .iter()
        .map(|member| member.symbol_on_day.to_string())
        .collect()
}

/// Every month file of `dir`: name and bytes, in name order.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// The newest generation of every month of `dir`: month and bytes.
fn newest(dir: &Path) -> Vec<((u16, u8), Vec<u8>)> {
    generations(dir)
        .unwrap()
        .0
        .into_iter()
        .map(|(month, (_, path))| (month, std::fs::read(path).unwrap()))
        .collect()
}

#[test]
fn a_contract_listed_but_untraded_on_d_still_counts() {
    // ZZALPHA's March contract is first listed on Mon 4 Mar 2024 and lists no
    // file on Tue 5 Mar: its life covers the 5th, so it is a member, unfiled.
    let source = Fake::default()
        .list(
            d(2024, 3, 4),
            &["ZZALPHA28MAR24500CE", "ZZBETA28MAR24700PE"],
        )
        .list(d(2024, 3, 5), &["ZZBETA28MAR24700PE"]);
    let (held, _, _) = march(&source, "untraded");
    assert_eq!(held.is_member("ZZALPHA", d(2024, 3, 5)), Ok(true));
    assert_eq!(held.pool_size(d(2024, 3, 5)), Ok(2));
    let alpha = |day| {
        *held
            .members(day)
            .unwrap()
            .iter()
            .find(|member| member.symbol_on_day.as_str() == "ZZALPHA")
            .unwrap()
    };
    assert!(alpha(d(2024, 3, 4)).filed, "listed a file on the 4th");
    assert!(
        !alpha(d(2024, 3, 5)).filed,
        "a member with no file on the 5th"
    );
    assert_eq!(held.row(d(2024, 3, 5)).unwrap().filed, 1);
}

#[test]
fn an_expired_contract_does_not_count() {
    // The only ZZALPHA contract settles Wed 6 Mar 2024: a member on the 6th,
    // not on Thu the 7th, although ZZBETA keeps the day known.
    let source = Fake::default()
        .list(
            d(2024, 3, 4),
            &["ZZALPHA06MAR24500CE", "ZZBETA28MAR24700PE"],
        )
        .list(d(2024, 3, 6), &["ZZBETA28MAR24700PE"])
        .list(d(2024, 3, 7), &["ZZBETA28MAR24700PE"]);
    let (held, _, _) = march(&source, "expired");
    assert_eq!(held.is_member("ZZALPHA", d(2024, 3, 6)), Ok(true));
    assert_eq!(held.is_member("ZZALPHA", d(2024, 3, 7)), Ok(false));
    assert_eq!(names(&held, d(2024, 3, 7)), ["ZZBETA"]);
    assert_eq!(held.pool_size(d(2024, 3, 7)), Ok(1));
}

#[test]
fn a_sourced_rename_is_one_member_under_either_name() {
    // NSE/CML/67420: ZOMATO became ETERNAL on 9 Apr 2025. The vendor's stems
    // change that day; the decoder files both under ETERNAL, the store key
    // of the cash series on both days.
    let source = Fake::default()
        .list(d(2025, 4, 8), &["ZOMATO24APR2599999.5CE"])
        .list(d(2025, 4, 9), &["ETERNAL24APR2599999.5CE"]);
    let dir = scratch("sourced-rename");
    build_into(&source, &dir, Some(d(2025, 4, 1)), d(2025, 4, 30)).unwrap();
    let held = Membership::open(&dir).unwrap();
    for day in [d(2025, 4, 8), d(2025, 4, 9)] {
        assert_eq!(held.pool_size(day), Ok(1), "{day}");
        assert_eq!(held.is_member("ZOMATO", day), Ok(true), "{day}");
        assert_eq!(held.is_member("ETERNAL", day), Ok(true), "{day}");
        assert_eq!(names(&held, day), ["ETERNAL"], "{day}");
    }
    assert_eq!(held.series_count(), 1);
    assert!(
        held.handovers().is_empty(),
        "a sourced rename is no handover"
    );
}

/// Three contract keys under `share`.
fn keys(share: &str) -> Vec<String> {
    ["28MAR24500CE", "28MAR24520CE", "28MAR24480PE"]
        .iter()
        .map(|key| format!("{share}{key}"))
        .collect()
}

fn refs(names: &[String]) -> Vec<&str> {
    names.iter().map(String::as_str).collect()
}

#[test]
fn an_unsourced_rename_is_a_measured_handover_and_one_series() {
    // ZZOLDCO's three keys move to ZZNEWCO on Wed 6 Mar 2024 and ZZOLDCO lists
    // no file that day: one series, filed under ZZOLDCO to the 5th and under
    // ZZNEWCO from the 6th, never two members on one day.
    let (old, new) = (keys("ZZOLDCO"), keys("ZZNEWCO"));
    let source = Fake::default()
        .list(d(2024, 3, 4), &refs(&old))
        .list(d(2024, 3, 5), &refs(&old))
        .list(d(2024, 3, 6), &refs(&new))
        .list(d(2024, 3, 7), &refs(&new));
    let (held, months, _) = march(&source, "handover");
    assert_eq!(names(&held, d(2024, 3, 5)), ["ZZOLDCO"]);
    for day in [d(2024, 3, 6), d(2024, 3, 7)] {
        assert_eq!(held.pool_size(day), Ok(1), "{day}");
        assert_eq!(names(&held, day), ["ZZNEWCO"], "{day}");
        assert_eq!(held.is_member("ZZNEWCO", day), Ok(true), "{day}");
        assert_eq!(
            held.is_member("ZZOLDCO", day),
            Ok(true),
            "one series, either name"
        );
    }
    let series = |day| held.members(day).unwrap()[0].series_id;
    assert_eq!(series(d(2024, 3, 5)), series(d(2024, 3, 6)));
    assert_eq!(held.series_count(), 1);
    let [handover] = held.handovers() else {
        panic!("one handover: {:?}", held.handovers());
    };
    assert_eq!(
        (handover.day, handover.shared, handover.old_keys),
        (d(2024, 3, 6), 3, 3)
    );
    assert_eq!(
        held.spelling(handover.old).unwrap().name.as_str(),
        "ZZOLDCO"
    );
    assert_eq!(
        held.spelling(handover.new).unwrap().name.as_str(),
        "ZZNEWCO"
    );
    assert_eq!(months[0].handovers.len(), 1, "the month reports it");
}

#[test]
fn a_minority_of_shared_keys_is_no_handover() {
    // ZZNEWCO lists one of ZZOLDCO's three keys: two series, and ZZOLDCO stays
    // a member to its expiry with no file.
    let new = [
        "ZZNEWCO28MAR24500CE",
        "ZZNEWCO28MAR24900CE",
        "ZZNEWCO28MAR24950PE",
    ];
    let source = Fake::default()
        .list(d(2024, 3, 4), &refs(&keys("ZZOLDCO")))
        .list(d(2024, 3, 5), &new);
    let (held, _, _) = march(&source, "minority");
    assert_eq!(names(&held, d(2024, 3, 5)), ["ZZOLDCO", "ZZNEWCO"]);
    assert!(held.handovers().is_empty());
    assert_eq!(held.series_count(), 2);
}

#[test]
fn a_handover_matching_two_candidates_is_not_joined() {
    // Two new names each list all of ZZOLDCO's keys: ambiguous, reported, and
    // neither joins; three series.
    let source = Fake::default()
        .list(d(2024, 3, 4), &refs(&keys("ZZOLDCO")))
        .list(
            d(2024, 3, 5),
            &refs(&[keys("ZZNEWAA"), keys("ZZNEWBB")].concat()),
        );
    let (held, months, _) = march(&source, "ambiguous");
    assert!(held.handovers().is_empty());
    assert_eq!(held.series_count(), 3);
    assert_eq!(held.pool_size(d(2024, 3, 5)), Ok(3));
    assert_eq!(months[0].ambiguous.len(), 2, "{:?}", months[0].ambiguous);
    assert!(months[0].ambiguous[0].1.contains("not joined"));
}

#[test]
fn an_unknown_day_is_refused_by_name_and_never_filled() {
    // Known on 4, 7 and 11 Mar 2024; the 5th and 6th have no day file, the
    // 9th is a Saturday; 1 Mar is before the span, 12 Mar after. (A day file
    // the SOURCE refuses is never sealed at all since D-5168: its own test is
    // `a_refused_source_day_is_never_sealed_and_the_next_build_resumes_at_it`.)
    let source = Fake::default()
        .list(d(2024, 3, 4), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 7), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 11), &["ZZALPHA28MAR24500CE"]);
    let (held, months, _) = march(&source, "unknown");
    let why = |day| held.members(day).unwrap_err().why;
    assert_eq!(why(d(2024, 3, 5)), UnknownWhy::Refused(DayStatus::NoSource));
    assert_eq!(why(d(2024, 3, 6)), UnknownWhy::Refused(DayStatus::NoSource));
    assert_eq!(
        why(d(2024, 3, 9)),
        UnknownWhy::Refused(DayStatus::NotSession)
    );
    assert_eq!(
        why(d(2024, 3, 1)),
        UnknownWhy::BeforeSpan {
            known_from: d(2024, 3, 4)
        }
    );
    assert_eq!(
        why(d(2024, 3, 12)),
        UnknownWhy::AfterSpan {
            known_to: d(2024, 3, 11)
        }
    );
    // Every lookup refuses the same day, and none answers from a list.
    let refusal = held.is_member("ZZALPHA", d(2024, 3, 5)).unwrap_err();
    assert!(
        refusal
            .to_string()
            .starts_with("membership unknown: 2024-03-05"),
        "{refusal}"
    );
    assert!(held.pool_size(d(2024, 3, 6)).is_err());
    assert!(
        held.members(d(2024, 3, 1))
            .unwrap_err()
            .to_string()
            .contains("never filled")
    );
    // Each unknown session reaches the month's report with its reason.
    let six = months[0]
        .unknown
        .iter()
        .find(|(day, _, _)| *day == d(2024, 3, 6))
        .unwrap();
    assert_eq!(six.1, DayStatus::NoSource);
    // An empty directory answers nothing at all; a missing one is refused by
    // name (`a_missing_directory_is_named_and_never_served_as_empty`).
    let empty = Membership::open(&scratch("empty")).unwrap();
    assert_eq!(
        empty.members(d(2024, 3, 4)).unwrap_err().why,
        UnknownWhy::Empty
    );
}

#[test]
fn a_file_on_a_closed_day_or_with_no_share_option_is_refused_and_not_folded() {
    // Sat 9 Mar 2024 has a day file: named, refused, not folded (ZZGHOST is
    // never a member). Mon 11 Mar lists only an index: no share option.
    let source = Fake::default()
        .list(d(2024, 3, 7), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 9), &["ZZGHOST28MAR24500CE"])
        .list(d(2024, 3, 11), &["NIFTY14MAR2499999.5CE"])
        .list(d(2024, 3, 12), &["ZZALPHA28MAR24500CE"]);
    let (held, _, _) = march(&source, "closed");
    assert_eq!(
        held.row(d(2024, 3, 9)).unwrap().status,
        DayStatus::SourceOnClosedDay
    );
    assert_eq!(
        held.row(d(2024, 3, 11)).unwrap().status,
        DayStatus::NoShareOptions
    );
    assert_eq!(held.row(d(2024, 3, 11)).unwrap().index_names, 1);
    assert_eq!(held.is_member("ZZGHOST", d(2024, 3, 12)), Ok(false));
    assert_eq!(held.series_count(), 1);
}

#[test]
fn a_day_the_calendar_does_not_cover_is_not_read() {
    // The IST calendar ends 2026-12-31: a 2027 day is refused without asking
    // the source, and a window reaching it is refused before anything runs.
    let source = Fake::default().refuse(d(2027, 1, 4));
    assert_eq!(
        read_day(&source, d(2027, 1, 4)),
        DayInput::Unknown(DayStatus::NoCalendar, None)
    );
    let dir = scratch("no-calendar");
    assert!(matches!(
        build_into(&source, &dir, Some(d(2026, 12, 1)), d(2027, 1, 4)),
        Err(MembershipError::BadWindow { .. })
    ));
    assert!(files(&dir).is_empty());
}

#[test]
fn the_pool_size_is_per_day() {
    let source = Fake::default()
        .list(d(2024, 3, 4), &["ZZALPHA04MAR24500CE"])
        .list(
            d(2024, 3, 5),
            &[
                "ZZALPHA28MAR24500CE",
                "ZZBETA28MAR24700PE",
                "ZZGAMMA28MAR24900CE",
            ],
        )
        .list(d(2024, 3, 6), &["ZZBETA06MAR24700PE"]);
    let (held, _, _) = march(&source, "pool");
    assert_eq!(held.pool_size(d(2024, 3, 4)), Ok(1));
    assert_eq!(held.pool_size(d(2024, 3, 5)), Ok(3));
    assert_eq!(held.pool_size(d(2024, 3, 6)), Ok(3));
    for day in [d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6)] {
        assert_eq!(
            usize::from(held.pool_size(day).unwrap()),
            held.members(day).unwrap().len(),
            "{day}"
        );
    }
    assert_eq!(held.max_pool(), 3);
}

#[test]
fn an_index_is_never_a_share_member() {
    let source = Fake::default().list(
        d(2024, 3, 4),
        &[
            "NIFTY07MAR2499999.5CE",
            "BANKNIFTY06MAR2499999.5PE",
            "ZZALPHA28MAR24500CE",
        ],
    );
    let (held, _, _) = march(&source, "index");
    assert_eq!(names(&held, d(2024, 3, 4)), ["ZZALPHA"]);
    assert_eq!(held.row(d(2024, 3, 4)).unwrap().index_names, 2);
    assert!(matches!(
        held.is_member("NIFTY", d(2024, 3, 4)),
        Err(MembershipError::NotAShare { .. })
    ));
    assert!(matches!(
        held.is_member("not a symbol!", d(2024, 3, 4)),
        Err(MembershipError::BadName { .. })
    ));
    assert_eq!(held.is_member("ZZNEVER", d(2024, 3, 4)), Ok(false));
}

#[test]
fn members_are_in_series_order_with_their_spelling_and_the_current_list_flag() {
    // Series ids are dense in first-appearance order, so ZZBETA (first seen
    // on the 4th) sorts before ZZALPHA (the 5th), whatever the alphabet says.
    // ETERNAL is a share of the 2026-08-01 snapshot; ZZ names are not.
    let source = Fake::default()
        .list(d(2025, 4, 8), &["ZZBETA24APR25700PE"])
        .list(
            d(2025, 4, 9),
            &[
                "ZZALPHA24APR25500CE",
                "ETERNAL24APR2599999.5CE",
                "ZZBETA24APR25700PE",
            ],
        );
    let dir = scratch("order");
    build_into(&source, &dir, Some(d(2025, 4, 1)), d(2025, 4, 30)).unwrap();
    let held = Membership::open(&dir).unwrap();
    let members = held.members(d(2025, 4, 9)).unwrap();
    let ids: Vec<u32> = members.iter().map(|member| member.series_id).collect();
    assert_eq!(ids, [0, 1, 2]);
    assert_eq!(
        names(&held, d(2025, 4, 9)),
        ["ZZBETA", "ETERNAL", "ZZALPHA"]
    );
    let flags: Vec<bool> = members
        .iter()
        .map(|member| member.in_current_list)
        .collect();
    assert_eq!(flags, [false, true, false]);
    assert_eq!(held.series(1).unwrap().name.as_str(), "ETERNAL");
}

#[test]
fn the_span_runs_from_the_first_to_the_last_known_day_and_is_written_in_the_header() {
    // Sessions before the first known day and after the last are probed,
    // reported and never written; the header carries the measured start.
    let source = Fake::default()
        .list(d(2024, 3, 5), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 12), &["ZZALPHA28MAR24500CE"]);
    let dir = scratch("span");
    let (built, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 31)).unwrap();
    let held = Membership::open(&dir).unwrap();
    assert_eq!(held.known_span(), Some((d(2024, 3, 5), d(2024, 3, 12))));
    assert_eq!(built.leading.first().map(|one| one.0), Some(d(2024, 3, 1)));
    assert_eq!(built.leading.last().map(|one| one.0), Some(d(2024, 3, 4)));
    assert_eq!(
        built.trailing.first().map(|one| one.0),
        Some(d(2024, 3, 13))
    );
    assert_eq!(built.trailing.last().map(|one| one.0), Some(d(2024, 3, 31)));
    assert_eq!(built.folded, 8, "5..=12 March");
    assert_eq!(
        months[0].table.held(),
        Some((d(2024, 3, 5), d(2024, 3, 12)))
    );
    let bytes = &newest(&dir)[0].1;
    assert_eq!(
        bytes[20..24],
        d(2024, 3, 5).days_from_epoch().to_le_bytes(),
        "known_from is in the header"
    );
    assert_eq!(
        bytes[18..20],
        1_u16.to_le_bytes(),
        "max_pool is in the header"
    );
}

/// Every session of March and April 2024 lists ZZBETA's current contract
/// (a file after its expiry is refused by the decoder, so the next one rolls
/// in), ZZALPHA's to 20 Mar, and ZZOLDCO hands over to ZZNEWCO on 6 Mar.
fn two_months() -> Fake {
    let mut source = Fake::default();
    let mut day = d(2024, 3, 1);
    while day <= d(2024, 4, 30) {
        let beta = if day <= d(2024, 3, 28) {
            "ZZBETA28MAR24700PE"
        } else if day <= d(2024, 4, 25) {
            "ZZBETA25APR24700PE"
        } else {
            "ZZBETA30MAY24700PE"
        };
        let mut tickers: Vec<String> = vec![beta.to_owned()];
        if day <= d(2024, 3, 20) {
            tickers.push("ZZALPHA28MAR24500CE".to_owned());
        }
        if day <= d(2024, 3, 5) {
            tickers.extend(keys("ZZOLDCO"));
        } else if day.month() == 3 {
            tickers.extend(keys("ZZNEWCO"));
        }
        let session = matches!(
            sourced_kind_of(i64::from(day.days_from_epoch())),
            DayKind::Open(_)
        );
        if session && day != d(2024, 3, 13) {
            source.days.insert(day, tickers);
        }
        day = day.succ().unwrap();
    }
    source
}

#[test]
fn one_pass_and_day_by_day_builds_are_byte_identical() {
    let source = two_months();
    let once = scratch("one-pass");
    build_into(&source, &once, Some(d(2024, 3, 1)), d(2024, 4, 30)).unwrap();
    let daily = scratch("day-by-day");
    let mut to = d(2024, 3, 1);
    build_into(&source, &daily, Some(d(2024, 3, 1)), to).unwrap();
    while to < d(2024, 4, 30) {
        to = to.succ().unwrap();
        build_into(&source, &daily, None, to).unwrap();
    }
    assert_eq!(
        newest(&once),
        newest(&daily),
        "the newest bytes are the same"
    );
    let (a, b) = (
        Membership::open(&once).unwrap(),
        Membership::open(&daily).unwrap(),
    );
    assert_eq!(
        a.file_digest(),
        b.file_digest(),
        "the generation is not identity"
    );
    assert_eq!(a.known_span(), Some((d(2024, 3, 1), d(2024, 4, 30))));
    assert!(
        b.months().iter().any(|(_, generation, _)| *generation > 1),
        "the daily build extended its months generation by generation"
    );
    // A third, resumed run with nothing new writes nothing.
    let before = files(&daily);
    build(&source, &daily, Window::Resume, 2, &mut |_| {}).unwrap();
    assert_eq!(files(&daily), before);
}

#[test]
fn the_carry_crosses_a_month_and_a_month_computes_only_itself() {
    // ZZALPHA's last file is 20 Mar and its contract settles 28 Mar: a member
    // to the 28th, gone in April; ZZBETA's April contract is first listed
    // 1 Apr. April's file starts from March's carry.
    let source = two_months();
    let dir = scratch("carry");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 4, 30)).unwrap();
    let held = Membership::open(&dir).unwrap();
    assert_eq!(held.is_member("ZZALPHA", d(2024, 3, 27)), Ok(true));
    assert_eq!(held.is_member("ZZALPHA", d(2024, 4, 1)), Ok(false));
    assert_eq!(held.is_member("ZZNEWCO", d(2024, 3, 28)), Ok(true));
    assert_eq!(
        held.is_member("ZZNEWCO", d(2024, 4, 1)),
        Ok(false),
        "expired 28 Mar"
    );
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].table.carry_out(), months[1].table.carry_in());
    assert!(!months[1].table.carry_in().live.is_empty());
    // A missing session inside the span is refused and the carry survives it.
    assert_eq!(
        held.row(d(2024, 3, 13)).unwrap().status,
        DayStatus::NoSource
    );
    assert_eq!(held.is_member("ZZALPHA", d(2024, 3, 14)), Ok(true));
}

#[test]
fn a_window_overlapping_a_sealed_record_is_refused_and_so_is_a_gap() {
    let source = two_months();
    let dir = scratch("overlap");
    build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    let before = files(&dir);
    assert!(matches!(
        build_into(&source, &dir, Some(d(2024, 3, 15)), d(2024, 3, 31)),
        Err(MembershipError::Overlap { .. })
    ));
    assert!(matches!(
        build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 31)),
        Err(MembershipError::Overlap { .. })
    ));
    assert!(matches!(
        build_into(&source, &dir, Some(d(2024, 3, 20)), d(2024, 3, 31)),
        Err(MembershipError::Gap { .. })
    ));
    assert_eq!(files(&dir), before, "a refusal writes nothing");
    let refusal = build_into(&source, &dir, Some(d(2024, 3, 15)), d(2024, 3, 31))
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("never rebuilt"), "{refusal}");
}

#[test]
fn a_resume_refuses_a_source_whose_last_sealed_session_changed() {
    let source = two_months();
    let dir = scratch("changed");
    build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    let changed = source.clone().list(d(2024, 3, 15), &["ZZBETA28MAR24700PE"]);
    let before = files(&dir);
    assert_eq!(
        build_into(&changed, &dir, None, d(2024, 3, 31)).map(|_| ()),
        Err(MembershipError::SourceChanged {
            day: d(2024, 3, 15)
        })
    );
    assert_eq!(files(&dir), before);
    // The unchanged source resumes.
    build_into(&source, &dir, None, d(2024, 3, 31)).unwrap();
}

#[test]
fn a_rerun_writes_nothing_and_a_new_generation_only_extends_the_old() {
    let source = two_months();
    let dir = scratch("generations");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    let table = months[0].table.clone();
    assert!(matches!(
        months[0].installed,
        Installed::Written { generation: 1, .. }
    ));
    assert!(matches!(
        install(&dir, &table),
        Ok(Installed::Unchanged { generation: 1, .. })
    ));
    // A table that changes a sealed row is refused, and generation 1 stays.
    let mut changed = table.clone();
    changed.rows[0].names += 1;
    assert!(matches!(
        install(&dir, &changed),
        Err(MembershipError::WouldRewrite { .. })
    ));
    assert_eq!(files(&dir).len(), 1);
    // Extending writes generation 2 beside generation 1.
    build_into(&source, &dir, None, d(2024, 3, 22)).unwrap();
    let names: Vec<String> = files(&dir).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, [file_name(2024, 3, 1), file_name(2024, 3, 2)]);
    let first = MonthTable::decode(&files(&dir)[0].1).unwrap();
    let second = MonthTable::decode(&files(&dir)[1].1).unwrap();
    assert!(second.extends(&first));
    assert!(!first.extends(&second));
}

#[test]
fn a_damaged_newest_generation_is_refused_never_served_from_the_older_one() {
    let source = two_months();
    let dir = scratch("damaged");
    build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    build_into(&source, &dir, None, d(2024, 3, 22)).unwrap();
    let second = dir.join(file_name(2024, 3, 2));
    let mut bytes = std::fs::read(&second).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&second, &bytes).unwrap();
    let refusal = Membership::open(&dir).unwrap_err();
    assert!(
        matches!(&refusal, MembershipError::Corrupt { path, why }
            if *path == second && why.contains("BLAKE3")),
        "{refusal}"
    );
}

#[test]
fn an_unknown_file_is_refused_by_name_and_a_hidden_one_is_counted() {
    let source = two_months();
    let dir = scratch("names");
    build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    std::fs::write(dir.join(".2024-03-g000002.fnm.tmp.1"), b"half").unwrap();
    let held = Membership::open(&dir).unwrap();
    assert_eq!(held.temporaries(), 1, "counted, never read");
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    assert!(matches!(
        Membership::open(&dir),
        Err(MembershipError::Corrupt { path, .. }) if path.ends_with("notes.txt")
    ));
    for name in [
        "2024-3-g000001.fnm",
        "2024-03-g1.fnm",
        "2024-03-g000001.fnm.bak",
    ] {
        assert_eq!(parse_file_name(name), None, "{name}");
    }
    assert_eq!(parse_file_name("2024-03-g000007.fnm"), Some((2024, 3, 7)));
}

/// Re-seals `bytes`' digest after a change to its body.
fn reseal(mut bytes: Vec<u8>) -> Vec<u8> {
    let body = bytes.len() - DIGEST_LEN;
    let digest = brutex_core::blake3::hash(&bytes[..body]);
    bytes[body..].copy_from_slice(&digest);
    bytes
}

/// One damage done to a month file's bytes.
type Damage = Box<dyn Fn(&mut Vec<u8>)>;

#[test]
fn every_decoder_check_refuses_its_own_damage() {
    let source = two_months();
    let dir = scratch("decoder");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    let good = months[0].table.encode();
    assert_eq!(MonthTable::decode(&good).unwrap(), months[0].table);
    let added = months[0].table.added().len();
    let rows = HEADER_LEN + added * SPELLING_LEN;
    let entries = rows + months[0].table.rows().len() * DAY_ROW_LEN;
    let cases: Vec<(&str, Damage, bool)> = vec![
        ("BLAKE3", Box::new(|b: &mut Vec<u8>| b[30] ^= 1), false),
        ("magic", Box::new(|b: &mut Vec<u8>| b[0] = b'X'), true),
        ("version", Box::new(|b: &mut Vec<u8>| b[8] = 9), true),
        ("header length", Box::new(|b: &mut Vec<u8>| b[10] = 1), true),
        ("reserved byte", Box::new(|b: &mut Vec<u8>| b[17] = 1), true),
        ("largest pool", Box::new(|b: &mut Vec<u8>| b[18] += 1), true),
        (
            "span starts after",
            Box::new(|b: &mut Vec<u8>| b[21] = 0xff),
            true,
        ),
        ("reserved word", Box::new(|b: &mut Vec<u8>| b[68] = 1), true),
        (
            "implies another length",
            Box::new(|b: &mut Vec<u8>| b[16] -= 1),
            true,
        ),
        (
            "day status",
            Box::new(move |b: &mut Vec<u8>| b[rows] = 9),
            true,
        ),
        (
            "running sum",
            Box::new(move |b: &mut Vec<u8>| b[rows + 6] = 7),
            true,
        ),
        (
            "unknown flag",
            Box::new(move |b: &mut Vec<u8>| b[entries + 2] = 4),
            true,
        ),
        (
            "shorter than its digest",
            Box::new(|b: &mut Vec<u8>| b.truncate(8)),
            false,
        ),
    ];
    for (expected, damage, sealed) in cases {
        let mut bytes = good.clone();
        damage(&mut bytes);
        if sealed {
            bytes = reseal(bytes);
        }
        let refusal = MonthTable::decode(&bytes).unwrap_err().to_string();
        assert!(refusal.contains(expected), "{expected}: {refusal}");
    }
    assert_eq!(ENTRY_LEN, 4);
}

#[test]
fn a_month_that_does_not_continue_the_one_before_is_refused_as_stale() {
    let source = two_months();
    let dir = scratch("stale");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 4, 30)).unwrap();
    let (march, april) = (months[0].table.clone(), months[1].table.clone());
    assert!(Membership::of([march.clone(), april.clone()]).is_ok());
    // April alone does not start the span; April before March is a gap.
    let refusal = Membership::of([april.clone()]).unwrap_err().to_string();
    assert!(refusal.contains("span does not start"), "{refusal}");
    // An April built from another carry than March ends with.
    let mut other = april;
    other.carry_in.refused_in_year += 1;
    let refusal = Membership::of([march, other]).unwrap_err();
    assert!(
        matches!(&refusal, MembershipError::Stale { why, .. } if why.contains("carry")),
        "{refusal}"
    );
}

#[test]
fn every_chain_check_refuses_its_own_damage() {
    // The reader's checks across months and inside the day index, each hit
    // by one damaged table and refused by name (operator rule 6: an extreme
    // case is refused loudly, never served). Tables are served without files
    // (`Membership::of`), so the decoder's own checks do not stand in front.
    let source = two_months();
    let dir = scratch("stale");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 4, 30)).unwrap();
    let (march, april) = (months[0].table.clone(), months[1].table.clone());
    assert!(Membership::of([march.clone(), april.clone()]).is_ok());
    assert_eq!(march.handovers.len(), 1, "ZZOLDCO -> ZZNEWCO on 6 Mar");
    let handover = march.handovers[0];
    let at = |name: &str| {
        march
            .added
            .iter()
            .position(|spelling| spelling.name.as_str() == name)
            .unwrap()
    };
    let refused = |tables: Vec<MonthTable>, says: &str| {
        let why = Membership::of(tables).unwrap_err().to_string();
        assert!(why.contains(says), "{says}: {why}");
    };
    // The span's first month carries nothing in.
    let mut first = march.clone();
    first.carry_in.refused_in_year += 1;
    refused(vec![first], "the span's first month starts from a carry");
    // A month that ends a day early leaves a gap before the next.
    let mut short = march.clone();
    short.rows.pop();
    refused(vec![short, april.clone()], "a gap lies between this month");
    // Every month names the span's one start.
    let mut start = april.clone();
    start.known_from += 1;
    refused(vec![march.clone(), start], "it names another span start");
    // The registry continues the month before's, spelling for spelling.
    let mut registry = april.clone();
    registry.spellings_before += 1;
    refused(
        vec![march.clone(), registry],
        "its registry does not continue",
    );
    // A spelling that opens a series takes the next id.
    let mut order = march.clone();
    order.added[at("ZZBETA")].series_id += 7;
    refused(vec![order], "a spelling names a series out of order");
    // A handover joins two spellings of ONE series.
    let mut two = march.clone();
    two.handovers[0].old = u32::try_from(at("ZZALPHA")).unwrap();
    refused(vec![two], "a handover joins spellings of two series");
    // A handover lies on one of the month's known records (Sun 3 Mar is not).
    let mut closed = march.clone();
    closed.handovers[0].day = d(2024, 3, 3);
    refused(vec![closed], "a handover lies on no known record");
    // A day's members are in series order.
    let mut swapped = march.clone();
    swapped.entries.swap(0, 1);
    refused(vec![swapped], "a day's members are not in series order");
    // A day's members are all there (a file's decoder checks this too).
    let mut cut = march.clone();
    cut.entries.truncate(1);
    refused(vec![cut], "a day's members run past the month's entries");
    // One spelling names one series once.
    let mut twice = march.clone();
    let (old, new) = (
        usize::try_from(handover.old).unwrap(),
        usize::try_from(handover.new).unwrap(),
    );
    twice.added[new].name = twice.added[old].name;
    refused(vec![twice], "is registered twice");
}

#[test]
fn refused_sessions_are_running_sums_per_year_indexed_by_day() {
    // Known on 27 Dec 2024, 31 Dec 2024 and 2 Jan 2025; 30 Dec and 1 Jan are
    // sessions with no file. The sum restarts with the year.
    let source = Fake::default()
        .list(d(2024, 12, 27), &["ZZALPHA30JAN25500CE"])
        .list(d(2024, 12, 31), &["ZZALPHA30JAN25500CE"])
        .list(d(2025, 1, 2), &["ZZALPHA30JAN25500CE"]);
    let dir = scratch("running");
    build_into(&source, &dir, Some(d(2024, 12, 1)), d(2025, 1, 31)).unwrap();
    let held = Membership::open(&dir).unwrap();
    assert_eq!(held.refused_through(d(2024, 12, 27)), Ok(0));
    assert_eq!(held.refused_through(d(2024, 12, 30)), Ok(1));
    assert_eq!(held.refused_through(d(2024, 12, 31)), Ok(1));
    assert_eq!(held.refused_through(d(2025, 1, 1)), Ok(1), "a new year");
    assert_eq!(held.refused_through(d(2025, 1, 2)), Ok(1));
    let counts = held.year_counts();
    let year = |y| *counts.iter().find(|count| count.year == y).unwrap();
    assert_eq!((year(2024).known, year(2024).refused_inside), (2, 1));
    assert_eq!((year(2025).known, year(2025).refused_inside), (1, 1));
    assert!(
        year(2024).outside > 200,
        "sessions outside the span are counted"
    );
    assert!(year(2019).outside > 200);
}

#[test]
fn the_digest_names_the_content_and_not_the_path_or_generation() {
    let source = two_months();
    let (a, b) = (scratch("digest-a"), scratch("digest-b"));
    build_into(&source, &a, Some(d(2024, 3, 1)), d(2024, 3, 31)).unwrap();
    build_into(&source, &b, Some(d(2024, 3, 1)), d(2024, 3, 31)).unwrap();
    let (a, b) = (Membership::open(&a).unwrap(), Membership::open(&b).unwrap());
    assert_eq!(a.file_digest(), b.file_digest());
    assert_eq!(a.file_digest_hex().len(), 64);
    let fewer = scratch("digest-c");
    build_into(&source, &fewer, Some(d(2024, 3, 1)), d(2024, 3, 27)).unwrap();
    let fewer = Membership::open(&fewer).unwrap();
    assert_ne!(a.file_digest(), fewer.file_digest());
    assert_eq!(a.max_pool(), 3);
}

#[test]
fn spans_open_and_close_on_known_days_and_count_unknown_ones() {
    let source = two_months();
    let dir = scratch("spans");
    build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 4, 30)).unwrap();
    let held = Membership::open(&dir).unwrap();
    let spans = held.spans();
    let of = |name: &str| {
        spans
            .iter()
            .find(|span| span.share.as_str() == name)
            .cloned()
            .unwrap()
    };
    let alpha = of("ZZALPHA");
    assert_eq!((alpha.entry, alpha.exit), (d(2024, 3, 1), d(2024, 3, 28)));
    assert_eq!(alpha.unknown_inside, 1, "13 March");
    assert_eq!(alpha.source, "inferred_gdfl_options");
    let renamed = of("ZZNEWCO");
    assert_eq!(
        (renamed.entry, renamed.exit),
        (d(2024, 3, 1), d(2024, 3, 28)),
        "one span across the handover"
    );
    assert_eq!(of("ZZBETA").exit, d(2024, 4, 30));
}

#[test]
fn the_state_refuses_a_day_out_of_order() {
    let mut state = State::default();
    let input = || DayInput::Unknown(DayStatus::NotSession, None);
    state.fold(d(2024, 3, 2), input()).unwrap();
    assert_eq!(
        state.fold(d(2024, 3, 4), input()),
        Err(MembershipError::Gap {
            from: d(2024, 3, 4),
            last: d(2024, 3, 2)
        })
    );
    assert!(matches!(
        state.fold(d(2024, 3, 3), DayInput::Unknown(DayStatus::Known, None)),
        Err(MembershipError::Corrupt { .. })
    ));
}

/// Every session of August and September 2024 lists ZZBETA's current
/// contract. August ends on Sat 31 Aug, so August is installed with its
/// closed days when Mon 2 Sep folds, BEFORE September is installed.
fn aug_sep() -> Fake {
    let mut source = Fake::default();
    let mut day = d(2024, 8, 1);
    while day <= d(2024, 9, 30) {
        let ticker = if day <= d(2024, 8, 29) {
            "ZZBETA29AUG24700PE"
        } else if day <= d(2024, 9, 26) {
            "ZZBETA26SEP24700PE"
        } else {
            "ZZBETA31OCT24700PE"
        };
        if matches!(
            sourced_kind_of(i64::from(day.days_from_epoch())),
            DayKind::Open(_)
        ) {
            source.days.insert(day, vec![ticker.to_owned()]);
        }
        day = day.succ().unwrap();
    }
    source
}

/// The typed window of [`aug_sep`].
fn aug_sep_window() -> Window {
    Window::Typed {
        from: Some(d(2024, 8, 1)),
        to: d(2024, 9, 30),
    }
}

#[test]
fn a_kill_between_two_month_installs_leaves_a_servable_resumable_directory() {
    // verify-PIT-1-r1 blocking 1 (D-5167): the build dies right after August
    // (ending on Sat 31 Aug) is installed and before September is. The
    // directory used to be refused by `open` and by the resume ("the span
    // does not end on a known record") for ever.
    let source = aug_sep();
    let once = scratch("kill-once");
    build(&source, &once, aug_sep_window(), 2, &mut |_| {}).unwrap();
    let cut = scratch("kill-cut");
    let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build(&source, &cut, aug_sep_window(), 2, &mut |month| {
            assert_eq!(month.table.month(), (2024, 8), "August first");
            panic!("killed between the two installs");
        })
    }));
    assert!(died.is_err(), "the build died after August's install");
    assert_eq!(files(&cut).len(), 1, "August alone is installed");
    let held = Membership::open(&cut).unwrap();
    assert_eq!(held.known_span(), Some((d(2024, 8, 1), d(2024, 8, 30))));
    assert_eq!(held.sealed_through(), Some(d(2024, 8, 31)));
    assert_eq!(held.pool_size(d(2024, 8, 30)), Ok(1));
    // The sealed closed day after known_to answers as after the span, as a
    // build that ended on 30 Aug would.
    assert_eq!(
        held.members(d(2024, 8, 31)).unwrap_err().why,
        UnknownWhy::AfterSpan {
            known_to: d(2024, 8, 30)
        }
    );
    // The resume continues after the last SEALED record and ends byte for
    // byte where one pass ends.
    build(
        &source,
        &cut,
        Window::Typed {
            from: None,
            to: d(2024, 9, 30),
        },
        2,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(newest(&cut), newest(&once));
    let (a, b) = (
        Membership::open(&cut).unwrap(),
        Membership::open(&once).unwrap(),
    );
    assert_eq!(a.file_digest(), b.file_digest());
    assert_eq!(a.sealed_through(), Some(d(2024, 9, 30)));
    assert_eq!(
        a.members(d(2024, 8, 31)).unwrap_err().why,
        UnknownWhy::Refused(DayStatus::NotSession),
        "sealed inside the span once September is"
    );
}

#[test]
fn a_failed_write_between_two_month_installs_resumes_to_the_one_pass_bytes() {
    // The same window, the second install refused by the file system (the
    // directory made read-only once August is in). Under an account the
    // mode does not stop (root), the build succeeds and the bytes are still
    // compared.
    use std::os::unix::fs::PermissionsExt as _;
    let source = aug_sep();
    let once = scratch("fail-once");
    build(&source, &once, aug_sep_window(), 2, &mut |_| {}).unwrap();
    let dir = scratch("fail-cut");
    let mode = |bits| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(bits));
    let built = build(&source, &dir, aug_sep_window(), 2, &mut |month| {
        if month.table.month() == (2024, 8) {
            mode(0o555).unwrap();
        }
    });
    let stopped = std::fs::write(dir.join(".probe"), b"").is_err();
    mode(0o755).unwrap();
    if stopped {
        assert!(
            matches!(&built, Err(MembershipError::Io { .. })),
            "{built:?}"
        );
        let names: Vec<String> = files(&dir).into_iter().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [file_name(2024, 8, 1)],
            "August only, and no temporary"
        );
        let held = Membership::open(&dir).unwrap();
        assert_eq!(held.known_span().map(|(_, to)| to), Some(d(2024, 8, 30)));
        build(
            &source,
            &dir,
            Window::Typed {
                from: None,
                to: d(2024, 9, 30),
            },
            2,
            &mut |_| {},
        )
        .unwrap();
    } else {
        built.unwrap();
        std::fs::remove_file(dir.join(".probe")).unwrap();
    }
    assert_eq!(newest(&dir), newest(&once));
}

#[test]
fn a_handover_needs_a_majority_of_both_sides_and_two_shared_keys() {
    // verify-PIT-1-r1 blocking 4 (probe c5_4, D-5166). Mon 4 Mar 2024:
    // ZZILLIQ lists one key, ZZONE one other, ZZTWO two more. Tue 5 Mar none
    // of them lists; ZZNEWBIE lists ZZILLIQ's key and three of its own,
    // ZZNEWONE lists exactly ZZONE's key, ZZNEWTWO exactly ZZTWO's two.
    let source = Fake::default()
        .list(
            d(2024, 3, 4),
            &[
                "ZZILLIQ28MAR24500CE",
                "ZZONE28MAR24900PE",
                "ZZTWO28MAR241200CE",
                "ZZTWO28MAR241210CE",
            ],
        )
        .list(
            d(2024, 3, 5),
            &[
                "ZZNEWBIE28MAR24500CE",
                "ZZNEWBIE28MAR24600CE",
                "ZZNEWBIE28MAR24700PE",
                "ZZNEWBIE25APR24500CE",
                "ZZNEWONE28MAR24900PE",
                "ZZNEWTWO28MAR241200CE",
                "ZZNEWTWO28MAR241210CE",
            ],
        );
    let (held, months, _) = march(&source, "both-sides");
    let series = |name| held.series_of(name).unwrap().unwrap();
    // A minority of the new spelling's keys: two companies, never one.
    assert_ne!(series("ZZILLIQ"), series("ZZNEWBIE"));
    // One shared key on each side is no evidence: two series.
    assert_ne!(series("ZZONE"), series("ZZNEWONE"));
    // Two shared keys, a majority of both: one series (the boundary).
    assert_eq!(series("ZZTWO"), series("ZZNEWTWO"));
    assert_eq!(held.handovers().len(), 1);
    // ZZILLIQ and ZZONE stay members to their expiry with no file: the pool
    // of the 5th is ZZILLIQ, ZZONE, ZZTWO's series, ZZNEWBIE, ZZNEWONE.
    assert_eq!(held.pool_size(d(2024, 3, 5)), Ok(5));
    assert_eq!(held.is_member("ZZILLIQ", d(2024, 3, 5)), Ok(true));
    let reported: Vec<&str> = months[0]
        .ambiguous
        .iter()
        .map(|(_, why)| why.as_str())
        .collect();
    assert_eq!(reported.len(), 2, "{reported:?}");
    assert!(
        reported
            .iter()
            .any(|why| why.starts_with("ZZILLIQ -> ZZNEWBIE")
                && why.ends_with("a minority of the new spelling's keys")),
        "{reported:?}"
    );
    assert!(
        reported
            .iter()
            .any(|why| why.starts_with("ZZONE -> ZZNEWONE")
                && why.ends_with("fewer than 2 shared keys")),
        "{reported:?}"
    );
}

#[test]
fn a_file_under_a_spelling_its_series_moved_on_from_is_filed_under_the_newest() {
    // verify-PIT-1-r1 blocking 4 (probe c5_5, D-5166): ZZOLDCO hands over to
    // ZZNEWCO on Wed 6 Mar 2024, then lists one file alone on Thu 7 Mar and
    // one beside ZZNEWCO's on Mon 11 Mar (Fri 8 Mar 2024 was no session,
    // Mahashivratri). The build used to stop on the 7th
    // ("one series would be live as ZZOLDCO and as ZZNEWCO") on every rerun.
    let (old, new) = (keys("ZZOLDCO"), keys("ZZNEWCO"));
    let mut both = new.clone();
    both.push("ZZOLDCO28MAR24520CE".to_owned());
    let source = Fake::default()
        .list(d(2024, 3, 4), &refs(&old))
        .list(d(2024, 3, 5), &refs(&old))
        .list(d(2024, 3, 6), &refs(&new))
        .list(d(2024, 3, 7), &["ZZOLDCO28MAR24500CE"])
        .list(d(2024, 3, 11), &refs(&both));
    let (held, months, dir) = march(&source, "stray");
    for day in [d(2024, 3, 7), d(2024, 3, 11)] {
        assert_eq!(held.pool_size(day), Ok(1), "{day}");
        assert_eq!(names(&held, day), ["ZZNEWCO"], "{day}");
        assert!(
            held.members(day).unwrap()[0].filed,
            "{day}: a file was listed"
        );
        assert_eq!(held.is_member("ZZOLDCO", day), Ok(true), "{day}");
    }
    assert_eq!(held.series_count(), 1);
    assert_eq!(held.handovers().len(), 1);
    let strays: Vec<(Day, &str)> = months[0]
        .strays
        .iter()
        .map(|(day, why)| (*day, why.as_str()))
        .collect();
    assert_eq!(strays.len(), 2, "{strays:?}");
    assert_eq!(strays[0].0, d(2024, 3, 7));
    assert!(
        strays[0].1.starts_with("ZZOLDCO listed 1 contract key(s)")
            && strays[0].1.contains("filed under ZZNEWCO"),
        "{strays:?}"
    );
    // A rerun resumes past it, and a day-by-day build writes the same bytes.
    build_into(&source, &dir, None, d(2024, 4, 5)).unwrap();
    let daily = scratch("stray-daily");
    let mut to = d(2024, 3, 1);
    build_into(&source, &daily, Some(to), to).unwrap();
    while to < d(2024, 4, 5) {
        to = to.succ().unwrap();
        build_into(&source, &daily, None, to).unwrap();
    }
    assert_eq!(newest(&daily), newest(&dir));
}

#[test]
fn a_refused_source_day_is_never_sealed_and_the_next_build_resumes_at_it() {
    // D-5168 (verify-PIT-1-r1 follow-up): a refusal may be transient, so a
    // refused day is not sealed as refused for ever. Known on 4, 6 and 7 Mar
    // 2024; the 5th has no file; the source refuses the 6th until it
    // recovers.
    let listed = Fake::default()
        .list(d(2024, 3, 4), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 6), &["ZZALPHA28MAR24500CE"])
        .list(d(2024, 3, 7), &["ZZALPHA28MAR24500CE"]);
    let refusing = listed.clone().refuse(d(2024, 3, 6));
    let dir = scratch("refused");
    let refusal = build_into(&refusing, &dir, Some(d(2024, 3, 1)), d(2024, 3, 31))
        .map(|_| ())
        .unwrap_err();
    assert!(
        matches!(&refusal, MembershipError::SourceRefused { day, why }
            if *day == d(2024, 3, 6) && !why.is_empty()),
        "{refusal}"
    );
    assert!(refusal.to_string().contains("is not sealed"), "{refusal}");
    // Every record before it is installed: the 4th, known. The 5th waits
    // with the 6th: a session is sealed only when a known day follows.
    let held = Membership::open(&dir).unwrap();
    assert_eq!(held.known_span(), Some((d(2024, 3, 4), d(2024, 3, 4))));
    assert_eq!(held.sealed_through(), Some(d(2024, 3, 4)));
    assert_eq!(
        held.members(d(2024, 3, 6)).unwrap_err().why,
        UnknownWhy::AfterSpan {
            known_to: d(2024, 3, 4)
        }
    );
    // Still refusing: the next build stops at the same day, writing nothing.
    let before = files(&dir);
    assert!(matches!(
        build_into(&refusing, &dir, None, d(2024, 3, 31)),
        Err(MembershipError::SourceRefused { .. })
    ));
    assert_eq!(files(&dir), before);
    // Recovered: the resume seals the 6th and ends where one pass ends.
    build_into(&listed, &dir, None, d(2024, 3, 31)).unwrap();
    let once = scratch("refused-once");
    build_into(&listed, &once, Some(d(2024, 3, 1)), d(2024, 3, 31)).unwrap();
    assert_eq!(newest(&dir), newest(&once));
    // The fold never seals one either (a reader thread that panicked is the
    // same refusal), and a file holding one is refused by the decoder.
    let mut state = State::default();
    assert!(matches!(
        state.fold(
            d(2024, 3, 4),
            DayInput::Unknown(
                DayStatus::SourceRefused,
                Some("a reader thread panicked".into())
            )
        ),
        Err(MembershipError::SourceRefused { .. })
    ));
    let mut table = MonthTable::decode(&newest(&once)[0].1).unwrap();
    let fifth = usize::from(5 - table.first);
    assert_eq!(table.rows[fifth].status, DayStatus::NoSource);
    table.rows[fifth].status = DayStatus::SourceRefused;
    let refusal = MonthTable::decode(&table.encode()).unwrap_err().to_string();
    assert!(refusal.contains("never sealed"), "{refusal}");
}

#[test]
fn a_missing_directory_is_named_and_never_served_as_empty() {
    // verify-PIT-1-r1 follow-up: `open` served a missing directory as an
    // empty membership, so a wrong path was never named.
    let missing = scratch("missing").join("no-such-dir");
    let refusal = Membership::open(&missing).unwrap_err();
    assert!(
        matches!(&refusal, MembershipError::Io { path, why }
            if *path == missing && why.contains("no such membership directory")),
        "{refusal}"
    );
    // A build is the one caller that starts from nothing: it makes it.
    build_into(&two_months(), &missing, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    assert_eq!(
        Membership::open(&missing).unwrap().known_span(),
        Some((d(2024, 3, 1), d(2024, 3, 15)))
    );
}

#[test]
fn a_lost_link_race_is_refused_by_name_and_the_other_writers_file_is_kept() {
    // verify-PIT-1-r1 follow-up: the lost link race returned "nothing
    // written" and left the hidden temporary. `publish` names the race and
    // writes nothing through the other writer's name; `install` unlinks its
    // temporary whatever `publish` answered (its one `remove_file`, before
    // any return), and the temporary is made by `store::staging::create`,
    // gate 28's one door.
    let dir = scratch("race");
    let path = dir.join(file_name(2024, 3, 1));
    std::fs::write(&path, b"another writer").unwrap();
    let temporary = dir.join(format!(
        ".{}.tmp.{}",
        file_name(2024, 3, 1),
        std::process::id()
    ));
    let staged = store::staging::create(&temporary).unwrap();
    assert!(matches!(
        super::publish(staged.file, b"bytes", &temporary, &path),
        Err(MembershipError::Exists { .. })
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"another writer");
    // The same name staged again by this writer: a dead writer's leftover of
    // one name is cleared by the door, never written through.
    let again = store::staging::create(&temporary).unwrap();
    assert_eq!(again.cleared, Some(5));
}

#[test]
fn a_dead_writers_leftover_under_the_temporary_name_is_reported_never_dropped() {
    // `store::staging::create` clears a dead writer's leftover under the name
    // it is asked for and says so in `cleared`, which its caller reports;
    // `install` used to drop that answer. A leftover planted under the name
    // this install will use is cleared, its size is returned in `Written`,
    // no hidden file is left and the month's bytes are the table's own.
    let source = two_months();
    let dir = scratch("generations");
    let (_, months) = build_into(&source, &dir, Some(d(2024, 3, 1)), d(2024, 3, 15)).unwrap();
    // A build through `install` names its temporary itself: nothing cleared.
    assert!(matches!(
        months[0].installed,
        Installed::Written {
            generation: 1,
            cleared: None,
            ..
        }
    ));
    // The same month to a later day, as a resumed build would install it.
    let more = scratch("generations");
    let (_, longer) = build_into(&source, &more, Some(d(2024, 3, 1)), d(2024, 3, 22)).unwrap();
    let longer = &longer[0].table;
    let planted = |name: &str| dir.join(format!(".{name}.tmp.{}", std::process::id()));
    let leftover = planted(&file_name(2024, 3, 2));
    let dead = b"a dead writer's bytes";
    std::fs::write(&leftover, dead).unwrap();
    let installed = super::install_with(&dir, longer, &planted).unwrap();
    assert_eq!(
        installed,
        Installed::Written {
            path: dir.join(file_name(2024, 3, 2)),
            generation: 2,
            cleared: Some(u64::try_from(dead.len()).unwrap()),
        }
    );
    assert!(!leftover.exists());
    assert_eq!(
        std::fs::read(dir.join(file_name(2024, 3, 2))).unwrap(),
        longer.encode()
    );
    let hidden = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .is_ok_and(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        })
        .count();
    assert_eq!(hidden, 0);
}

#[test]
fn two_builds_of_one_directory_at_once_leave_no_temporary_and_the_one_pass_bytes() {
    // verify-PIT-1-r1 probe c5_10 made a test: two builds of one directory on
    // two threads, 40 times. Whichever wins each install, a refused one says
    // why and leaves no hidden temporary (each is this call's own, made by
    // `store::staging::create` and unlinked before `install` returns), the
    // directory serves, and a resume ends on the one-pass bytes.
    let source = aug_sep();
    let once = scratch("two-once");
    build(&source, &once, aug_sep_window(), 2, &mut |_| {}).unwrap();
    for round in 0..40 {
        let dir = scratch(&format!("two-{round}"));
        let run = || build(&source, &dir, aug_sep_window(), 2, &mut |_| {}).map(|_| ());
        let (a, b) = std::thread::scope(|scope| {
            let first = scope.spawn(run);
            let second = scope.spawn(run);
            (first.join().unwrap(), second.join().unwrap())
        });
        assert!(a.is_ok() || b.is_ok(), "round {round}: {a:?} | {b:?}");
        for refusal in [&a, &b]
            .into_iter()
            .filter_map(|outcome| outcome.as_ref().err())
        {
            assert!(
                matches!(
                    refusal,
                    MembershipError::Exists { .. } | MembershipError::Overlap { .. }
                ),
                "round {round}: {refusal}"
            );
        }
        let hidden: Vec<String> = files(&dir)
            .into_iter()
            .map(|(name, _)| name)
            .filter(|name| name.starts_with('.'))
            .collect();
        assert!(hidden.is_empty(), "round {round}: {hidden:?}");
        assert_eq!(generations(&dir).unwrap().1, 0, "round {round}");
        assert_eq!(
            Membership::open(&dir).unwrap().sealed_through(),
            Some(d(2024, 9, 30)),
            "round {round}: the writer that finished sealed every day"
        );
        // A resume after both writes nothing: the source holds no later
        // known day, so the days after 30 Sep are trailing, never sealed.
        let resumed = build(&source, &dir, Window::Resume, 2, &mut |_| {}).unwrap();
        assert_eq!(resumed.folded, 0, "round {round}");
        assert_eq!(newest(&dir), newest(&once), "round {round}");
    }
}
