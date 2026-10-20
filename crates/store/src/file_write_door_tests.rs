//! The write door over a sealed month (SC-4, D-5904; store-compression design
//! §3, §4.5, §6).
//!
//! Every test writes a raw month through the writer door into its own temp
//! root and seals it the way SC-3's tests do (`sealed::build`, published by
//! rename beside the raw form; the sealer itself is SC-6's). Each then asks
//! the WRITER door and checks three things: what it answers, that it wrote
//! nothing to either form, and that the answer is the one the raw month gives
//! the same batch. Prices are invented and absurd (tens of millions of
//! rupees), so no row can be mistaken for a vendor's.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_arithmetic
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::instrument::Contract;

use super::sealed_door_tests::{Root, month_path, path_at, seal, sealed_bytes, seconds, write_raw};
use super::{Appended, BarFile, Conflict, Form, SealedOffer, StoreError};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, StorePath, Timeframe, YearMonth};
use crate::repair::{MonthRepair, Policy, repair_month, revision_root};

/// The symbol id `sealed_door_tests::write_raw` writes with.
const SYMBOL: u32 = 0x5C3;
const MICROS: i64 = 1_000_000;
const DAY: i64 = 86_400 * MICROS;
const POLICY: Policy = Policy {
    open_interest_applies: true,
    basis_witness: false,
};

thread_local! {
    /// What runs right after the writer door took the month's lock.
    static AFTER_LOCK: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

/// Called by [`BarFile::open_or_create`] right after it took the lock.
pub(super) fn after_the_lock() {
    if let Some(work) = AFTER_LOCK.with(|hook| hook.borrow_mut().take()) {
        work();
    }
}

fn arm_after_the_lock(work: impl FnOnce() + 'static) {
    AFTER_LOCK.with(|hook| *hook.borrow_mut() = Some(Box::new(work)));
}

/// Every file under `dir`, by path, with its bytes.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// `bars` moved by `days` whole days.
fn shift(bars: &[Bar], days: i64) -> Vec<Bar> {
    bars.iter()
        .map(|bar| Bar {
            ts_micros: bar.ts_micros + days * DAY,
            ..*bar
        })
        .collect()
}

/// A raw month of `bars`, sealed beside its raw form.
fn sealed_month(tag: &str, path: StorePath<'_>, bars: &[Bar]) -> Root {
    let root = Root::new(tag);
    write_raw(&root.0, path, bars);
    seal(&root.0, path);
    root
}

/// What the writer door answers `offer` with at a fresh copy of `held`, raw
/// or sealed.
fn answer(
    path: StorePath<'_>,
    held: &[Bar],
    offer: &[Bar],
    sealed: bool,
) -> Result<Appended, StoreError> {
    let root = Root::new(if sealed {
        "w-blind-sealed"
    } else {
        "w-blind-raw"
    });
    write_raw(&root.0, path, held);
    if sealed {
        seal(&root.0, path);
    }
    let before = snapshot(&root.0);
    let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    assert_eq!(writer.form() == Form::Sealed, sealed);
    let answered = writer.append(offer);
    drop(writer);
    if sealed {
        assert_eq!(snapshot(&root.0), before, "a sealed month is never written");
    }
    answered
}

/// The contract month the instrument-agnostic cases repeat on.
fn option_path() -> StorePath<'static> {
    month_path(Some(Contract::parse("2031-05-29-2100000000-CE").unwrap()))
}

#[test]
fn a_sealed_month_refuses_a_new_bar_by_name() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // THE LATE DAY (design §6): a day after the last one a closed month
    // holds. A spot month and an option contract month alike: one door.
    for path in [month_path(None), option_path()] {
        let bars = seconds(700);
        let root = sealed_month("w-refuse", path, &bars);
        let before = snapshot(&root.0);
        let late = shift(&seconds(3), 1);
        let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
        assert_eq!(
            writer.form(),
            Form::Sealed,
            "the writer door opens the sealed form"
        );
        assert_eq!(
            writer.path(),
            path.with_file(FileKind::SealedBars)
                .to_path_buf(&root.0)
                .as_path()
        );
        let refused = writer.append(&late).unwrap_err();
        let StoreError::Sealed {
            path: named,
            n_valid,
            last_ts_micros,
            at,
            ts_micros,
            offer,
        } = &refused
        else {
            panic!("a new bar at a sealed month is `Sealed`, got {refused}");
        };
        assert_eq!(
            *named,
            path.with_file(FileKind::SealedBars).to_path_buf(&root.0)
        );
        assert_eq!((*n_valid, *last_ts_micros), (700, bars[699].ts_micros));
        assert_eq!(
            (*at, *ts_micros, *offer),
            (0, late[0].ts_micros, SealedOffer::Follows)
        );
        let words = refused.to_string();
        for word in [
            ".sbar",
            "sealed_offer=follows",
            "nothing was written",
            "repair_month",
        ] {
            assert!(words.contains(word), "{word:?} in {words}");
        }
        // A second refusal from the same handle says the same.
        assert_eq!(writer.append(&late).unwrap_err().to_string(), words);
        drop(writer);
        assert_eq!(snapshot(&root.0), before, "nothing written to either form");
        let reader = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
        assert_eq!((reader.form(), reader.records()), (Form::Sealed, 700));
        assert_eq!(reader.read_range(0, 700).unwrap(), bars);
    }
}

