#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! The disk projection's tests (PERF-13, R4-07). Every number is invented,
//! except in the one `#[ignore]`d measurement at the end, which reads sizes the
//! operator's run measured from the environment and is never run by default.

use super::*;

/// A meter that answers what it was built with.
struct Fixed(Result<Space, String>);

impl FreeSpace for Fixed {
    fn space(&self, _path: &Path) -> Result<Space, String> {
        self.0.clone()
    }
}

fn kind(feed: &str, k: &str, rung: &str) -> Unit {
    Unit::Kind(KindKey::new(feed, k, rung))
}

#[test]
fn the_projection_is_measured_bytes_per_day_times_days_left() {
    let mut totals = Totals::new();
    // 1,000 bytes over 3 instrument-days: 334 a day, rounded up.
    totals
        .add_kind(KindKey::new("gdfl", "index", "1s"), 1_000, 3)
        .unwrap();
    totals
        .add_kind(KindKey::new("zerodha", "stock", "1min"), 600, 6)
        .unwrap();
    let sealed = totals.seal();
    assert_eq!(sealed.len(), 2);
    assert!(!sealed.is_empty());
    let plan = [
        PlanItem {
            unit: kind("gdfl", "index", "1s"),
            days: 30,
            instruments: 2,
        },
        PlanItem {
            unit: kind("zerodha", "stock", "1min"),
            days: 10,
            instruments: 213,
        },
    ];
    let projection = project(&sealed, &plan);
    assert_eq!(projection.lines[0].per_day, 334);
    assert_eq!(projection.lines[0].bytes, 30 * 2 * 334);
    assert_eq!(
        projection.lines[0].basis,
        Basis::Measured(Measured {
            bytes: 1_000,
            days: 3
        })
    );
    assert_eq!(projection.lines[1].bytes, 10 * 213 * 100);
    assert_eq!(projection.bytes, 30 * 2 * 334 + 10 * 213 * 100);
    assert!(projection.extrapolated.is_empty());
    assert!(projection.unmeasured.is_empty());
    assert_eq!(
        projection.summary(),
        format!("projected {} bytes over 2 lines", projection.bytes)
    );
    // An empty plan projects nothing, and says so.
    let none = project(&sealed, &[]);
    assert_eq!(none.bytes, 0);
    assert!(none.lines.is_empty());
}

#[test]
fn an_unstored_underlying_is_projected_from_its_kinds_median_and_named() {
    let mut totals = Totals::new();
    // Index options held: 100, 300 and 200 a day (median 200).
    totals
        .add_underlying("NIFTY", OptionClass::Index, 1_000, 10)
        .unwrap();
    totals
        .add_underlying("BANKNIFTY", OptionClass::Index, 3_000, 10)
        .unwrap();
    totals
        .add_underlying("SENSEXX", OptionClass::Index, 2_000, 10)
        .unwrap();
    // Stock options held: 10 and 21 a day (median of two: 16, rounded up).
    totals
        .add_underlying("AAA", OptionClass::Stock, 100, 10)
        .unwrap();
    totals
        .add_underlying("BBB", OptionClass::Stock, 210, 10)
        .unwrap();
    let sealed = totals.seal();
    assert_eq!(sealed.median_of(OptionClass::Index), Some((200, 3)));
    assert_eq!(sealed.median_of(OptionClass::Stock), Some((16, 2)));
    let plan = [
        PlanItem {
            unit: Unit::Underlying {
                name: "NIFTY".into(),
                class: OptionClass::Index,
            },
            days: 5,
            instruments: 1,
        },
        PlanItem {
            unit: Unit::Underlying {
                name: "FINNIFTY".into(),
                class: OptionClass::Index,
            },
            days: 7,
            instruments: 1,
        },
        PlanItem {
            unit: Unit::Underlying {
                name: "CCC".into(),
                class: OptionClass::Stock,
            },
            days: 2,
            instruments: 1,
        },
    ];
    let projection = project(&sealed, &plan);
    assert_eq!(projection.lines[0].bytes, 5 * 100);
    assert_eq!(projection.lines[1].bytes, 7 * 200);
    assert_eq!(
        projection.lines[1].basis,
        Basis::Extrapolated {
            class: OptionClass::Index,
            median_of: 3
        }
    );
    assert_eq!(projection.lines[2].bytes, 2 * 16);
    assert_eq!(
        projection.extrapolated,
        vec![
            "FINNIFTY (index_options)".to_owned(),
            "CCC (stock_options)".to_owned()
        ]
    );
    let words = projection.summary();
    assert!(words.contains("EXTRAPOLATION"), "{words}");
    assert!(words.contains("FINNIFTY (index_options)"), "{words}");
}

