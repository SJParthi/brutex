//! Generated references only. These tests do not pull or sweep market history.
use super::*;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use crate::index_stop::tests::{limits, programs, source};
use brutex_core::instrument::{Exchange, InstrumentKey};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe};

fn bounds() -> Bounds {
    Bounds {
        bytes: 64 * 1024 * 1024,
        records: 400_000,
        memory_bytes: 64 * 1024 * 1024,
        page_records: 256,
    }
}
fn symbol_id(symbol: &str) -> u32 {
    let bytes = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    u32::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ])
}
fn vix_path(fixture: &Fixture, feed: Vendor) -> Result<PathBuf, String> {
    let key = InstrumentKey::index(Exchange::Nse, "INDIAVIX").map_err(display)?;
    Ok(StorePath::for_key(
        feed,
        &key,
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 5).map_err(display)?,
        FileKind::Bars,
    )
    .map_err(display)?
    .to_path_buf(&fixture.root))
}
fn seed(fixture: &Fixture, feed: Vendor, offset: i64, empty: bool) -> Result<(), String> {
    let month = YearMonth::new(2025, 5).map_err(display)?;
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").map_err(display)?;
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        month,
        FileKind::Bars,
    )
    .map_err(display)?;
    let original =
        BarFile::open_existing(&fixture.root, path, symbol_id("NIFTY")).map_err(display)?;
    let mut bars = Vec::new();
    if !empty {
        bars.try_reserve_exact(address(original.records())?)
            .map_err(display)?;
        for index in 0..original.records() {
            let bar = original.read_record(index).map_err(display)?;
            bars.push(Bar {
                ts_micros: bar.ts_micros,
                open: 150_000 + offset,
                high: 150_100 + offset,
                low: 149_900 + offset,
                close: 150_050 + offset,
                volume: 7,
                open_interest: i64::MIN,
            });
        }
    }
    let key = InstrumentKey::index(Exchange::Nse, "INDIAVIX").map_err(display)?;
    let path = StorePath::for_key(feed, &key, Timeframe::MINUTE_1, month, FileKind::Bars)
        .map_err(display)?;
    let mut file =
        BarFile::open_or_create(&fixture.root, path, symbol_id("INDIAVIX")).map_err(display)?;
    if !bars.is_empty() {
        file.append(&bars).map_err(display)?;
    }
    Ok(())
}
fn saved(
    reference: Option<(Vendor, i64, bool)>,
) -> Result<
    (
        Fixture,
        crate::index_stop::Loaded,
        crate::index_stop::Committed,
    ),
    String,
> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    if let Some((feed, offset, empty)) = reference {
        seed(&fixture, feed, offset, empty)?;
    }
    let context = loaded.prepare(limits())?;
    let saved = crate::index_stop::produce_catalog(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
    )?;
    Ok((fixture, loaded, saved))
}
fn open(fixture: &Fixture, saved: &crate::index_stop::Committed) -> Result<Reader, String> {
    Reader::open(
        &fixture.output,
        saved.identity(),
        saved.completion_digest(),
        bounds(),
    )
}
fn directory(fixture: &Fixture, saved: &crate::index_stop::Committed) -> PathBuf {
    fixture
        .output
        .join(NAMESPACE)
        .join(crate::identity_hex(&lookup_identity(
            saved.identity(),
            saved.completion_digest(),
        )))
}
fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, [u8; 32]>, String> {
    let mut files = BTreeMap::new();
    for item in fs::read_dir(root).map_err(display)? {
        let item = item.map_err(display)?;
        let path = item.path();
        if item.file_type().map_err(display)?.is_dir() {
            files.extend(inventory(&path)?);
        } else {
            files.insert(path.clone(), hash(&fs::read(path).map_err(display)?));
        }
    }
    Ok(files)
}
fn reseal(
    fixture: &Fixture,
    saved: &crate::index_stop::Committed,
    body: &[u8],
) -> Result<(), String> {
    let id = lookup_identity(saved.identity(), saved.completion_digest());
    let path = directory(fixture, saved);
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&id);
    receipt.extend_from_slice(&hash(body));
    receipt.extend_from_slice(&(body.len() as u64).to_le_bytes());
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(path.join("body.bin"), body).map_err(display)?;
    fs::write(path.join("complete.bin"), receipt).map_err(display)
}