#[test]
fn a_reoffer_to_a_sealed_month_is_already_present() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // A RESUMED IMPORT STAYS IDEMPOTENT: any run of held bars, from the
    // first, the last, the middle and across a frame boundary (256 rows),
    // answers where it already sits, and nothing is written.
    let bars = seconds(700);
    let path = month_path(None);
    let root = sealed_month("w-reoffer", path, &bars);
    let before = snapshot(&root.0);
    let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    for (from, to) in [
        (0, 700),
        (0, 1),
        (699, 700),
        (255, 257),
        (256, 512),
        (300, 301),
        (511, 700),
    ] {
        assert_eq!(
            writer.append(&bars[from..to]).unwrap(),
            Appended::AlreadyPresent {
                first_index: from as u64,
                n_valid: 700,
            },
            "{from}..{to}"
        );
    }
    drop(writer);
    assert_eq!(snapshot(&root.0), before);
}

#[test]
fn every_offer_is_answered_as_the_raw_month_answers_it() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // THE FORM IS NEVER THE CONTENT: a batch is `AlreadyPresent` at the
    // sealed month exactly when it is at the raw one, `Follows` exactly when
    // the raw one commits it, and `Disagrees(c)` at the index and stamp of
    // the raw one's `OverlapDisagrees` with conflict `c`.
    let full = seconds(700);
    // A hole of ten seconds at rows 350..360, so a stamp inside the held
    // range that the month never held exists.
    let held: Vec<Bar> = full
        .iter()
        .enumerate()
        .filter(|(i, _)| !(350..360).contains(i))
        .map(|(_, bar)| *bar)
        .collect();
    let late = shift(&seconds(5), 1);
    let mut restated = held[100..110].to_vec();
    restated[3].close += 1;
    let mut overlap_then_new = held[680..].to_vec();
    overlap_then_new.extend_from_slice(&late);
    let mut overlap_differs_then_new = overlap_then_new.clone();
    overlap_differs_then_new[2].volume += 1;
    let earlier_day = shift(&seconds(3), -3);
    let cases: Vec<(&str, Vec<Bar>)> = vec![
        ("the whole month again", held.clone()),
        ("a tail after the last bar", late.clone()),
        ("held bars then new ones", overlap_then_new),
        (
            "held bars, one differing, then new ones",
            overlap_differs_then_new,
        ),
        ("a held bar restated", restated),
        ("inside the hole", full[352..355].to_vec()),
        ("across the hole", full[345..365].to_vec()),
        ("skipping a held bar", vec![held[10], held[12]]),
        ("a day before the first held", earlier_day),
    ];
    for path in [month_path(None), option_path()] {
        for (case, offer) in &cases {
            let raw = answer(path, &held, offer, false);
            let sealed = answer(path, &held, offer, true);
            match (&raw, &sealed) {
                (Ok(Appended::AlreadyPresent { .. }), Ok(_)) => {
                    assert_eq!(raw.as_ref().unwrap(), sealed.as_ref().unwrap(), "{case}");
                }
                (
                    Ok(Appended::Committed { first_index, .. }),
                    Err(StoreError::Sealed {
                        at,
                        ts_micros,
                        offer: SealedOffer::Follows,
                        n_valid,
                        ..
                    }),
                ) => {
                    // The raw month committed from its tail; the sealed one
                    // names the first offered bar it does not hold.
                    assert_eq!(*first_index, *n_valid, "{case}");
                    assert!(
                        (*at as usize) < offer.len(),
                        "{case}: `at` names an offered bar"
                    );
                    assert_eq!(*ts_micros, offer[*at as usize].ts_micros, "{case}");
                    assert!(
                        offer[..*at as usize].iter().all(|bar| held.contains(bar)),
                        "{case}: every bar before `at` is held"
                    );
                }
                (
                    Err(StoreError::OverlapDisagrees {
                        at,
                        ts_micros,
                        conflict,
                        ..
                    }),
                    Err(StoreError::Sealed {
                        at: sealed_at,
                        ts_micros: sealed_ts,
                        offer: SealedOffer::Disagrees(sealed_conflict),
                        ..
                    }),
                ) => assert_eq!(
                    (at, ts_micros, conflict),
                    (sealed_at, sealed_ts, sealed_conflict),
                    "{case}"
                ),
                _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
            }
        }
    }
    // And the conflicts named are the three there are.
    let named: Vec<SealedOffer> = cases
        .iter()
        .filter_map(
            |(_, offer)| match answer(month_path(None), &held, offer, true) {
                Err(StoreError::Sealed { offer, .. }) => Some(offer),
                _ => None,
            },
        )
        .collect();
    for want in [
        SealedOffer::Follows,
        SealedOffer::Disagrees(Conflict::Restated),
        SealedOffer::Disagrees(Conflict::NotHeld),
        SealedOffer::Disagrees(Conflict::Skipped),
    ] {
        assert!(named.contains(&want), "{want:?} among {named:?}");
    }
}

