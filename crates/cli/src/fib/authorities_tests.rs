//! Tests of the FIB authorities (FIB-8; D-6442).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::HashMap;

use pull::calendar::SpecialKind;

use super::*;

/// The IST epoch day of a civil date.
fn day(year: u16, month: u8, of_month: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, of_month)
            .unwrap()
            .days_from_epoch(),
    )
}

/// A synthetic, non-interim version of `kind`.
fn measured(kind: AuthorityKind, label: &'static str) -> AuthorityVersion {
    AuthorityVersion {
        kind,
        word: "synthetic",
        label,
        digest: version_digest(kind, "synthetic", label, b"t"),
        interim: false,
    }
}

/// Landings by day, and one break.
struct Landings {
    version: AuthorityVersion,
    by_day: HashMap<i64, u64>,
    split: Option<(i64, BasisBreak)>,
}

impl LandingAuthority for Landings {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn landing(&self, _: SeriesId, day: i64) -> Option<LandingId> {
        self.by_day.get(&day).copied().map(LandingId)
    }

    fn basis_break(&self, _: SeriesId, _: i64, day: i64) -> Option<BasisBreak> {
        self.split
            .filter(|(on, _)| *on == day)
            .map(|(_, record)| record)
    }
}

/// Factor pairs by day.
struct Factors {
    version: AuthorityVersion,
    close: HashMap<i64, FactorPair>,
    first: HashMap<i64, FactorPair>,
    from: i64,
}

impl FactorAuthority for Factors {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }

    fn factor(&self, _: SeriesId, day: i64) -> Result<FactorPair, String> {
        self.close
            .get(&day)
            .copied()
            .ok_or_else(|| "no pair".to_owned())
    }

    fn first_minute(&self, _: SeriesId, day: i64) -> Result<FactorPair, String> {
        self.first
            .get(&day)
            .copied()
            .ok_or_else(|| "no first minute".to_owned())
    }

    fn covers(&self, day: i64) -> bool {
        day >= self.from
    }
}

fn pair(zerodha_close: i64, gdfl_close: i64, shape: FactorShape) -> FactorPair {
    FactorPair {
        zerodha_close,
        gdfl_close,
        shape,
        measured: true,
    }
}

/// THE INTERIM GATE'S REASON FOR EACH NAMED DAY, PINNED, so FIB-10's swap
/// shows its difference.
#[test]
fn the_interim_calendar_excludes_the_named_days_by_reason() {
    let calendar = InterimCalendar::new();
    let series = SeriesId(7);
    let reason = |on: i64| calendar.tradable(series, on).market.unwrap_err();
    let budget = reason(day(2020, 2, 1));
    assert_eq!(budget.code, CalendarCode::SpecialSession);
    assert_eq!(budget.special, Some(SpecialKind::BudgetWeekend));
    assert!(budget.known_in_advance);
    let weekend = reason(day(2024, 1, 20));
    assert_eq!(weekend.code, CalendarCode::SpecialSession);
    assert_eq!(weekend.special, Some(SpecialKind::WeekendFullSession));
    assert!(weekend.known_in_advance);
    let muhurat = reason(day(2025, 10, 21));
    assert_eq!(muhurat.code, CalendarCode::SpecialSession);
    assert_eq!(muhurat.special, Some(SpecialKind::Muhurat));
    assert!(muhurat.known_in_advance);
    let halt = reason(day(2020, 3, 13));
    assert_eq!(halt.code, CalendarCode::Halt);
    assert_eq!(halt.special, None);
    assert!(!halt.known_in_advance, "a halt is known only in hindsight");
    let outage = reason(day(2021, 2, 24));
    assert_eq!(outage.code, CalendarCode::SystemsOutage);
    assert!(!outage.known_in_advance);
    let sunday = reason(day(2025, 3, 2));
    assert_eq!(sunday.code, CalendarCode::Closed);
    assert!(sunday.known_in_advance);
    let beyond = reason(pull::calendar::SOURCED_LAST_DAY + 400);
    assert_eq!(beyond.code, CalendarCode::Unmeasured);
    assert!(!beyond.known_in_advance);
    // 2018-10-01 is tradable through sourced_kind_of; kind_of alone answers
    // Unmeasured before 2019-12-02.
    let early = day(2018, 10, 1);
    assert_eq!(
        pull::calendar::kind_of(early),
        pull::calendar::DayKind::Unmeasured
    );
    let tradable = calendar.tradable(series, early);
    assert_eq!(tradable.market, Ok(()));
    assert_eq!(tradable.instrument, InstrumentSession::Unchecked);
    assert!(tradable.admits());
    assert_eq!(calendar.tradable(series, day(2025, 3, 3)).market, Ok(()));
    // The rendered reasons.
    assert_eq!(
        budget.to_string(),
        "special_session (Budget weekend session)"
    );
    assert_eq!(halt.to_string(), "halt, known only in hindsight");
    assert_eq!(sunday.to_string(), "closed");
}