#[test]
fn a_unit_nothing_measures_is_not_known_and_refuses_the_plan_by_name() {
    // No stock option is held, so a stock underlying has no median.
    let mut totals = Totals::new();
    totals
        .add_underlying("NIFTY", OptionClass::Index, 1_000, 10)
        .unwrap();
    let sealed = totals.seal();
    assert_eq!(sealed.median_of(OptionClass::Stock), None);
    let plan = [
        PlanItem {
            unit: Unit::Underlying {
                name: "DDD".into(),
                class: OptionClass::Stock,
            },
            days: 3,
            instruments: 1,
        },
        PlanItem {
            unit: kind("gdfl", "never", "1s"),
            days: 3,
            instruments: 1,
        },
    ];
    let projection = project(&sealed, &plan);
    assert_eq!(projection.bytes, 0);
    assert_eq!(projection.lines[0].basis, Basis::Unmeasured);
    assert_eq!(
        projection.unmeasured,
        vec!["DDD (stock_options)".to_owned(), "gdfl/never/1s".to_owned()]
    );
    assert!(projection.summary().contains("NOT KNOWN"));
    let roomy = Fixed(Ok(Space {
        total: 1_000_000,
        free: 1_000_000,
    }));
    let refused = check_plan(&roomy, Path::new("/"), &projection, DEFAULT_FLOOR).unwrap_err();
    assert_eq!(refused.code(), "projection_unknown");
    assert!(refused.to_string().contains("DDD (stock_options)"));
}

#[test]
fn a_plan_past_the_floor_is_refused_before_any_byte() {
    let space = Space {
        total: 1_000,
        free: 300,
    };
    // 15% of 1,000 is 150; 150 + 151 = 301 > 300.
    let refused = floor_check(space, 151, Floor::Percent(15)).unwrap_err();
    assert_eq!(
        refused,
        FloorRefusal::DiskFloor {
            space,
            floor_bytes: 150,
            projected: 151,
            short_by: 1
        }
    );
    assert_eq!(refused.code(), "disk_floor");
    assert!(refused.to_string().contains("nothing was read or written"));
    // Exactly at the floor passes and says what is left.
    let ok = floor_check(space, 150, Floor::Percent(15)).unwrap();
    assert_eq!(ok.left_after, 150);
    assert_eq!(ok.floor_bytes, 150);
    // A byte floor is a byte floor.
    assert!(floor_check(space, 0, Floor::Bytes(301)).is_err());
    assert!(floor_check(space, 0, Floor::Bytes(300)).is_ok());
    // A full disk refuses even an empty plan under any floor.
    let full = Space {
        total: 1_000,
        free: 0,
    };
    assert_eq!(
        floor_check(full, 0, Floor::Percent(1)).unwrap_err().code(),
        "disk_floor"
    );
    // Saturation: a projection near u64::MAX is refused, never wrapped.
    assert!(floor_check(space, u64::MAX, Floor::Percent(1)).is_err());
}

#[test]
fn check_plan_measures_the_disk_and_an_unmeasurable_disk_refuses() {
    let projection = Projection {
        bytes: 10,
        ..Projection::default()
    };
    let roomy = Fixed(Ok(Space {
        total: 1_000,
        free: 900,
    }));
    let ok = check_plan(&roomy, Path::new("/"), &projection, Floor::Percent(15)).unwrap();
    assert_eq!(ok.projected, 10);
    assert_eq!(ok.left_after, 890);
    let lost = Fixed(Err("the drive is gone".into()));
    let refused = check_plan(&lost, Path::new("/"), &projection, Floor::Percent(15)).unwrap_err();
    assert_eq!(refused.code(), "free_space_unknown");
    assert!(refused.to_string().contains("the drive is gone"));
}