/// A garbage batch, and how its refusal is recognised.
type Garbage = (&'static str, Vec<Bar>, fn(&StoreError) -> bool);

#[test]
fn a_garbage_batch_is_named_for_what_it_is_at_a_sealed_month() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let bars = seconds(300);
    let path = month_path(None);
    let root = sealed_month("w-garbage", path, &bars);
    let before = snapshot(&root.0);
    let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    let next_month = YearMonth::new(2031, 6).unwrap().ist_bounds_micros().0 + 555 * 60 * MICROS;
    let mut impossible = bars[10];
    impossible.high = impossible.low - 1;
    let mut negative = shift(&bars[..1], 1)[0];
    negative.volume = -1;
    let mut off_grid = shift(&bars[..1], 1)[0];
    off_grid.ts_micros += 1;
    let cases: Vec<Garbage> = vec![
        ("empty", vec![], |e| matches!(e, StoreError::EmptyBatch)),
        ("out of order", vec![bars[5], bars[4]], |e| {
            matches!(e, StoreError::BatchNotOrdered { .. })
        }),
        ("a duplicate stamp", vec![bars[5], bars[5]], |e| {
            matches!(e, StoreError::BatchNotOrdered { .. })
        }),
        ("impossible OHLC", vec![impossible], |e| {
            matches!(e, StoreError::ImpossibleBar { .. })
        }),
        ("a negative volume", vec![negative], |e| {
            matches!(e, StoreError::ImpossibleCount { .. })
        }),
        (
            "the next month",
            vec![Bar {
                ts_micros: next_month,
                ..bars[0]
            }],
            |e| matches!(e, StoreError::OutsideMonth { .. }),
        ),
        ("off the rung's grid", vec![off_grid], |e| {
            matches!(e, StoreError::OffGrid { .. })
        }),
    ];
    for (case, offer, is) in cases {
        let refused = writer.append(&offer).unwrap_err();
        assert!(is(&refused), "{case}: {refused}");
    }
    drop(writer);
    assert_eq!(snapshot(&root.0), before);
}

#[test]
fn an_empty_and_a_one_bar_sealed_month_answer_at_the_door() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let path = month_path(None);
    let empty = sealed_month("w-empty", path, &[]);
    let mut writer = BarFile::open_or_create(&empty.0, path, SYMBOL).unwrap();
    assert_eq!((writer.form(), writer.records()), (Form::Sealed, 0));
    let one = seconds(1);
    match writer.append(&one).unwrap_err() {
        StoreError::Sealed {
            n_valid: 0,
            at: 0,
            offer: SealedOffer::Follows,
            ..
        } => {}
        other => panic!("{other}"),
    }
    drop(writer);
    let single = sealed_month("w-one", path, &one);
    let mut writer = BarFile::open_or_create(&single.0, path, SYMBOL).unwrap();
    assert_eq!(
        writer.append(&one).unwrap(),
        Appended::AlreadyPresent {
            first_index: 0,
            n_valid: 1
        }
    );
    let next = shift(&one, 1);
    assert!(matches!(
        writer.append(&next).unwrap_err(),
        StoreError::Sealed {
            n_valid: 1,
            offer: SealedOffer::Follows,
            ..
        }
    ));
}