#[test]
fn a_length_unmeasured_or_short_session_is_refused_by_its_own_code() {
    // A sourced Muhurat the record has not sized answers OpenLengthUnmeasured
    // through sourced_kind_of; find one in the sourced spans and check the
    // code, so the arm is pinned without typing a date the table may move.
    let mut seen_length = false;
    let mut on = pull::calendar::SOURCED_FIRST_DAY;
    while on <= pull::calendar::SOURCED_LAST_DAY {
        if pull::calendar::special_session(on).is_none()
            && pull::calendar::halt_on(on).is_none()
            && pull::calendar::sourced_kind_of(on) == pull::calendar::DayKind::OpenLengthUnmeasured
        {
            let reason = market_verdict(on).unwrap_err();
            assert_eq!(reason.code, CalendarCode::LengthUnmeasured);
            assert!(!reason.known_in_advance);
            seen_length = true;
        }
        on += 1;
    }
    assert!(seen_length, "the sourced spans hold an unsized session");
    let names: Vec<&str> = CalendarCode::ALL.iter().map(|code| code.name()).collect();
    assert_eq!(
        names,
        [
            "closed",
            "calendar_unmeasured",
            "session_length_unmeasured",
            "not_full_session",
            "special_session",
            "halt",
            "systems_outage"
        ]
    );
    for (at, code) in CalendarCode::ALL.iter().enumerate() {
        assert_eq!(code.index(), at);
    }
}

/// The arms today's calendar never reaches, through the pure formula.
#[test]
fn the_formula_names_a_short_session_and_an_outage_kind() {
    use pull::calendar::{DayKind, Session, Window};
    let short = Session {
        windows: [Window { from: 555, to: 600 }, Window { from: 0, to: 0 }],
        count: 1,
    };
    let reason = classify(DayKind::Open(short), None, None).unwrap_err();
    assert_eq!(reason.code, CalendarCode::NotFullSession);
    assert!(!reason.known_in_advance);
    let outage = classify(
        DayKind::Open(Session::full()),
        Some(SpecialKind::SystemsOutage),
        None,
    )
    .unwrap_err();
    assert_eq!(outage.code, CalendarCode::SpecialSession);
    assert!(!outage.known_in_advance, "an outage is never scheduled");
    assert_eq!(
        classify(
            DayKind::Open(Session::full()),
            None,
            Some(CalendarCode::Halt)
        )
        .unwrap_err()
        .code,
        CalendarCode::Halt
    );
    assert_eq!(classify(DayKind::Open(Session::full()), None, None), Ok(()));
}

#[test]
fn the_calendar_says_whether_the_exchange_traded() {
    let calendar = InterimCalendar::new();
    assert!(calendar.traded(day(2021, 2, 24)), "the outage day traded");
    assert!(calendar.traded(day(2025, 10, 21)), "a Muhurat traded");
    assert!(calendar.traded(day(2025, 3, 3)));
    assert!(!calendar.traded(day(2025, 3, 2)), "a Sunday did not");
    let version = calendar.version();
    assert_eq!(version.kind, AuthorityKind::Calendar);
    assert_eq!(version.word, "interim");
    assert_eq!(version.label, INTERIM_CALENDAR_LABEL);
    assert!(version.interim);
    assert_eq!(*version, InterimCalendar::new().version().clone(), "stable");
}

