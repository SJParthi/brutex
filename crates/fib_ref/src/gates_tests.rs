use super::*;

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the six worked vectors, each checked"
)]
fn sniper_md_worked_vectors() {
    // (side, Z, G, REF, stop_Z paise, target_Z paise or none) -> (D_stop, S-STOP, D_tgt, S-RR price)
    type Row = (
        Side,
        i64,
        i64,
        i64,
        i64,
        Option<i64>,
        i128,
        Result<i128, Status>,
        Option<i128>,
        Result<i128, Status>,
    );
    let rows: [Row; 6] = [
        (
            Side::Long,
            100_000,
            100_020,
            100_020,
            99_695,
            Some(100_605),
            306,
            Ok(3_060),
            Some(605),
            Ok(1_977),
        ),
        (
            Side::Short,
            100_000,
            99_980,
            99_980,
            100_305,
            Some(99_395),
            305,
            Ok(3_051),
            Some(604),
            Ok(1_980),
        ),
        (
            Side::Long,
            100_000,
            100_000,
            99_200,
            99_695,
            Some(100_605),
            -495,
            Err(Status::SnStopThroughAtDecision),
            Some(1_405),
            Err(Status::SnStopThroughAtDecision),
        ),
        (
            Side::Long,
            100_000,
            100_020,
            100_020,
            99_695,
            Some(99_900),
            306,
            Ok(3_060),
            Some(0),
            Ok(0),
        ),
        (
            Side::Long,
            100_000,
            100_000,
            100_000,
            99_999,
            Some(3_100_000),
            1,
            Ok(10),
            Some(3_000_000),
            Ok(3_000_000_000),
        ),
        (
            Side::Long,
            100_000,
            100_000,
            100_000,
            99_990,
            Some(100_020),
            10,
            Ok(100),
            Some(20),
            Ok(2_000),
        ),
    ];
    for (side, z, g, r, stop, tgt, d_stop, sstop, d_tgt, srr) in rows {
        let (ds, dt) = distances(
            side,
            z,
            g,
            r,
            i128::from(stop) * 1000,
            tgt.map(|t| i128::from(t) * 1000),
        );
        assert_eq!(ds, d_stop);
        assert_eq!(dt, d_tgt);
        assert_eq!(s_stop(ds, r), sstop);
        assert_eq!(s_rr_price(ds, dt), srr);
    }
    assert_eq!(s_rr_price(10, None), Err(Status::SRrNoTarget));
}

#[test]
fn the_24_13_vectors_with_their_own_charges() {
    // (qty, D_stop, D_tgt, charges) -> S-RR
    for (q, ds, dt, ch, want) in [
        (1_000, 10, 50, 41_677, 161),
        (1_000, 10, 20, 41_677, -420),
        (250, 10, 50, 15_167, -151),
        (999, 306, 605, 41_671, 1_619),
        (999, 306, 0, 41_671, -120),
        (1_000, 2_500, 12_500, 41_677, 4_901),
    ] {
        assert_eq!(
            s_rr_net(ds, Some(dt), q, ch),
            Ok(want),
            "{q} {ds} {dt} {ch}"
        );
    }
    // charges 0 equal the price reading
    assert_eq!(s_rr_net(306, Some(605), 999, 0), s_rr_price(306, Some(605)));
    assert_eq!(
        s_rr_net(0, Some(5), 10, 0),
        Err(Status::SnStopThroughAtDecision)
    );
    assert_eq!(s_rr_net(5, Some(5), 0, 0), Err(Status::SnNoQuantity));
    assert_eq!(s_rr_net(5, None, 10, 0), Err(Status::SRrNoTarget));
    // D_stop = 1, a huge charge: floor of a large negative ratio
    assert_eq!(s_rr_net(1, Some(0), 1, 1_000_000), Ok(-1000));
}

#[test]
fn those_charges_are_the_worst_reading_and_the_official_one_differs() {
    let d = Date {
        y: 2026,
        m: 9,
        d: 9,
    };
    let r = Readings::HEADLINE;
    assert_eq!(planned_charges_worst(d, 1_000, 100_000, &r), 41_677);
    assert_eq!(planned_charges_worst(d, 250, 100_000, &r), 15_167);
    assert_eq!(planned_charges_worst(d, 999, 100_020, &r), 41_671);
    assert_eq!(planned_charges(d, 1_000, 100_000, &r), 40_201);
    assert_eq!(planned_charges(d, 250, 100_000, &r), 13_690);
    assert_eq!(planned_charges(d, 999, 100_020, &r), 40_195);
    // the six S-RR values on the headline charges
    let p = |q: i128, ds: i128, dt: i128, refpx: i64| {
        s_rr_net(ds, Some(dt), q, planned_charges(d, q, refpx, &r))
    };
    assert_eq!(p(1_000, 10, 50, 100_000), Ok(195));
    assert_eq!(p(1_000, 10, 20, 100_000), Ok(-403));
    assert_eq!(p(250, 10, 50, 100_000), Ok(-74));
    assert_eq!(p(999, 306, 605, 100_020), Ok(1_631));
    assert_eq!(p(999, 306, 0, 100_020), Ok(-117));
    assert_eq!(p(1_000, 2_500, 12_500, 100_000), Ok(4_905));
    assert_eq!(planned_qty(100_000_000, 100_000), 1_000);
    assert_eq!(planned_qty(25_000_000, 100_020), 249);
    assert_eq!(planned_qty(5, 100_000), 0);
    assert_eq!(planned_qty(5, 0), 5);
}

#[test]
fn map_gate_floors_a_long_and_ceils_a_short() {
    assert_eq!(map_gate(99_695_000, 100_000, 100_020, Side::Long), 99_714);
    assert_eq!(map_gate(99_695_000, 100_000, 100_020, Side::Short), 99_715);
    assert_eq!(
        map_gate(100_000_000, 100_000, 100_000, Side::Short),
        100_000
    );
    assert_eq!(s_stop(1, 0), Ok(1_000_000)); // REF read as at least one paisa
}