#[test]
fn the_frame_and_index_boundaries_answer_alike_at_the_door() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // 256 rows a frame, 1,024 the sort threshold (design §6): both sides.
    for n in [255usize, 256, 257, 1023, 1024, 1025] {
        let bars = seconds(n as u64);
        let path = month_path(None);
        let root = sealed_month("w-bound", path, &bars);
        let before = snapshot(&root.0);
        let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
        for from in [0, n.saturating_sub(2), 254.min(n - 1), n - 1] {
            let to = (from + 4).min(n);
            assert_eq!(
                writer.append(&bars[from..to]).unwrap(),
                Appended::AlreadyPresent {
                    first_index: from as u64,
                    n_valid: n as u64
                },
                "{n}: {from}..{to}"
            );
        }
        let mut last = bars[n - 1];
        last.open_interest = if last.open_interest == OI_NULL {
            1
        } else {
            OI_NULL
        };
        match writer.append(&[last]).unwrap_err() {
            StoreError::Sealed {
                at: 0,
                offer: SealedOffer::Disagrees(Conflict::Restated),
                ..
            } => {}
            other => panic!("{n}: {other}"),
        }
        let mut tail = bars[n - 2..].to_vec();
        tail.extend(shift(&bars[..2], 1));
        match writer.append(&tail).unwrap_err() {
            StoreError::Sealed {
                at: 2,
                offer: SealedOffer::Follows,
                ..
            } => {}
            other => panic!("{n}: {other}"),
        }
        drop(writer);
        assert_eq!(snapshot(&root.0), before, "{n}");
    }
}

#[test]
fn a_writer_probes_for_the_sealed_form_under_its_lock() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // A SEAL THAT FINISHES BETWEEN THE WRITER'S START AND ITS LOCK is seen:
    // the `.sbar` is published from inside the door, right after the lock,
    // where a probe made before the lock would already have looked and
    // missed it, and the writer would have appended to a `.bin` no reader
    // opens any more.
    let bars = seconds(700);
    let path = month_path(None);
    let root = Root::new("w-under-lock");
    write_raw(&root.0, path, &bars);
    let bytes = sealed_bytes(&root.0, path);
    let at = path.with_file(FileKind::SealedBars).to_path_buf(&root.0);
    let raw_files: Vec<PathBuf> = [FileKind::Bars, FileKind::Checksums]
        .into_iter()
        .map(|kind| path.with_file(kind).to_path_buf(&root.0))
        .collect();
    let raw_before: Vec<Vec<u8>> = raw_files
        .iter()
        .map(|file| fs::read(file).unwrap())
        .collect();
    let publish = at.clone();
    arm_after_the_lock(move || {
        let staged = publish.with_extension("sbar-staged");
        fs::write(&staged, bytes).unwrap();
        fs::rename(&staged, &publish).unwrap();
    });
    let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    assert!(at.exists(), "the hook ran inside the door");
    assert_eq!(
        writer.form(),
        Form::Sealed,
        "the probe is made after the lock"
    );
    let late = shift(&bars[..3], 1);
    assert!(matches!(
        writer.append(&late).unwrap_err(),
        StoreError::Sealed {
            offer: SealedOffer::Follows,
            ..
        }
    ));
    drop(writer);
    let raw_after: Vec<Vec<u8>> = raw_files
        .iter()
        .map(|file| fs::read(file).unwrap())
        .collect();
    assert_eq!(
        raw_after, raw_before,
        "the raw form a sealed one supersedes is never appended to"
    );

    // AND A SEALER HOLDING THE LOCK SHARED (its steps 1 to 12) KEEPS THE
    // WRITER OUT: refused at once, by name, never waited on.
    let lock = path.with_file(FileKind::Lock).to_path_buf(&root.0);
    let sealer = crate::repair::shared_lock(lock).unwrap();
    assert!(matches!(
        BarFile::open_or_create(&root.0, path, SYMBOL).unwrap_err(),
        StoreError::Locked { .. }
    ));
    drop(sealer);
    assert_eq!(
        BarFile::open_or_create(&root.0, path, SYMBOL)
            .unwrap()
            .form(),
        Form::Sealed
    );
}

#[test]
fn a_month_held_only_sealed_is_never_created_again_raw() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // After a retirement (SC-9) only the `.sbar` remains. The writer door
    // must not create an empty `.bin` beside it and append there: the read
    // door opens the `.sbar` first and those bars would never be served.
    let bars = seconds(700);
    let path = month_path(None);
    let source = Root::new("w-alone-source");
    write_raw(&source.0, path, &bars);
    let bytes = sealed_bytes(&source.0, path);
    let root = Root::new("w-alone");
    let at = path.with_file(FileKind::SealedBars).to_path_buf(&root.0);
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    fs::write(&at, &bytes).unwrap();
    let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    assert_eq!(writer.form(), Form::Sealed);
    assert!(matches!(
        writer.append(&shift(&bars[..2], 1)).unwrap_err(),
        StoreError::Sealed { .. }
    ));
    assert!(matches!(
        writer.append(&bars[40..90]).unwrap(),
        Appended::AlreadyPresent {
            first_index: 40,
            ..
        }
    ));
    drop(writer);
    for kind in [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::TimeIndex,
        FileKind::Provenance,
    ] {
        let file = path.with_file(kind).to_path_buf(&root.0);
        assert!(
            !file.exists(),
            "{} was created beside a sealed month",
            file.display()
        );
    }
    assert_eq!(fs::read(&at).unwrap(), bytes);
    assert_eq!(
        BarFile::open_existing(&root.0, path, SYMBOL)
            .unwrap()
            .read_range(0, 700)
            .unwrap(),
        bars
    );
}