#[test]
fn year_counts_count_per_year_and_position() {
    let mut counts: YearCounts<3> = YearCounts::new();
    counts.add(2020, 0);
    counts.add(2020, 0);
    counts.add(2021, 2);
    counts.add(2020, 1);
    counts.add(2021, 9);
    assert_eq!(counts.get(2020, 0), 2);
    assert_eq!(counts.get(2020, 1), 1);
    assert_eq!(counts.get(2021, 2), 1);
    assert_eq!(counts.get(2022, 0), 0);
    assert_eq!(counts.get(2020, 9), 0);
    assert_eq!(counts.total(0), 2);
    assert_eq!(counts.total(2), 1);
    assert_eq!(counts.years().len(), 2);
    assert_eq!(counts.years()[0], (2020, [2, 1, 0]));
    assert_eq!(counts.years()[1], (2021, [0, 0, 1]));
    assert_eq!(year_of(day(2024, 12, 31)), Some(2024));
    assert_eq!(year_of(-1), None);
}

fn synthetic_pool(size: u32, stride: u32) -> Vec<Member> {
    // Series ids spread and out of order, as a point-in-time pool's are.
    (0..size)
        .rev()
        .map(|n| Member {
            series_id: SeriesId(n * stride + n % 3),
            symbol_on_day: format!("SYN{n}"),
            zerodha_series_present: n % 2 == 0,
            in_current_list: n % 5 != 0,
        })
        .collect()
}

/// FM-F086: `slot_of` equals a linear search, at three pools spanning 100x.
#[test]
fn slot_of_equals_linear_search() {
    for size in [10, 100, 1_000] {
        let members = synthetic_pool(size, 7);
        let slots = DaySlots::build(&members).unwrap();
        let widest = members.iter().map(|m| m.series_id.0).max().unwrap();
        for id in 0..=widest + 3 {
            let linear = members
                .iter()
                .position(|m| m.series_id.0 == id)
                .map(|at| u16::try_from(at).unwrap());
            assert_eq!(slots.slot_of(SeriesId(id)), linear, "size {size} id {id}");
        }
        assert_eq!(slots.slot_of(SeriesId(u32::MAX)), None);
    }
    let empty = DaySlots::build(&[]).unwrap();
    assert_eq!(empty.slot_of(SeriesId(0)), None);
}

#[test]
fn a_pool_listing_a_series_twice_or_past_a_u16_is_refused() {
    let mut members = synthetic_pool(4, 1);
    members.push(members[0].clone());
    let twice = DaySlots::build(&members).unwrap_err();
    assert!(twice.starts_with("series_listed_twice: series"), "{twice}");
    let wide: Vec<Member> = (0..=u32::from(u16::MAX) + 1)
        .map(|n| Member {
            series_id: SeriesId(n),
            symbol_on_day: String::new(),
            zerodha_series_present: true,
            in_current_list: true,
        })
        .collect();
    let refused = DaySlots::build(&wide).unwrap_err();
    assert!(
        refused.starts_with("pool_exceeds_slots: 65537 members"),
        "{refused}"
    );
}

#[test]
fn the_development_pool_is_the_fno_shares_labelled() {
    let pool = DevelopmentPool::new(&|symbol: &str| symbol.starts_with('A')).unwrap();
    let today = day(2025, 3, 3);
    let members = pool.members(today);
    let shares = brutex_core::universe::FNO_UNDERLYINGS.len()
        - brutex_core::universe::FNO_INDEX_UNDERLYINGS.len();
    assert_eq!(members.len(), shares);
    for (slot, member) in members.iter().enumerate() {
        assert!(
            !brutex_core::universe::FNO_INDEX_UNDERLYINGS.contains(&member.symbol_on_day.as_str())
        );
        assert_eq!(member.series_id.0, u32::try_from(slot).unwrap());
        assert_eq!(
            pool.slot_of(member.series_id, today),
            Some(u16::try_from(slot).unwrap())
        );
        assert_eq!(
            member.zerodha_series_present,
            member.symbol_on_day.starts_with('A')
        );
        assert!(member.in_current_list);
    }
    let mm = pool.member_named("M&M", today).expect("M&M is a member");
    assert_eq!(mm.symbol_on_day, "M&M");
    assert!(pool.member_named("NIFTY", today).is_none());
    let version = pool.version();
    assert_eq!(version.word, "dev");
    assert!(
        version
            .label
            .contains("survivorship-biased development pool")
    );
    assert!(version.label.contains("renames not stitched"));
    assert!(version.interim);
    assert_eq!(development_symbols().count(), shares);
}