#[test]
fn index_stop_vix_exact_original_fields_both_directions_and_forced_interval_are_visible()
-> Result<(), String> {
    let (fixture, _, saved) = saved(Some((Vendor::Zerodha, 0, false)))?;
    let reader = open(&fixture, &saved)?;
    let before = inventory(&fixture.root)?;
    reader.with_current(|view| {
        assert_eq!(view.metadata().unavailable_stamps, 0);
        assert_eq!(view.metadata().absent_stamps, 0);
        assert!(view.metadata().exact_stamps > 0);
        assert_eq!(view.metadata().feed, "zerodha");
        let mut forced = 0;
        let mut stopped = 0;
        for setting in 0..address(view.metadata().settings)? {
            let native = view.native_record(setting)?;
            let mut offset = 0;
            loop {
                let page = view.page(setting, offset, 256)?;
                for row in page.rows {
                    let trade = view.original_trade(setting, row.trade_index)?;
                    assert_eq!(row.run_id, native.run_id());
                    assert_eq!(row.trade_digest, hash(&trade.canonical_bytes()));
                    for (stamp, ts) in [
                        (row.entry, trade.entry_micros),
                        (row.exit, trade.exit_bar_micros),
                    ] {
                        let Stamp::Exact(candle) = stamp else {
                            return Err("original exact VIX candle disappeared".into());
                        };
                        assert_eq!(
                            candle,
                            Candle {
                                ts_micros: ts,
                                open: 150_000,
                                high: 150_100,
                                low: 149_900,
                                close: 150_050,
                                volume: 7,
                                open_interest: i64::MIN
                            }
                        );
                    }
                    let month = view.month(row.month_index)?;
                    assert!(month.snapshot_digest.is_some());
                    assert!(month.records.is_some_and(|count| count > 0));
                    assert_eq!(month.unavailable_code(), None);
                    match trade.exit_reason {
                        crate::index_stop_store::ExitReason::Forced1510 => {
                            forced += 1;
                            assert_eq!(row.exit_from_micros, row.exit_until_micros);
                            assert_eq!(row.exit_from_micros, row.exit_bar_micros + 60_000_000);
                        }
                        crate::index_stop_store::ExitReason::Stop => {
                            stopped += 1;
                            assert_eq!(row.exit_from_micros, row.exit_bar_micros);
                            assert_eq!(row.exit_until_micros, row.exit_bar_micros + 60_000_000);
                        }
                    }
                }
                match page.next_offset {
                    Some(next) => offset = next,
                    None => break,
                }
            }
        }
        assert!(forced + stopped > 0);
        assert_eq!(view.page(2, 0, 1)?.total, 0);
        Ok(())
    })?;
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn index_stop_vix_valid_month_holes_and_unavailable_months_are_never_conflated()
-> Result<(), String> {
    for reference in [
        None,
        Some((Vendor::Zerodha, 0, true)),
        Some((Vendor::Dhan, 0, false)),
    ] {
        let (fixture, _, saved) = saved(reference)?;
        let reader = open(&fixture, &saved)?;
        reader.with_current(|view| {
            let row = view.page(0, 0, 1)?.rows.first().ok_or("native trade")?;
            let month = view.month(row.month_index)?;
            if reference.is_some_and(|(feed, _, empty)| feed == Vendor::Zerodha && empty) {
                assert_eq!(row.entry, Stamp::Absent);
                assert_eq!(row.exit, Stamp::Absent);
                assert_eq!(month.records, Some(0));
                assert!(month.snapshot_digest.is_some());
                assert!(month.unavailable_reason.is_none());
                assert_eq!(view.metadata().unavailable_stamps, 0);
            } else {
                assert_eq!(row.entry, Stamp::Unavailable);
                assert_eq!(row.exit, Stamp::Unavailable);
                assert_eq!(month.records, None);
                assert_eq!(month.snapshot_digest, None);
                assert_eq!(month.unavailable_code(), Some("reference_month_refused"));
                assert!(
                    month
                        .unavailable_reason
                        .as_ref()
                        .is_some_and(|reason| reason.contains("zerodha")
                            && reason.contains("Nothing was stamped"))
                );
                assert_eq!(view.metadata().absent_stamps, 0);
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn index_stop_vix_retries_and_cold_reopen_retain_snapshot_after_current_data_changes()
-> Result<(), String> {
    let (fixture, loaded, saved) = saved(Some((Vendor::Zerodha, 0, false)))?;
    let reader = open(&fixture, &saved)?;
    let metadata = reader.with_current(|view| Ok(view.metadata().clone()))?;
    let path = vix_path(&fixture, Vendor::Zerodha)?;
    fs::rename(&path, path.with_extension("original-fixture")).map_err(display)?;
    seed(&fixture, Vendor::Zerodha, 500, false)?;
    let current = VixReferenceMonth::open(
        &fixture.root,
        Vendor::Zerodha,
        YearMonth::new(2025, 5).map_err(display)?,
    )?;
    reader.with_current(|view| {
        let row = view.page(0, 0, 1)?.rows.first().ok_or("native trade")?;
        assert_ne!(
            view.month(row.month_index)?.snapshot_digest,
            Some(current.snapshot_digest())
        );
        Ok(())
    })?;
    let context = loaded.prepare(limits())?;
    let repeated = crate::index_stop::produce_catalog(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
    )?;
    assert_eq!(saved.identity(), repeated.identity());
    assert_eq!(saved.completion_digest(), repeated.completion_digest());
    let reopened = open(&fixture, &repeated)?;
    reopened.with_current(|view| {
        assert_eq!(*view.metadata(), metadata);
        assert!(matches!(
            view.page(0, 0, 1)?.rows.first().ok_or("trade")?.entry,
            Stamp::Exact(Candle { open: 150_000, .. })
        ));
        Ok(())
    })?;
    Ok(())
}

#[test]
fn index_stop_vix_reference_changes_only_its_independent_publication() -> Result<(), String> {
    let (first, _, a) = saved(Some((Vendor::Zerodha, 0, false)))?;
    let (second, _, b) = saved(Some((Vendor::Zerodha, 1000, false)))?;
    assert_eq!(a.identity(), b.identity());
    assert_eq!(a.completion_digest(), b.completion_digest());
    for (left, right) in a.evaluations().iter().zip(b.evaluations()) {
        assert_eq!(left.run_id(), right.run_id());
        assert_eq!(left.source_id(), right.source_id());
        assert_eq!(left.digest(), right.digest());
        assert_eq!(left.trades(), right.trades());
        assert_eq!(left.metrics(), right.metrics());
    }
    let a = open(&first, &a)?;
    let b = open(&second, &b)?;
    let ap = a.with_current(|view| Ok(view.metadata().publication_id))?;
    let bp = b.with_current(|view| Ok(view.metadata().publication_id))?;
    assert_ne!(ap, bp);
    Ok(())
}

#[test]
fn index_stop_vix_bounds_pages_and_zero_trade_extents_refuse_without_inspection_writes()
-> Result<(), String> {
    let (fixture, _, saved) = saved(None)?;
    let reader = open(&fixture, &saved)?;
    let before = inventory(&fixture.root)?;
    for small in [
        Bounds {
            bytes: 1,
            ..bounds()
        },
        Bounds {
            records: 1,
            ..bounds()
        },
        Bounds {
            memory_bytes: 1,
            ..bounds()
        },
        Bounds {
            page_records: 0,
            ..bounds()
        },
    ] {
        assert!(
            Reader::open(
                &fixture.output,
                saved.identity(),
                saved.completion_digest(),
                small
            )
            .is_err()
        );
    }
    assert!(Reader::open(&fixture.output, saved.identity(), [9; 32], bounds()).is_err());
    reader.with_current(|view| {
        assert!(view.page(usize::MAX, 0, 1).is_err());
        assert!(view.page(0, 0, 0).is_err());
        assert!(view.page(0, 0, 257).is_err());
        assert!(view.page(0, u64::MAX, 1).is_err());
        assert!(view.page(2, 1, 1).is_err());
        assert!(view.page(2, 0, 1)?.rows.is_empty());
        assert!(view.original_trade(2, 0).is_err());
        assert!(view.month(u64::MAX).is_err());
        Ok(())
    })?;
    assert!(require_read_cost(u64::MAX, 112, bounds()).is_err());
    assert!(require_capture_cost(0, 0, u64::MAX, bounds()).is_err());
    assert!(codec::maximum_bytes(u64::MAX, 0, 0).is_err());
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn index_stop_vix_missing_torn_or_corrupt_companion_never_backfills_on_read() -> Result<(), String>
{
    let (fixture, _, saved) = saved(None)?;
    let directory = directory(&fixture, &saved);
    let completion = fs::read(directory.join("complete.bin")).map_err(display)?;
    fs::remove_file(directory.join("complete.bin")).map_err(display)?;
    let before = inventory(&fixture.root)?;
    assert!(open(&fixture, &saved).is_err());
    assert_eq!(inventory(&fixture.root)?, before);
    fs::write(directory.join("complete.bin"), completion).map_err(display)?;
    let original = fs::read(directory.join("body.bin")).map_err(display)?;
    fs::write(
        directory.join("body.bin"),
        original
            .get(..original.len().checked_sub(1).ok_or("fixture byte")?)
            .ok_or("fixture body")?,
    )
    .map_err(display)?;
    assert!(open(&fixture, &saved).is_err());
    fs::write(directory.join("body.bin"), &original).map_err(display)?;
    let cached = open(&fixture, &saved)?;
    let mut corrupt = original;
    *corrupt.last_mut().ok_or("fixture byte")? ^= 1;
    fs::write(directory.join("body.bin"), corrupt).map_err(display)?;
    assert!(cached.require_current().is_err());
    assert!(open(&fixture, &saved).is_err());
    Ok(())
}

#[test]
fn index_stop_vix_resealed_foreign_trade_and_hidden_unavailable_state_are_rejected()
-> Result<(), String> {
    let (fixture, _, saved) = saved(None)?;
    let original = fs::read(directory(&fixture, &saved).join("body.bin")).map_err(display)?;
    let mut image = codec::decode(
        &original,
        saved.identity(),
        saved.completion_digest(),
        bounds().records,
        bounds().memory_bytes,
    )?;
    image.rows.first_mut().ok_or("native trade")?.trade_digest = [7; 32];
    let forged = codec::encode(&image, bounds().bytes)?;
    reseal(&fixture, &saved, &forged)?;
    assert!(
        open(&fixture, &saved)
            .err()
            .ok_or("forged trade accepted")?
            .contains("exact original native trade")
    );
    let mut image = codec::decode(
        &original,
        saved.identity(),
        saved.completion_digest(),
        bounds().records,
        bounds().memory_bytes,
    )?;
    image.rows.first_mut().ok_or("native trade")?.entry = Stamp::Absent;
    let forged = codec::encode(&image, bounds().bytes)?;
    reseal(&fixture, &saved, &forged)?;
    assert!(
        open(&fixture, &saved)
            .err()
            .ok_or("hidden unavailable accepted")?
            .contains("hides an unavailable")
    );
    let mut tail = original;
    tail.push(0);
    assert!(
        codec::decode(
            &tail,
            saved.identity(),
            saved.completion_digest(),
            bounds().records,
            bounds().memory_bytes,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn index_stop_vix_corrupt_publication_prevents_completed_native_attempt_without_rewriting_catalog()
-> Result<(), String> {
    let (fixture, loaded, saved) = saved(None)?;
    let native_path = fixture
        .output
        .join(crate::index_stop_store::NAMESPACE)
        .join(crate::identity_hex(&saved.identity()))
        .join("body.bin");
    let native = fs::read(&native_path).map_err(display)?;
    fs::write(directory(&fixture, &saved).join("complete.bin"), b"torn").map_err(display)?;
    let context = loaded.prepare(limits())?;
    assert!(
        crate::index_stop::produce_catalog(
            &fixture.output,
            &loaded,
            &context,
            &programs()?,
            loaded.days(),
            limits()
        )
        .is_err()
    );
    assert_eq!(fs::read(native_path).map_err(display)?, native);
    let evidence = crate::sweep_evidence::read(&fixture.output, saved.identity(), bounds().bytes)?
        .ok_or("native attempt")?;
    assert_eq!(
        evidence.completion,
        crate::sweep_evidence::Completion::Refused
    );
    for row in saved.evaluations() {
        let evidence = crate::sweep_evidence::read(&fixture.output, row.run_id(), bounds().bytes)?
            .ok_or("native run attempt")?;
        assert_eq!(
            evidence.completion,
            crate::sweep_evidence::Completion::Refused
        );
    }
    Ok(())
}

#[test]
fn index_stop_vix_resealed_repeated_minutes_cannot_disagree_across_settings_or_within_a_trade()
-> Result<(), String> {
    let (fixture, _, saved) = saved(Some((Vendor::Zerodha, 0, false)))?;
    let original = fs::read(directory(&fixture, &saved).join("body.bin")).map_err(display)?;
    let decode = || {
        codec::decode(
            &original,
            saved.identity(),
            saved.completion_digest(),
            bounds().records,
            bounds().memory_bytes,
        )
    };
    let image = decode()?;
    let first = image.settings.first().ok_or("first native setting")?;
    let second = image.settings.get(1).ok_or("second native setting")?;
    assert_ne!(first.run_id, second.run_id);
    let first_rows = image
        .rows
        .get(address(first.first)?..address(first.first + first.count)?)
        .ok_or("first native rows")?;
    let second_rows = image
        .rows
        .get(address(second.first)?..address(second.first + second.count)?)
        .ok_or("second native rows")?;
    let matched = first_rows
        .iter()
        .find_map(|left| {
            second_rows
                .iter()
                .enumerate()
                .find(|(_, right)| right.entry_micros == left.entry_micros)
                .map(|(index, _)| index)
        })
        .ok_or("generated long/short settings must share an entry minute")?;
    let changed_index = address(second.first)?
        .checked_add(matched)
        .ok_or("fixture row coordinate")?;
    for change_state in [false, true] {
        let mut forged = decode()?;
        let row = forged
            .rows
            .get_mut(changed_index)
            .ok_or("shared native row")?;
        let Stamp::Exact(mut bar) = row.entry else {
            return Err("generated exact reference missing".into());
        };
        // Both alterations remain individually valid, with every native trade
        // byte and snapshot pin unchanged. Keep within-row equality intact so
        // rejection must also cover another setting's original same minute.
        bar.high += 1;
        let changed = if change_state {
            Stamp::Absent
        } else {
            Stamp::Exact(bar)
        };
        row.entry = changed;
        if row.entry_micros == row.exit_bar_micros {
            row.exit = changed;
        }
        let body = codec::encode(&forged, bounds().bytes)?;
        reseal(&fixture, &saved, &body)?;
        let why = open(&fixture, &saved)
            .err()
            .ok_or("contradictory cross-setting VIX reference accepted")?;
        assert!(why.contains("repeated original minute carries contradictory complete stamps"));
    }
    let mut forged = decode()?;
    let row = forged
        .rows
        .iter_mut()
        .find(|row| row.entry_micros == row.exit_bar_micros)
        .ok_or("generated immediate-stop trade")?;
    row.exit = Stamp::Absent;
    reseal(&fixture, &saved, &codec::encode(&forged, bounds().bytes)?)?;
    assert!(
        open(&fixture, &saved)
            .err()
            .ok_or("contradictory entry/exit reference accepted")?
            .contains("contradictory complete stamps")
    );
    Ok(())
}

#[test]
fn index_stop_vix_repeated_minute_scratch_is_admitted_before_cold_validation() -> Result<(), String>
{
    let (fixture, _, saved) = saved(None)?;
    let reader = open(&fixture, &saved)?;
    let scratch = codec::validation_bytes(reader.image.rows.len() as u64)?;
    assert!(scratch > 0);
    let base = reader
        .admitted_bytes
        .checked_mul(4)
        .and_then(|n| n.checked_add(4096))
        .ok_or("fixture base memory")?;
    let tight = Bounds {
        memory_bytes: base
            .checked_add(scratch)
            .and_then(|n| n.checked_sub(1))
            .ok_or("fixture scratch limit")?,
        ..bounds()
    };
    let before = inventory(&fixture.root)?;
    let why = Reader::open(
        &fixture.output,
        saved.identity(),
        saved.completion_digest(),
        tight,
    )
    .err()
    .ok_or("insufficient repeated-minute scratch accepted")?;
    assert!(why.contains("repeated-minute validation exceeds independent scratch admission"));
    assert!(codec::validation_bytes(u64::MAX).is_err());
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn index_stop_vix_corrupt_and_locked_reference_months_publish_explicit_unavailability()
-> Result<(), String> {
    for locked in [false, true] {
        let fixture = Fixture::new()?;
        let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
        seed(&fixture, Vendor::Zerodha, 0, false)?;
        let month = YearMonth::new(2025, 5).map_err(display)?;
        let key = InstrumentKey::index(Exchange::Nse, "INDIAVIX").map_err(display)?;
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::MINUTE_1,
            month,
            FileKind::Bars,
        )
        .map_err(display)?;
        let owner = if locked {
            Some(
                BarFile::open_or_create(&fixture.root, path, symbol_id("INDIAVIX"))
                    .map_err(display)?,
            )
        } else {
            fs::write(
                vix_path(&fixture, Vendor::Zerodha)?,
                b"torn reference month",
            )
            .map_err(display)?;
            None
        };
        let context = loaded.prepare(limits())?;
        let saved = crate::index_stop::produce_catalog(
            &fixture.output,
            &loaded,
            &context,
            &programs()?,
            loaded.days(),
            limits(),
        )?;
        let reader = open(&fixture, &saved)?;
        reader.with_current(|view| {
            assert_eq!(view.metadata().exact_stamps, 0);
            assert_eq!(view.metadata().absent_stamps, 0);
            assert!(view.metadata().unavailable_stamps > 0);
            let row = view.page(0, 0, 1)?.rows.first().ok_or("native trade")?;
            assert_eq!(row.entry, Stamp::Unavailable);
            let month = view.month(row.month_index)?;
            assert_eq!(month.snapshot_digest, None);
            assert_eq!(month.unavailable_code(), Some("reference_month_refused"));
            assert!(
                month
                    .unavailable_reason
                    .as_ref()
                    .is_some_and(|reason| !reason.is_empty())
            );
            Ok(())
        })?;
        drop(owner);
    }
    Ok(())
}

#[test]
fn index_stop_vix_full_capture_budget_refuses_before_any_companion_publication()
-> Result<(), String> {
    let (fixture, _, saved) = saved(None)?;
    let directory = directory(&fixture, &saved);
    // Retain the existing generated fixture as an orphan while exercising a
    // fresh namespace admission. Production never moves or removes history.
    fs::rename(&directory, fixture.output.join("retained-vix-fixture")).map_err(display)?;
    let catalog = Catalog::open(
        &fixture.output,
        saved.identity(),
        bounds().bytes,
        bounds().records,
    )?;
    let before = inventory(&fixture.root)?;
    let tiny = Bounds {
        memory_bytes: 1024,
        ..bounds()
    };
    let why = publish(
        &fixture.output,
        &fixture.root,
        Vendor::Zerodha,
        &catalog,
        tiny,
    )
    .err()
    .ok_or("inadmissible companion published")?;
    assert!(why.contains("requires"));
    assert!(!directory.exists());
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn index_stop_vix_compound_projection_excludes_each_writer_and_releases_on_error()
-> Result<(), String> {
    let (fixture, _, saved) = saved(None)?;
    let reader = open(&fixture, &saved)?;
    let native = fixture
        .output
        .join(crate::index_stop_store::NAMESPACE)
        .join(crate::identity_hex(&saved.identity()))
        .join("owner.lock");
    let companion = directory(&fixture, &saved).join("owner.lock");
    let files = [
        crate::readonly_file::open(&native).map_err(display)?,
        crate::readonly_file::open(&companion).map_err(display)?,
    ];
    for failed in [false, true] {
        let projected = reader.with_current(|view| {
            for owner in &files {
                assert!(owner.try_lock().is_err());
            }
            assert!(reader.require_current().is_err());
            assert!(!view.page(0, 0, 1)?.rows.is_empty());
            for owner in &files {
                assert!(owner.try_lock().is_err());
            }
            if failed {
                Err("generated callback failure".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(projected.is_err(), failed);
        for owner in &files {
            owner.try_lock().map_err(display)?;
            owner.unlock().map_err(display)?;
        }
    }
    for owner in &files {
        owner.try_lock().map_err(display)?;
        let mut called = false;
        assert!(
            reader
                .with_current(|_| {
                    called = true;
                    Ok(())
                })
                .is_err()
        );
        assert!(!called);
        owner.unlock().map_err(display)?;
        for other in &files {
            other.try_lock().map_err(display)?;
            other.unlock().map_err(display)?;
        }
    }
    Ok(())
}
