use super::*;

#[test]
fn side_helpers() {
    assert_eq!(Side::Long.name(), "long");
    assert_eq!(Side::Short.name(), "short");
    assert!(Side::Long.long() && !Side::Short.long());
    assert_eq!(Side::Long.mirror(), Side::Short);
    assert_eq!(Side::Short.mirror(), Side::Long);
    assert_eq!(stop_toward(Side::Long), Toward::Down);
    assert_eq!(stop_toward(Side::Short), Toward::Up);
    assert_eq!(target_toward(Side::Long), Toward::Up);
    assert_eq!(target_toward(Side::Short), Toward::Down);
}

#[test]
fn every_status_has_a_distinct_name() {
    let mut names: Vec<&str> = Status::ALL.iter().map(|s| s.name()).collect();
    let n = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), n);
    assert_eq!(Status::DayCap.name(), "day cap");
    assert_eq!(Status::MonthCap.name(), "month cap");
}

#[test]
fn readings_label_names_each_departure() {
    assert_eq!(Readings::HEADLINE.label(), "headline");
    let mut r = Readings::HEADLINE;
    r.ties = ExtremeTies::Latest;
    r.interim = InterimLine::NoCanPrint;
    assert_eq!(r.label(), "A02=latest,A19=no_can_print");
    let all = Readings {
        ties: ExtremeTies::Latest,
        grid: CandleGrid::Clock,
        taint: TaintScope::ExtremeCandles,
        mover_sign: MoverSign::SignedOnly,
        after_break: AfterBreak::NextBox,
        box_anchor: BoxAnchor::Anchored,
        retouch: Retouch::Ignore,
        orb_boundary: OrbBoundary::Arms,
        within_second: WithinSecond::ExitFirst,
        resting_after_cut: RestingAfterCut::Rejected,
        zero_fill_count: ZeroFillCount::Kept,
        below_half: BelowHalf::Refused,
        dd_round: DdRound::Ceil,
        data_charge_at: DataChargeAt::OutOfCurve,
        state_stamp: StateStamp::Literal,
        option_brokerage: OptionBrokerage::Min20,
        target_ladder: TargetLadder::Judging,
        lock_end: LockEnd::Exclusive,
        interim: InterimLine::NoCanPrint,
        rule_order: RuleOrder::SixFirst,
    };
    assert_eq!(
        all.label(),
        "A01=signed_only,A02=latest,A03=extreme_candles,A04=clock,A05=next_box,A06=anchored,\
         A07=ignore,A08=arms,A09=exit_first,A10=rejected,A11=kept,A12=refused,A13=ceil,\
         A14=out_of_curve,A15=literal,A16=min20,A17=judging,A18=exclusive,A19=no_can_print,\
         A20=six_first"
    );
}

#[test]
fn map_level_rounds_against_the_position() {
    // SNIPER.md 1.2's long vector: Z 100000, G 100020, stop_Z 99695
    let floor = map_level(99_695_000, 100_000, 100_020, 1, Toward::Down);
    assert_eq!(floor, 99_714);
    let ceil = map_level(99_695_000, 100_000, 100_020, 1, Toward::Up);
    assert_eq!(ceil, 99_715);
    // to a 5-paise step
    assert_eq!(
        map_level(99_695_000, 100_000, 100_020, 5, Toward::Down),
        99_710
    );
    assert_eq!(
        map_level(99_695_000, 100_000, 100_020, 5, Toward::Up),
        99_715
    );
    // an exact level stays put either way
    assert_eq!(
        map_level(100_000_000, 100_000, 100_000, 5, Toward::Up),
        100_000
    );
    assert_eq!(
        map_level(100_000_000, 100_000, 100_000, 5, Toward::Down),
        100_000
    );
    // a zero or negative step reads as one paisa
    assert_eq!(
        map_level(100_001_000, 100_000, 100_000, 0, Toward::Down),
        100_001
    );
    // a fractional milli-paise level
    assert_eq!(
        map_level(100_000_400, 100_000, 100_000, 1, Toward::Up),
        100_001
    );
    assert_eq!(
        map_level(100_000_400, 100_000, 100_000, 1, Toward::Down),
        100_000
    );
}

#[test]
fn r1_at_reads_the_last_closed_minute() {
    let minute = |m: u32, c: i64, g: Option<i64>| Minute {
        m,
        o: c,
        h: c,
        l: c,
        c,
        v: 1,
        held: true,
        padded: false,
        g,
    };
    let minutes = [minute(0, 100, Some(101)), minute(1, 102, None)];
    let open = crate::clock::OPEN;
    assert_eq!(r1_at(&minutes, open + 30), None); // nothing closed yet
    assert_eq!(r1_at(&minutes, open + 60), Some((100, 101)));
    assert_eq!(r1_at(&minutes, open + 119), Some((100, 101)));
    assert_eq!(r1_at(&minutes, open + 120), None); // no GDFL trade in minute 1
    assert_eq!(r1_at(&minutes, open + 600), None); // minute 9 not in the data
}