#[test]
fn the_interim_answers_are_named_constants() {
    let series = SeriesId(3);
    let on = day(2025, 3, 3);
    let factor = IdentityFactor::new();
    let identity = factor.factor(series, on).unwrap();
    assert_eq!(identity, factor.first_minute(series, on).unwrap());
    assert_eq!((identity.zerodha_close, identity.gdfl_close), (1, 1));
    assert!(!identity.measured, "the identity is never measured");
    assert!(!factor.covers(on));
    assert_eq!(factor.version().label, IDENTITY_FACTOR_LABEL);
    let seconds = UncheckedSeconds::new();
    assert_eq!(seconds.status(series, on), SecondsStatus::Unchecked);
    assert_eq!(seconds.version().word, "unchecked");
    let flow = AbsentOrderFlow::new();
    assert_eq!(flow.window(series, on, 0, 1), OrderFlowWindow::Off);
    assert_eq!(flow.version().word, "absent");
    let landing = SingleLanding::new();
    assert_eq!(landing.landing(series, on), Some(LandingId(0)));
    assert_eq!(landing.basis_break(series, on - 1, on), None);
    let tail = UnwitnessedTail::new();
    assert_eq!(tail.after(series, on, 0), TailReading::Unknown);
    let screen = UnscreenedIndices::new();
    assert_eq!(screen.screen("NIFTY", on), IndexDay::Unscreened);
    let expiry = UnloadedExpiry::new();
    assert_eq!(
        expiry.expiry(series, on),
        ExpiryStamp::Absent("expiry_source_not_loaded")
    );
    assert_eq!(FNO_BAN_ABSENT, "ban list UNVERIFIED");
    for version in [
        seconds.version(),
        flow.version(),
        landing.version(),
        tail.version(),
        screen.version(),
        expiry.version(),
    ] {
        assert!(version.interim);
        assert_eq!(version.word, version.kind.interim_word());
    }
    assert_eq!(IndexDay::Unscreened.absent_reason(), None);
    assert_eq!(IndexDay::Clean.absent_reason(), None);
    assert_eq!(IndexDay::Foreign.absent_reason(), Some("index_day_foreign"));
    assert_eq!(
        IndexDay::Rescaled.absent_reason(),
        Some("index_day_rescaled")
    );
    assert_eq!(IndexDay::Junk.absent_reason(), Some("index_day_junk"));
    assert_eq!(
        InstrumentSession::Unchecked.word(),
        "instrument_session_unchecked"
    );
    assert_eq!(
        InstrumentSession::Admitted.word(),
        "instrument_session_admitted"
    );
    assert_eq!(
        InstrumentSession::Refused("tmpv_rekey").word(),
        "tmpv_rekey"
    );
    let refused = Tradable {
        market: Ok(()),
        instrument: InstrumentSession::Refused("special"),
    };
    assert!(
        !refused.admits(),
        "a per-instrument refusal refuses the stock"
    );
}

fn landings(by_day: &[(i64, u64)], split: Option<(i64, BasisBreak)>) -> Landings {
    Landings {
        version: measured(AuthorityKind::Landing, "landings"),
        by_day: by_day.iter().copied().collect(),
        split,
    }
}

fn factors(close: &[(i64, FactorPair)], first: &[(i64, FactorPair)], from: i64) -> Factors {
    Factors {
        version: measured(AuthorityKind::Factor, "factors"),
        close: close.iter().copied().collect(),
        first: first.iter().copied().collect(),
        from,
    }
}