#[test]
fn a_damaged_or_misfiled_sealed_month_refuses_the_writer_by_name() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // Refused as the read door refuses it (`SealedRefused`), never passed
    // over for the `.bin` beside it, and never answered `AlreadyPresent`.
    let bars = seconds(700);
    let path = month_path(None);
    let root = sealed_month("w-damaged", path, &bars);
    let at = path.with_file(FileKind::SealedBars).to_path_buf(&root.0);
    let mut damaged = fs::read(&at).unwrap();
    damaged[100] ^= 0x40;
    fs::write(&at, &damaged).unwrap();
    let before = snapshot(&root.0);
    match BarFile::open_or_create(&root.0, path, SYMBOL).unwrap_err() {
        StoreError::SealedRefused { path: named, .. } => assert_eq!(named, at),
        other => panic!("{other}"),
    }
    assert_eq!(snapshot(&root.0), before);

    // A `.sbar` copied to another month's place, with nothing else there.
    let june = path_at(None, Timeframe::SECOND_1, YearMonth::new(2031, 6).unwrap());
    let good = sealed_month("w-misfiled-source", path, &bars);
    let bytes = fs::read(path.with_file(FileKind::SealedBars).to_path_buf(&good.0)).unwrap();
    let misfiled = Root::new("w-misfiled");
    let there = june
        .with_file(FileKind::SealedBars)
        .to_path_buf(&misfiled.0);
    fs::create_dir_all(there.parent().unwrap()).unwrap();
    fs::write(&there, &bytes).unwrap();
    match BarFile::open_or_create(&misfiled.0, june, SYMBOL).unwrap_err() {
        StoreError::SealedRefused { path: named, .. } => assert_eq!(named, there),
        other => panic!("{other}"),
    }
    assert!(
        !june.to_path_buf(&misfiled.0).exists(),
        "no raw month is created beside a refused sealed one"
    );
}

#[test]
fn two_writers_at_a_sealed_month_never_meet_and_never_write() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let bars = seconds(700);
    let path = month_path(None);
    let root = sealed_month("w-two", path, &bars);
    let before = snapshot(&root.0);
    let first = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    assert!(matches!(
        BarFile::open_or_create(&root.0, path, SYMBOL).unwrap_err(),
        StoreError::Locked { .. }
    ));
    drop(first);
    // Eight threads, each opening and offering forty times: every open is
    // the sealed handle or `Locked`, every re-offer `AlreadyPresent`, every
    // new bar `Sealed`, and the files never change.
    let late = shift(&bars[..5], 1);
    std::thread::scope(|scope| {
        for thread in 0..8usize {
            let (root, bars, late) = (&root.0, &bars, &late);
            scope.spawn(move || {
                for turn in 0..40usize {
                    match BarFile::open_or_create(root, path, SYMBOL) {
                        Ok(mut writer) => {
                            assert_eq!(writer.form(), Form::Sealed);
                            let from = (thread * 41 + turn * 13) % 690;
                            assert!(matches!(
                                writer.append(&bars[from..from + 10]).unwrap(),
                                Appended::AlreadyPresent { .. }
                            ));
                            assert!(matches!(
                                writer.append(late).unwrap_err(),
                                StoreError::Sealed { .. }
                            ));
                        }
                        Err(StoreError::Locked { .. }) => {}
                        Err(other) => panic!("{other}"),
                    }
                }
            });
        }
    });
    assert_eq!(snapshot(&root.0), before);
}

/// Days 5 and 7 of the month (Monday and Wednesday), 700 bars each: day 6
/// is a hole inside the held range, and day 8 a late day after it.
fn two_days() -> (Vec<Bar>, Vec<Bar>, Vec<Bar>) {
    let day5 = seconds(700);
    let mut held = day5.clone();
    held.extend(shift(&day5, 2));
    (held, shift(&day5, 1), shift(&day5, 3))
}