#[test]
fn a_sample_with_no_day_or_measured_twice_is_refused_by_name() {
    let mut totals = Totals::new();
    let none = totals
        .add_kind(KindKey::new("gdfl", "index", "1s"), 5, 0)
        .unwrap_err();
    assert_eq!(none.code(), "sample_holds_no_day");
    assert!(none.to_string().contains("gdfl/index/1s"));
    totals
        .add_kind(KindKey::new("gdfl", "index", "1s"), 5, 1)
        .unwrap();
    let twice = totals
        .add_kind(KindKey::new("gdfl", "index", "1s"), 9, 1)
        .unwrap_err();
    assert_eq!(twice.code(), "sample_duplicate");
    assert!(twice.to_string().contains("measured twice"));
    assert_eq!(
        totals
            .add_underlying("X", OptionClass::Stock, 1, 0)
            .unwrap_err()
            .code(),
        "sample_holds_no_day"
    );
    totals
        .add_underlying("X", OptionClass::Stock, 1, 1)
        .unwrap();
    assert_eq!(
        totals
            .add_underlying("X", OptionClass::Index, 1, 1)
            .unwrap_err()
            .code(),
        "sample_duplicate"
    );
    // The first measurement is the one kept.
    let sealed = totals.seal();
    assert_eq!(sealed.per_day(&kind("gdfl", "index", "1s")).0, 5);
    assert!(Totals::new().seal().is_empty());
}

#[test]
fn the_median_takes_the_middle_and_rounds_an_even_pair_up() {
    assert_eq!(median(&[]), None);
    assert_eq!(median(&[7]), Some(7));
    assert_eq!(median(&[1, 2]), Some(2));
    assert_eq!(median(&[2, 2]), Some(2));
    assert_eq!(median(&[1, 4, 9]), Some(4));
    assert_eq!(median(&[0, u64::MAX]), Some(u64::MAX / 2 + 1));
}

#[test]
fn the_floor_has_one_grammar_and_rounds_a_percent_up() {
    use store::settings::disk_floor;
    assert_eq!(disk_floor("15%"), Ok(Floor::Percent(15)));
    assert_eq!(disk_floor(" 1% "), Ok(Floor::Percent(1)));
    assert_eq!(disk_floor("95%"), Ok(Floor::Percent(95)));
    assert_eq!(disk_floor("1TB"), Ok(Floor::Bytes(1_000_000_000_000)));
    assert_eq!(disk_floor("2GiB"), Ok(Floor::Bytes(2 << 30)));
    assert_eq!(disk_floor("1TiB"), Ok(Floor::Bytes(1 << 40)));
    assert_eq!(disk_floor("3GB"), Ok(Floor::Bytes(3_000_000_000)));
    assert_eq!(
        disk_floor("1000000000"),
        Ok(Floor::Bytes(store::settings::MIN_FLOOR_BYTES))
    );
    for bad in [
        "4096",
        "15",
        "0%",
        "96%",
        "100%",
        "05%",
        "+5%",
        "x%",
        "",
        "0",
        "GiB",
        "12XB",
        "007",
        "-1",
        "99999999999999999999",
        "20000000TiB",
    ] {
        assert!(disk_floor(bad).is_err(), "{bad:?} must be refused");
    }
    assert_eq!(Floor::Percent(15).to_string(), "15%");
    assert_eq!(Floor::Bytes(9).to_string(), "9");
    // 15% of 101 is 15.15: rounded up to 16, never down.
    assert_eq!(Floor::Percent(15).bytes_of(101), 16);
    assert_eq!(Floor::Percent(15).bytes_of(100), 15);
    assert_eq!(
        Floor::Percent(95).bytes_of(u64::MAX),
        u64::MAX / 100 * 95 + 15
    );
    assert_eq!(Floor::Bytes(7).bytes_of(0), 7);
    assert_eq!(floor_of(None).unwrap().0, DEFAULT_FLOOR);
    assert_eq!(
        floor_of(Some("1TB")).unwrap().0,
        Floor::Bytes(1_000_000_000_000)
    );
    assert!(
        floor_of(Some("lots")).is_err(),
        "a malformed floor is never the default"
    );
}

#[test]
fn df_output_is_read_by_its_capacity_column() {
    let text = "Filesystem 1024-blocks Used Available Capacity Mounted on\n\
                /dev/disk7s1 7813821744 6042852884 1770133020 78% /Volumes/WD BLACK\n";
    assert_eq!(
        parse_df(text),
        Ok(Space {
            total: 7_813_821_744 * 1024,
            free: 1_770_133_020 * 1024
        })
    );
    assert!(parse_df("").is_err());
    assert!(parse_df("Filesystem\n/dev/x 1 2 3 nope /\n").is_err());
    assert!(parse_df("a b 9%\n").is_err());
    assert!(parse_df("/dev/x one 2 3 4% /\n").is_err());
    // The live meter reads the root filesystem of this machine.
    let measured = Df.space(Path::new("/")).unwrap();
    assert!(measured.total >= measured.free && measured.total > 0);
    assert!(Df.space(Path::new("/no/such/brutex/dir")).is_err());
}