/// FM-F092 (c) and RQ-4: the historical basis proof.
#[test]
fn the_historical_basis_proof_follows_fm_f092() {
    let series = SeriesId(1);
    let (d1, d2) = (day(2024, 6, 3), day(2024, 6, 4));
    // One landing: proven, landing only when the factor does not cover.
    let one = landings(&[(d1, 5), (d2, 5)], None);
    let none = factors(&[], &[], i64::MAX);
    let verdict = HistoricalBasis::new(&one, &none).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Ok(BasisProof::SameLanding));
    assert!(verdict.landing_only);
    // A synthetic 1:2 split across two landings with no record: unproven.
    let two = landings(&[(d1, 5), (d2, 6)], None);
    let verdict = HistoricalBasis::new(&two, &none).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
    assert_eq!(BasisRefusal::Unproven.name(), "basis_unproven");
    // Covered, two landings, a PREC-10 record: proven by the record.
    let record = BasisBreak {
        numerator: 1,
        denominator: 2,
        source: "synthetic PREC-10 row",
    };
    let covered = factors(&[], &[], d1);
    let recorded = landings(&[(d1, 5), (d2, 6)], Some((d2, record)));
    let verdict = HistoricalBasis::new(&recorded, &covered).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Ok(BasisProof::BreakRecord(record)));
    assert!(!verdict.landing_only);
    // A record with a zero denominator is no record.
    let broken = landings(
        &[(d1, 5), (d2, 6)],
        Some((
            d2,
            BasisBreak {
                denominator: 0,
                ..record
            },
        )),
    );
    let verdict = HistoricalBasis::new(&broken, &covered).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
    // DATA-1's measured pairs equal as a ratio: proven.
    let equal = factors(
        &[(d1, pair(2_000, 1_000, FactorShape::Proportional))],
        &[(d2, pair(4_000, 2_000, FactorShape::Proportional))],
        d1,
    );
    let verdict = HistoricalBasis::new(&two, &equal).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Ok(BasisProof::FactorPairsEqual));
    // Unequal ratios: unproven.
    let unequal = factors(
        &[(d1, pair(2_000, 1_000, FactorShape::Proportional))],
        &[(d2, pair(4_000, 1_000, FactorShape::Proportional))],
        d1,
    );
    let verdict = HistoricalBasis::new(&two, &unequal).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
    // Equal but unmeasured (a factor of one going forward): never proof.
    let raw = FactorPair {
        measured: false,
        ..pair(1, 1, FactorShape::Proportional)
    };
    let unmeasured = factors(&[(d1, raw)], &[(d2, raw)], d1);
    let verdict = HistoricalBasis::new(&two, &unmeasured).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
    // An affine or non-constant day is unreliable, even inside one landing.
    for shape in [FactorShape::Affine, FactorShape::NonConstant] {
        let marked = factors(&[(d2, pair(2_000, 1_000, shape))], &[], d1);
        let verdict = HistoricalBasis::new(&one, &marked).verdict(series, d1, d2);
        assert_eq!(verdict.outcome, Err(BasisRefusal::Unreliable), "{shape:?}");
    }
    assert_eq!(BasisRefusal::Unreliable.name(), "basis_unreliable");
    // A landing missing on either day proves nothing.
    let missing = landings(&[(d2, 5)], None);
    let verdict = HistoricalBasis::new(&missing, &none).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
    let missing_today = landings(&[(d1, 5)], None);
    let verdict = HistoricalBasis::new(&missing_today, &none).verdict(series, d1, d2);
    assert_eq!(verdict.outcome, Err(BasisRefusal::Unproven));
}

#[test]
fn the_basis_verdict_is_interim_when_either_input_is() {
    let measured_landings = landings(&[], None);
    let measured_factors = factors(&[], &[], 0);
    let single = SingleLanding::new();
    let identity = IdentityFactor::new();
    assert!(
        !HistoricalBasis::new(&measured_landings, &measured_factors)
            .version()
            .interim
    );
    assert!(
        HistoricalBasis::new(&single, &measured_factors)
            .version()
            .interim
    );
    assert!(
        HistoricalBasis::new(&measured_landings, &identity)
            .version()
            .interim
    );
    let version = HistoricalBasis::new(&single, &identity).version().clone();
    assert_eq!(version.kind, AuthorityKind::Basis);
    assert_eq!(version.label, HISTORICAL_BASIS_LABEL);
    assert_ne!(
        version.digest,
        HistoricalBasis::new(&measured_landings, &identity)
            .version()
            .digest,
        "the digest binds its inputs"
    );
}