#[test]
fn a_repair_reads_a_sealed_month_and_publishes_its_successor_raw() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // THE DOOR THE CALLER TAKES (design §3): `repair_month` reads the sealed
    // month through the read door, keeps every held bar, adds the offered
    // ones, and serves the successor as a raw month. The sealed original and
    // its raw form are byte-identical afterwards.
    let (held, hole, late) = two_days();
    let path = month_path(None);
    for (case, offered) in [
        ("a day inside the held range", &hole),
        ("a late day after it", &late),
    ] {
        let root = sealed_month("w-repair", path, &held);
        let original = snapshot(&root.0.join("bars"));
        let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
        assert!(
            matches!(
                writer.append(offered).unwrap_err(),
                StoreError::Sealed { .. }
            ),
            "{case}"
        );
        drop(writer);
        let repaired = repair_month(&root.0, path, SYMBOL, offered, POLICY).unwrap();
        let MonthRepair::Repaired {
            from,
            held: kept,
            added,
            ..
        } = repaired
        else {
            panic!("{case}: {repaired:?}");
        };
        assert_eq!((from, kept, added), (0, 1400, 700), "{case}");
        let mut merged = held.clone();
        merged.extend_from_slice(offered);
        merged.sort_by_key(|bar| bar.ts_micros);
        let served = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
        assert_eq!(served.form(), Form::Raw, "{case}: the successor is raw");
        assert_eq!(
            served.read_range(0, served.records()).unwrap(),
            merged,
            "{case}"
        );
        assert_eq!(
            snapshot(&root.0.join("bars")),
            original,
            "{case}: the original is untouched"
        );
        // The same repair again: nothing new, no generation spent.
        assert!(matches!(
            repair_month(&root.0, path, SYMBOL, offered, POLICY).unwrap(),
            MonthRepair::NothingNew { ordinal: 1, .. }
        ));
    }
}

#[test]
fn a_month_repaired_twice_over_sealed_generations_keeps_every_generation() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // Original sealed; revision 1 raw, then sealed; revision 2 read from
    // the sealed revision 1. Every generation stays, each in its own form.
    let (held, hole, late) = two_days();
    let path = month_path(None);
    let root = sealed_month("w-twice", path, &held);
    repair_month(&root.0, path, SYMBOL, &hole, POLICY).unwrap();
    let first = revision_root(&root.0, crate::repair::Revision::new(1).unwrap());
    seal(&first, path);
    let reader = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!((reader.form(), reader.records()), (Form::Sealed, 2100));
    drop(reader);
    let generations = (snapshot(&root.0.join("bars")), snapshot(&first));
    let repaired = repair_month(&root.0, path, SYMBOL, &late, POLICY).unwrap();
    assert!(
        matches!(
            repaired,
            MonthRepair::Repaired {
                from: 1,
                held: 2100,
                added: 700,
                ..
            }
        ),
        "{repaired:?}"
    );
    let served = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    let mut all = held.clone();
    all.extend_from_slice(&hole);
    all.extend_from_slice(&late);
    all.sort_by_key(|bar| bar.ts_micros);
    assert_eq!(served.read_range(0, served.records()).unwrap(), all);
    assert_eq!(
        (snapshot(&root.0.join("bars")), snapshot(&first)),
        generations,
        "the original and revision 1 are untouched"
    );
}

#[test]
fn a_revision_ordinal_holding_a_sealed_form_is_stepped_over() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // A `.sbar` at the next ordinal's month path that no reservation names
    // (another request's, or a copy): a raw revision written beside it would
    // be served as the `.sbar`'s bars, so the ordinal is stepped over.
    let (held, hole, _) = two_days();
    let path = month_path(None);
    let root = Root::new("w-ordinal");
    write_raw(&root.0, path, &held);
    let stray = sealed_bytes(&root.0, path);
    let first = revision_root(&root.0, crate::repair::Revision::new(1).unwrap());
    let at = path.with_file(FileKind::SealedBars).to_path_buf(&first);
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    fs::write(&at, &stray).unwrap();
    let repaired = repair_month(&root.0, path, SYMBOL, &hole, POLICY).unwrap();
    let MonthRepair::Repaired { to, skipped, .. } = repaired else {
        panic!("{repaired:?}");
    };
    assert_eq!(to.ordinal(), 2);
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0].0, 1);
    assert!(skipped[0].1.contains(".sbar"), "{skipped:?}");
    assert_eq!(
        fs::read(&at).unwrap(),
        stray,
        "the stray file is left as found"
    );
    let served = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(served.records(), 2100);
}