#[test]
fn within_is_integer_and_symmetric_about_the_actual() {
    assert!(within(110, 100, 10));
    assert!(within(90, 100, 10));
    assert!(!within(111, 100, 10));
    assert!(!within(89, 100, 10));
    assert!(within(0, 0, 0));
    assert!(!within(1, 0, 50));
    assert!(within(u64::MAX, u64::MAX, 0));
}

/// The real-data check (R4-07, NIFTY first): sizes measured on the
/// operator's drive by the caller (a delivery container's bytes over the day
/// zips its central directory lists; an option underlying's stored bytes over
/// the days it holds), projected through this module, and held to the floor
/// against `df -Pk` of the real store's drive. Read only: it opens no store
/// file and writes nothing. Run with
/// `BRUTEX_REAL_SAMPLE="<feed>/<kind>/<rung>=<bytes>:<days>;U:<name>@index|stock=<bytes>:<days>;..."`,
/// `BRUTEX_REAL_PLAN="<feed>/<kind>/<rung>=<days>;U:<name>@index|stock=<days>;..."` and
/// `BRUTEX_REAL_STORE=<the store root>`.
#[test]
#[ignore = "reads the operator's measured sizes and the real drive's df; run by hand"]
fn real_delivery_sizes_project_through_the_module_and_meet_the_floor() {
    let read = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    let unit_of = |text: &str| {
        if let Some(rest) = text.strip_prefix("U:") {
            let (name, class) = rest.split_once('@').unwrap();
            let class = if class == "index" {
                OptionClass::Index
            } else {
                OptionClass::Stock
            };
            Unit::Underlying {
                name: name.to_owned(),
                class,
            }
        } else {
            let mut words = text.split('/');
            let (feed, k, rung) = (
                words.next().unwrap(),
                words.next().unwrap(),
                words.next().unwrap(),
            );
            Unit::Kind(KindKey::new(feed, k, rung))
        }
    };
    let items = |name: &str| -> Vec<(Unit, String)> {
        read(name)
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|item| {
                let (key, value) = item.split_once('=').unwrap();
                (unit_of(key), value.to_owned())
            })
            .collect()
    };
    let mut totals = Totals::new();
    for (unit, value) in items("BRUTEX_REAL_SAMPLE") {
        let (bytes, days) = value.split_once(':').unwrap();
        let (bytes, days) = (bytes.parse().unwrap(), days.parse().unwrap());
        match unit {
            Unit::Kind(key) => totals.add_kind(key, bytes, days).unwrap(),
            Unit::Underlying { name, class } => {
                totals.add_underlying(&name, class, bytes, days).unwrap();
            }
        }
    }
    let sealed = totals.seal();
    let plan: Vec<PlanItem> = items("BRUTEX_REAL_PLAN")
        .into_iter()
        .map(|(unit, days)| PlanItem {
            unit,
            days: days.parse().unwrap(),
            instruments: 1,
        })
        .collect();
    let projection = project(&sealed, &plan);
    for line in &projection.lines {
        println!(
            "{} : {} bytes a day x {} days = {} bytes ({:?})",
            line.unit, line.per_day, line.days, line.bytes, line.basis
        );
        assert_eq!(line.bytes, line.per_day * line.days, "days x bytes a day");
        if let Basis::Extrapolated { class, .. } = line.basis {
            assert_eq!(Some(line.per_day), sealed.median_of(class).map(|(m, _)| m));
            assert!(
                projection.extrapolated.contains(&line.unit.to_string()),
                "named"
            );
        }
    }
    println!("{}", projection.summary());
    assert!(
        projection.unmeasured.is_empty(),
        "nothing NOT KNOWN in this plan"
    );
    let store = std::path::PathBuf::from(read("BRUTEX_REAL_STORE"));
    let space = Df.space(&store).unwrap();
    let verdict = floor_check(space, projection.bytes, DEFAULT_FLOOR);
    println!(
        "df: total {} free {}; floor {DEFAULT_FLOOR} = {} bytes; verdict {verdict:?}",
        space.total,
        space.free,
        DEFAULT_FLOOR.bytes_of(space.total)
    );
    let fits = space.free
        >= projection
            .bytes
            .saturating_add(DEFAULT_FLOOR.bytes_of(space.total));
    assert_eq!(
        verdict.is_ok(),
        fits,
        "the verdict is the arithmetic, both ways"
    );
}