/// FIB-15 4.8 through RQ-4: the forward rule and its latch.
#[test]
fn the_forward_basis_rule_refuses_by_name_and_latches_until_rebuilt() {
    let read = |row_for_day, rebuilt, re_read| CorporateActions::Read {
        row_for_day,
        rebuilt,
        re_read,
    };
    let (d1, d2, d3, d4) = (100, 101, 102, 103);
    let mut stock = ForwardBasis::unlatched();
    assert_eq!(
        stock.judge(d1, read(false, false, Some(ReReadClass::Same))),
        Ok(())
    );
    assert_eq!(stock.latched_since(), None);
    // A measured step with no NSE row refuses, unsourced, and latches.
    assert_eq!(
        stock.judge(d2, read(false, false, Some(ReReadClass::RatioStep))),
        Err(BasisRefusal::Unsourced)
    );
    assert_eq!(stock.latched_since(), Some(d2));
    // A same re-read the next day never clears the latch.
    assert_eq!(
        stock.judge(d3, read(false, false, Some(ReReadClass::Same))),
        Err(BasisRefusal::Latched { since: d2 })
    );
    assert_eq!(
        BasisRefusal::Latched { since: d2 }.name(),
        "basis_unresolved"
    );
    // A sealed rebuild clears it.
    assert_eq!(
        stock.judge(d4, read(false, true, Some(ReReadClass::Same))),
        Ok(())
    );
    assert_eq!(stock.latched_since(), None);
    // Every class, one fresh stock each.
    for (input, expected) in [
        (
            read(false, false, Some(ReReadClass::AffineStep)),
            Err(BasisRefusal::Unsourced),
        ),
        (
            read(false, false, Some(ReReadClass::Revised)),
            Err(BasisRefusal::Differs),
        ),
        (
            read(true, false, Some(ReReadClass::Differs)),
            Err(BasisRefusal::Differs),
        ),
        (read(false, false, None), Err(BasisRefusal::UnprovenForward)),
        (
            read(true, false, Some(ReReadClass::RatioStep)),
            Err(BasisRefusal::RebuildPending),
        ),
        (read(true, true, Some(ReReadClass::RatioStep)), Ok(())),
        (read(true, true, None), Ok(())),
    ] {
        let mut fresh = ForwardBasis::unlatched();
        assert_eq!(fresh.judge(d1, input), expected, "{input:?}");
        assert_eq!(
            fresh.latched_since().is_some(),
            expected.is_err(),
            "{input:?}"
        );
    }
    // No source and an unread list never latch.
    let mut fresh = ForwardBasis::unlatched();
    assert_eq!(
        fresh.judge(d1, CorporateActions::NoSource),
        Err(BasisRefusal::SourceMissing)
    );
    assert_eq!(
        fresh.judge(d1, CorporateActions::Unread),
        Err(BasisRefusal::Pending)
    );
    assert_eq!(fresh.latched_since(), None);
    let names: Vec<&str> = [
        BasisRefusal::UnprovenForward,
        BasisRefusal::Unsourced,
        BasisRefusal::Differs,
        BasisRefusal::RebuildPending,
        BasisRefusal::SourceMissing,
        BasisRefusal::Pending,
    ]
    .iter()
    .map(|refusal| refusal.name())
    .collect();
    assert_eq!(
        names,
        [
            "basis_unproven",
            "basis_unsourced",
            "basis_differs",
            "basis_rebuild_pending",
            "basis_source_missing",
            "corporate_actions_unread"
        ]
    );
}

#[test]
fn every_word_selects_by_name_and_new_work_is_refused_until_fib_10() {
    for kind in AuthorityKind::ALL {
        assert_eq!(select(*kind, kind.interim_word()), Ok(Choice::Interim));
        if let Some(word) = kind.new_work_word() {
            assert_eq!(
                select(*kind, word),
                Err(format!(
                    "authority `{word}` is not built yet: it arrives with FIB-10"
                ))
            );
        }
        let other = select(*kind, "maybe").unwrap_err();
        assert!(
            other.starts_with(&format!("`maybe` names no {} authority", kind.name())),
            "{other}"
        );
        assert!(other.contains(kind.interim_word()));
    }
    // Another kind's new-work word reaches the same refusal.
    assert_eq!(
        select(AuthorityKind::Expiry, "pit"),
        Err("authority `pit` is not built yet: it arrives with FIB-10".to_owned())
    );
    let calendar = select(AuthorityKind::Calendar, "x").unwrap_err();
    assert!(
        calendar.ends_with("the words are interim and cal1 (FIB-10)"),
        "{calendar}"
    );
    let basis = select(AuthorityKind::Basis, "x").unwrap_err();
    assert!(basis.ends_with("the words are fm-f092"), "{basis}");
    let flags: Vec<&str> = AuthorityKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(
        flags,
        [
            "calendar",
            "pool",
            "factor",
            "seconds",
            "order-flow",
            "landing",
            "basis",
            "tail",
            "screen",
            "expiry"
        ]
    );
}