/// The sealed month is what is served, bar for bar, and the writer door still
/// answers a late day `Sealed`/`Follows` and writes nothing.
fn the_sealed_month_is_still_served(
    root: &Path,
    path: StorePath<'_>,
    held: &[Bar],
    late: &[Bar],
    case: &str,
) {
    let reader = BarFile::open_existing(root, path, SYMBOL).unwrap();
    assert_eq!(
        (reader.form(), reader.records()),
        (Form::Sealed, held.len() as u64),
        "{case}: the sealed month is served"
    );
    assert_eq!(
        reader.read_range(0, reader.records()).unwrap(),
        held,
        "{case}"
    );
    drop(reader);
    let before = snapshot(root);
    let mut writer = BarFile::open_or_create(root, path, SYMBOL).unwrap();
    let refused = writer.append(late).unwrap_err();
    assert!(
        matches!(
            refused,
            StoreError::Sealed {
                offer: SealedOffer::Follows,
                ..
            }
        ),
        "{case}: {refused}"
    );
    drop(writer);
    assert_eq!(
        snapshot(root),
        before,
        "{case}: the writer door wrote nothing"
    );
}

#[test]
fn a_successor_interrupted_at_each_step_serves_the_sealed_month_until_it_is_promoted() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    // FM-283 AT THE WRITE DOOR (design §3, §5.2 L-SC-7): committed bars are
    // never rewritten. A late day for a sealed month lands only as a whole
    // successor generation, so a kill at any step before that generation's
    // promotion leaves the sealed month served exactly as it was, the writer
    // door still answering `Sealed`, and the next repair finishes the job.
    // The kill points are logical, as `REPAIR.md`'s own are: the state each
    // step leaves on disk, built directly or stopped by the caller's step.
    let (held, _, late) = two_days();
    let mut merged = held.clone();
    merged.extend_from_slice(&late);
    let path = month_path(None);
    let first = crate::repair::Revision::new(1).unwrap();

    // KILLED MID-WRITE: the ordinal reserved and its `.bin` torn.
    let case = "killed after the reservation, the bars torn";
    let root = sealed_month("w-kill-torn", path, &held);
    let original = snapshot(&root.0.join("bars"));
    let physical = path.to_path_buf(&revision_root(&root.0, first));
    fs::create_dir_all(physical.parent().unwrap()).unwrap();
    fs::write(physical.with_extension("reserved-v1"), []).unwrap();
    fs::write(&physical, b"torn by a kill").unwrap();
    the_sealed_month_is_still_served(&root.0, path, &held, &late, case);
    let repaired = repair_month(&root.0, path, SYMBOL, &late, POLICY).unwrap();
    let MonthRepair::Repaired { to, skipped, .. } = repaired else {
        panic!("{case}: {repaired:?}");
    };
    assert_eq!(to.ordinal(), 2, "{case}: the torn ordinal is stepped over");
    assert_eq!(skipped.len(), 1, "{case}: {skipped:?}");
    assert_eq!(skipped[0].0, 1, "{case}");
    assert_eq!(
        fs::read(&physical).unwrap(),
        b"torn by a kill",
        "{case}: the torn file is kept as found"
    );
    let served = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(served.form(), Form::Raw, "{case}");
    assert_eq!(
        served.read_range(0, served.records()).unwrap(),
        merged,
        "{case}"
    );
    assert_eq!(snapshot(&root.0.join("bars")), original, "{case}");

    // KILLED BETWEEN THE WHOLE REVISION AND ITS PROMOTION.
    let case = "killed after the revision was whole, before its promotion";
    let root = sealed_month("w-kill-promote", path, &held);
    let original = snapshot(&root.0.join("bars"));
    let mut killed = |_: &Path| Err("killed before the promotion".to_owned());
    let stopped =
        crate::repair::repair_month_with(&root.0, path, SYMBOL, &late, POLICY, Some(&mut killed))
            .unwrap_err();
    assert!(
        matches!(stopped, crate::repair::RepairError::BeforePromote { .. }),
        "{case}: {stopped:?}"
    );
    the_sealed_month_is_still_served(&root.0, path, &held, &late, case);
    let repaired = repair_month(&root.0, path, SYMBOL, &late, POLICY).unwrap();
    let MonthRepair::Repaired {
        to,
        published,
        skipped,
        ..
    } = repaired
    else {
        panic!("{case}: {repaired:?}");
    };
    assert_eq!(
        (to.ordinal(), published, skipped.len()),
        (1, crate::repair::Published::Reused, 0),
        "{case}: the whole revision is found again and promoted"
    );
    let served = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
    assert_eq!(served.form(), Form::Raw, "{case}");
    assert_eq!(
        served.read_range(0, served.records()).unwrap(),
        merged,
        "{case}"
    );
    assert_eq!(snapshot(&root.0.join("bars")), original, "{case}");
}

/// p50, p99 and max of `samples`, in microseconds.
fn percentiles(samples: &mut [f64]) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    let at = |q: f64| samples[((samples.len() as f64 - 1.0) * q).round() as usize];
    (at(0.50), at(0.99), samples[samples.len() - 1])
}