#[test]
fn no_authority_is_chosen_for_a_caller_who_named_none() {
    let all = words_of(&["--authorities", "interim"]).unwrap();
    assert_eq!(all.len(), AuthorityKind::ALL.len());
    for ((kind, word), expected) in all.iter().zip(AuthorityKind::ALL) {
        assert_eq!(kind, expected);
        assert_eq!(word, kind.interim_word());
    }
    let none = words_of(&[]).unwrap_err();
    assert!(none.starts_with("authorities_required: "), "{none}");
    for kind in AuthorityKind::ALL {
        assert!(none.contains(&format!("--{} {}", kind.name(), kind.interim_word())));
    }
    let mut named: Vec<String> = Vec::new();
    for kind in AuthorityKind::ALL {
        named.push(format!("--{}", kind.name()));
        named.push(kind.interim_word().to_owned());
    }
    let words: Vec<&str> = named.iter().map(String::as_str).collect();
    assert_eq!(words_of(&words).unwrap(), all);
    let partial = words_of(&["--calendar", "interim"]).unwrap_err();
    assert!(partial.contains("--pool dev"), "{partial}");
    assert!(!partial.contains("--calendar interim"), "{partial}");
    let set = words_of(&["--authorities", "everything"]).unwrap_err();
    assert!(
        set.starts_with("`--authorities everything` is not a set"),
        "{set}"
    );
    let new_work = words_of(&["--authorities", "interim", "--pool", "pit"]).unwrap_err();
    assert!(new_work.contains("arrives with FIB-10"), "{new_work}");
}

/// Every interim authority is refused for `--out-store`, by name.
#[test]
fn out_store_is_refused_with_every_interim_authority() {
    let set = Authorities::interim(&|_: &str| true).unwrap();
    let versions = set.versions();
    assert_eq!(versions.len(), AuthorityKind::ALL.len());
    for (version, kind) in versions.iter().zip(AuthorityKind::ALL) {
        assert_eq!(version.kind, *kind);
        assert!(version.interim, "{}", kind.name());
    }
    let refused = set.refuse_for_out_store().unwrap_err();
    assert!(
        refused.starts_with("interim_authority_for_out_store: "),
        "{refused}"
    );
    for version in &versions {
        assert!(
            refused.contains(&format!(
                "{} = {} ({})",
                version.kind.name(),
                version.word,
                version.label
            )),
            "{refused}"
        );
    }
}

struct Measured<T> {
    version: AuthorityVersion,
    inner: T,
}

impl<T: CalendarAuthority> CalendarAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn tradable(&self, series: SeriesId, day: i64) -> Tradable {
        self.inner.tradable(series, day)
    }
    fn market(&self, day: i64) -> Result<(), CalendarReason> {
        self.inner.market(day)
    }
    fn traded(&self, day: i64) -> bool {
        self.inner.traded(day)
    }
}

impl<T: PoolAuthority> PoolAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn members(&self, day: i64) -> &[Member] {
        self.inner.members(day)
    }
    fn slot_of(&self, series: SeriesId, day: i64) -> Option<u16> {
        self.inner.slot_of(series, day)
    }
}

impl<T: FactorAuthority> FactorAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn factor(&self, series: SeriesId, day: i64) -> Result<FactorPair, String> {
        self.inner.factor(series, day)
    }
    fn first_minute(&self, series: SeriesId, day: i64) -> Result<FactorPair, String> {
        self.inner.first_minute(series, day)
    }
    fn covers(&self, day: i64) -> bool {
        self.inner.covers(day)
    }
}

impl<T: SecondsTrust> SecondsTrust for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn status(&self, series: SeriesId, day: i64) -> SecondsStatus {
        self.inner.status(series, day)
    }
}

impl<T: OrderFlowAuthority> OrderFlowAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn window(&self, series: SeriesId, day: i64, from: u32, to: u32) -> OrderFlowWindow {
        self.inner.window(series, day, from, to)
    }
}

impl<T: LandingAuthority> LandingAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn landing(&self, series: SeriesId, day: i64) -> Option<LandingId> {
        self.inner.landing(series, day)
    }
    fn basis_break(&self, series: SeriesId, from: i64, day: i64) -> Option<BasisBreak> {
        self.inner.basis_break(series, from, day)
    }
}

impl<T: TailWitness> TailWitness for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn after(&self, series: SeriesId, day: i64, last: u32) -> TailReading {
        self.inner.after(series, day, last)
    }
}

impl<T: IndexScreen> IndexScreen for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn screen(&self, index: &str, day: i64) -> IndexDay {
        self.inner.screen(index, day)
    }
}

impl<T: ExpiryAuthority> ExpiryAuthority for Measured<T> {
    fn version(&self) -> &AuthorityVersion {
        &self.version
    }
    fn expiry(&self, series: SeriesId, day: i64) -> ExpiryStamp {
        self.inner.expiry(series, day)
    }
}

fn wrap<T>(kind: AuthorityKind, inner: T) -> Measured<T> {
    Measured {
        version: measured(kind, "a measured stand-in for the test"),
        inner,
    }
}

#[test]
fn a_set_with_no_interim_authority_may_write_the_store() {
    let set = Authorities {
        calendar: Box::new(wrap(AuthorityKind::Calendar, InterimCalendar::new())),
        pool: Box::new(wrap(
            AuthorityKind::Pool,
            DevelopmentPool::new(&|_: &str| false).unwrap(),
        )),
        factor: Box::new(wrap(AuthorityKind::Factor, IdentityFactor::new())),
        seconds: Box::new(wrap(AuthorityKind::SecondsTrust, UncheckedSeconds::new())),
        order_flow: Box::new(wrap(AuthorityKind::OrderFlow, AbsentOrderFlow::new())),
        landing: Box::new(wrap(AuthorityKind::Landing, SingleLanding::new())),
        tail: Box::new(wrap(AuthorityKind::TailWitness, UnwitnessedTail::new())),
        screen: Box::new(wrap(AuthorityKind::IndexScreen, UnscreenedIndices::new())),
        expiry: Box::new(wrap(AuthorityKind::Expiry, UnloadedExpiry::new())),
    };
    assert_eq!(set.refuse_for_out_store(), Ok(()));
    assert!(set.versions().iter().all(|version| !version.interim));
    // The wrappers answer as what they wrap.
    let on = day(2025, 3, 3);
    let series = SeriesId(0);
    assert!(set.calendar.tradable(series, on).admits());
    assert!(set.calendar.traded(on));
    assert_eq!(set.calendar.market(on), Ok(()));
    assert_eq!(set.pool.slot_of(series, on), Some(0));
    assert!(!set.pool.members(on).is_empty());
    assert!(set.factor.factor(series, on).is_ok());
    assert!(set.factor.first_minute(series, on).is_ok());
    assert!(!set.factor.covers(on));
    assert_eq!(set.seconds.status(series, on), SecondsStatus::Unchecked);
    assert_eq!(
        set.order_flow.window(series, on, 0, 1),
        OrderFlowWindow::Off
    );
    assert_eq!(set.landing.landing(series, on), Some(LandingId(0)));
    assert_eq!(set.landing.basis_break(series, on, on), None);
    assert_eq!(set.tail.after(series, on, 0), TailReading::Unknown);
    assert_eq!(set.screen.screen("NIFTY", on), IndexDay::Unscreened);
    assert!(matches!(
        set.expiry.expiry(series, on),
        ExpiryStamp::Absent(_)
    ));
}

#[test]
fn a_version_digest_binds_every_part() {
    let base = version_digest(AuthorityKind::Pool, "dev", "label", b"table");
    assert_ne!(
        base,
        version_digest(AuthorityKind::Factor, "dev", "label", b"table")
    );
    assert_ne!(
        base,
        version_digest(AuthorityKind::Pool, "pit", "label", b"table")
    );
    assert_ne!(
        base,
        version_digest(AuthorityKind::Pool, "dev", "other", b"table")
    );
    assert_ne!(
        base,
        version_digest(AuthorityKind::Pool, "dev", "label", b"tablf")
    );
    // Framed: moving a byte across a boundary changes the digest.
    assert_ne!(
        version_digest(AuthorityKind::Pool, "de", "vlabel", b"table"),
        base
    );
    assert_eq!(
        base,
        version_digest(AuthorityKind::Pool, "dev", "label", b"table")
    );
}

/// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): the shipping part of this
/// file holds no numeric literal but 0 and 1.
#[test]
fn authorities_rs_types_no_number() {
    let source = include_str!("authorities.rs");
    let typed = vocab::fib_cube::untyped_numbers(source);
    assert!(typed.is_empty(), "{typed:?}");
    assert!(vocab::fib_cube::shipping_part(source).len() < source.len());
}