#[test]
fn the_write_door_costs_the_same_at_one_ten_and_a_hundred_times_the_bars() {
    const N: usize = 2_000;
    let _sink_is_mine = crate::emits::hold_the_sink();
    // O(1) IN THE MONTH (CLAUDE.md §3 rule 4; operator's rule (1)): a
    // re-offer of 60 held bars and a refused late batch of 60 each cost one
    // time lookup and at most 60 record reads, whatever the month holds.
    // Measured on one warm writer handle per size, n = 2,000 each, times
    // printed; the assertion has margins for a loaded machine.
    let mut p99s = Vec::new();
    for size in [2_250u64, 22_500, 225_000] {
        let bars = seconds(size);
        let path = month_path(None);
        let root = sealed_month("w-cost", path, &bars);
        // THE SAME MONTH RAW, beside it: writer opens alternate between the
        // two, so both forms meet the same moment of the machine's load.
        let raw = Root::new("w-cost-raw");
        write_raw(&raw.0, path, &bars);
        let (mut opens, mut raw_opens) = (Vec::with_capacity(200), Vec::with_capacity(200));
        for _ in 0..200 {
            let started = Instant::now();
            let writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
            opens.push(started.elapsed().as_secs_f64() * 1e6);
            assert_eq!(writer.form(), Form::Sealed);
            drop(writer);
            let started = Instant::now();
            let writer = BarFile::open_or_create(&raw.0, path, SYMBOL).unwrap();
            raw_opens.push(started.elapsed().as_secs_f64() * 1e6);
            assert_eq!(writer.form(), Form::Raw);
            drop(writer);
        }
        let mut raw_writer = BarFile::open_or_create(&raw.0, path, SYMBOL).unwrap();
        let mut raw_reoffers = Vec::with_capacity(N);
        let mut writer = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
        let late = shift(&bars[..60], 3);
        let (mut reoffers, mut refusals) = (Vec::with_capacity(N), Vec::with_capacity(N));
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..N {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let from = (seed >> 33) as usize % (bars.len() - 60);
            let started = Instant::now();
            let answered = writer.append(&bars[from..from + 60]);
            reoffers.push(started.elapsed().as_secs_f64() * 1e6);
            assert!(matches!(answered, Ok(Appended::AlreadyPresent { .. })));
            let started = Instant::now();
            let answered = raw_writer.append(&bars[from..from + 60]);
            raw_reoffers.push(started.elapsed().as_secs_f64() * 1e6);
            assert!(matches!(answered, Ok(Appended::AlreadyPresent { .. })));
            let started = Instant::now();
            let answered = writer.append(&late);
            refusals.push(started.elapsed().as_secs_f64() * 1e6);
            assert!(matches!(answered, Err(StoreError::Sealed { .. })));
        }
        let (o50, o99, omax) = percentiles(&mut opens);
        let (q50, q99, qmax) = percentiles(&mut raw_opens);
        let (r50, r99, rmax) = percentiles(&mut reoffers);
        let (w50, w99, wmax) = percentiles(&mut raw_reoffers);
        let (s50, s99, smax) = percentiles(&mut refusals);
        println!(
            "SC4 write door at {size} bars: writer open n=200 sealed p50 {o50:.1} p99 {o99:.1} \
             max {omax:.1} us, raw p50 {q50:.1} p99 {q99:.1} max {qmax:.1} us; re-offer of 60 \
             n={N} sealed p50 {r50:.1} p99 {r99:.1} max {rmax:.1} us, raw p50 {w50:.1} p99 \
             {w99:.1} max {wmax:.1} us; refused late 60 n={N} p50 {s50:.1} p99 {s99:.1} max \
             {smax:.1} us"
        );
        // THE MEDIAN IS ASSERTED FOR OPENS, THE p99 PRINTED: a writer open
        // takes and notes the month's lock, and a stall of the shared machine
        // in two of 200 opens moves the p99 of either form by milliseconds
        // (about 8 ms in both forms on 20 Oct, L-SC4-02). The re-offer and
        // the refusal are asserted at p99, over 2,000 calls each.
        assert!(
            o50 <= 3.0 * q50 + 1_000.0,
            "{size}: a sealed writer open p50 {o50:.1} us against raw {q50:.1} us"
        );
        p99s.push((o50, r99, s99));
    }
    let (small, large) = (p99s[0], p99s[2]);
    for (what, at_1, at_100) in [
        ("open p50", small.0, large.0),
        ("re-offer p99", small.1, large.1),
        ("refusal p99", small.2, large.2),
    ] {
        assert!(
            at_100 <= 3.0 * at_1 + 1_000.0,
            "{what}: {at_100:.1} us at 100x against {at_1:.1} us at 1x"
        );
    }
}
