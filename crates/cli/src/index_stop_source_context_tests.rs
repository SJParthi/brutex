//! Generated complete-calendar fixtures; these are source/authentication tests,
//! never historical profitability evidence or an operator sweep.
use super::*;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use crate::index_stop::tests::{limits, programs, source};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn bounds() -> ReadBounds {
    ReadBounds {
        bytes: 512 * 1024 * 1024,
        records: 400_000,
        source_records: 400_000,
        memory_bytes: 512 * 1024 * 1024,
        page_records: 1000,
    }
}
fn saved() -> Result<(Fixture, Loaded, crate::index_stop::Committed), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let context = loaded.prepare(limits())?;
    let original = publish(
        &fixture.output,
        &loaded,
        &context,
        bounds().bytes,
        bounds().bytes,
    )?;
    let saved = crate::index_stop::produce_catalog_with_context(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
        &original,
    )?;
    Ok((fixture, loaded, saved))
}
fn open(
    fixture: &Fixture,
    saved: &crate::index_stop::Committed,
    setting: usize,
    limit: ReadBounds,
) -> Result<Reader, String> {
    Reader::open(
        &fixture.output,
        saved.identity(),
        saved.completion_digest(),
        setting,
        limit,
    )
    .map_err(display)
}

fn owners(fixture: &Fixture, reader: &Reader) -> Result<[fs::File; 3], String> {
    let meta = reader.metadata();
    let paths = [
        (crate::index_stop_store::NAMESPACE, meta.catalog_identity),
        (
            CATALOG_NAMESPACE,
            catalog_link_identity(meta.catalog_identity, meta.catalog_completion),
        ),
        (NAMESPACE, meta.source_context_identity),
    ]
    .map(|(namespace, identity)| {
        fixture
            .output
            .join(namespace)
            .join(crate::identity_hex(&identity))
            .join("owner.lock")
    });
    let [catalog, relation, context] =
        paths.map(|path| crate::readonly_file::open(&path).map_err(display));
    Ok([catalog?, relation?, context?])
}
fn writers_available(owners: &[fs::File; 3]) -> Result<(), String> {
    for owner in owners {
        owner.try_lock().map_err(display)?;
        owner.unlock().map_err(display)?;
    }
    Ok(())
}
fn writers_blocked(owners: &[fs::File; 3]) {
    for (slot, owner) in owners.iter().enumerate() {
        assert!(
            owner.try_lock().is_err(),
            "source publication owner {slot} became writable inside a complete projection"
        );
    }
}

#[test]
fn original_context_projection_holds_all_three_owners_until_success_and_error_callbacks_finish()
-> Result<(), String> {
    let (fixture, _, saved) = saved()?;
    let mut reader = open(&fixture, &saved, 0, bounds())?;
    let owners = owners(&fixture, &reader)?;
    let before = inventory(&fixture.root)?;
    writers_available(&owners)?;
    let window = reader
        .with_current(|view| {
            writers_blocked(&owners);
            assert_eq!(view.metadata(), reader.metadata());
            assert_eq!(view.condition_names(), reader.condition_names());
            assert_eq!(view.direction(), reader.direction());
            assert_eq!(view.total_bars(), reader.total_bars());
            assert_eq!(view.admitted_bytes(), reader.admitted_bytes());
            assert_eq!(
                view.observation_byte_limit(),
                reader.observation_byte_limit()
            );
            let window = view.window(0, 0, 0).map_err(display)?;
            writers_blocked(&owners);
            assert!(view.window(usize::MAX, 0, 0).is_err());
            writers_blocked(&owners);
            Ok(window)
        })
        .map_err(display)?;
    writers_available(&owners)?;
    assert_eq!(reader.window(0, 0, 0).map_err(display)?, window);
    writers_available(&owners)?;
    let result: Result<(), Refusal> = reader.with_current(|view| {
        writers_blocked(&owners);
        assert_eq!(view.window(0, 0, 0).map_err(display)?, window);
        writers_blocked(&owners);
        Err("generated projection refused after the full window".into())
    });
    let why = result
        .err()
        .ok_or("generated error callback unexpectedly succeeded")?;
    assert!(why.message.contains("generated projection refused"));
    writers_available(&owners)?;
    reader.select_setting(1).map_err(display)?;
    assert_eq!(reader.direction(), Direction::Short);
    assert_eq!(
        reader
            .with_current(|view| Ok(view.metadata().setting))
            .map_err(display)?,
        1
    );
    reader.require_current().map_err(display)?;
    writers_available(&owners)?;
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_busy_owner_prevents_projection_and_releases_earlier_leases()
-> Result<(), String> {
    let (fixture, _, saved) = saved()?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    let owners = owners(&fixture, &reader)?;
    for (busy, held) in owners.iter().enumerate() {
        held.try_lock().map_err(display)?;
        let mut called = false;
        let result = reader.with_current(|_| {
            called = true;
            Ok(())
        });
        assert!(result.is_err());
        assert!(
            !called,
            "projection ran while publication owner {busy} was held by a writer"
        );
        for (slot, other) in owners.iter().enumerate() {
            if slot != busy {
                other.try_lock().map_err(display)?;
                other.unlock().map_err(display)?;
            }
        }
        held.unlock().map_err(display)?;
        writers_available(&owners)?;
        reader.require_current().map_err(display)?;
    }
    assert!(reader.window(0, 0, 0).is_ok());
    writers_available(&owners)?;
    Ok(())
}

#[test]
fn original_context_nested_reader_access_cannot_unlock_a_guarded_view() -> Result<(), String> {
    let (fixture, _, saved) = saved()?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    let owners = owners(&fixture, &reader)?;
    reader
        .with_current(|view| {
            let why = reader
                .require_current()
                .err()
                .ok_or("nested reader check acquired recursive locks")?;
            assert!(why.message.contains("projection is already active"));
            writers_blocked(&owners);
            assert!(reader.window(0, 0, 0).is_err());
            writers_blocked(&owners);
            let mut nested = false;
            assert!(
                reader
                    .with_current(|_| {
                        nested = true;
                        Ok(())
                    })
                    .is_err()
            );
            assert!(!nested);
            assert!(view.window(0, 0, 0).is_ok());
            writers_blocked(&owners);
            Ok(())
        })
        .map_err(display)?;
    writers_available(&owners)?;
    reader.require_current().map_err(display)?;
    Ok(())
}
fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, [u8; 32]>, String> {
    let mut entries = BTreeMap::new();
    for item in fs::read_dir(root).map_err(display)? {
        let item = item.map_err(display)?;
        let path = item.path();
        if item.file_type().map_err(display)?.is_dir() {
            entries.extend(inventory(&path)?);
        } else {
            entries.insert(path.clone(), hash(&fs::read(path).map_err(display)?));
        }
    }
    Ok(entries)
}
fn replace_bytes(body: &mut [u8], from: &[u8], to: &[u8]) -> Result<(), String> {
    if from.len() != to.len() {
        return Err("generated same-width replacement".into());
    }
    let start = body
        .windows(from.len())
        .position(|value| value == from)
        .ok_or("generated replacement absent")?;
    body.get_mut(start..start + from.len())
        .ok_or("generated replacement extent")?
        .copy_from_slice(to);
    Ok(())
}
fn reseal(root: &Path, namespace: &str, identity: [u8; 32], body: &[u8]) -> Result<(), String> {
    // Model an attacker replacing both internally valid payload and completion;
    // CRC/hash correctness alone must not establish original-source ownership.
    let directory = root.join(namespace).join(crate::identity_hex(&identity));
    fs::write(directory.join("body.bin"), body).map_err(display)?;
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&identity);
    receipt.extend_from_slice(&hash(body));
    receipt.extend_from_slice(&(body.len() as u64).to_le_bytes());
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(directory.join("complete.bin"), receipt).map_err(display)
}

#[test]
fn original_context_reads_exact_archived_trades_and_zero_trade_names_without_writes()
-> Result<(), String> {
    let (fixture, loaded, saved) = saved()?;
    let before = inventory(&fixture.root)?;
    for setting in 0..4 {
        let reader = open(&fixture, &saved, setting, bounds())?;
        let row = saved
            .evaluations()
            .get(setting)
            .ok_or("generated setting")?;
        assert_eq!(
            reader.metadata().original_build_commit,
            "generated-index-stop-source-fixture"
        );
        assert_eq!(reader.metadata().feed, "zerodha");
        assert_eq!(reader.metadata().instrument, "NSE-NIFTY");
        assert_eq!(reader.metadata().vocabulary_version, vocab::VOCAB_VERSION);
        assert_eq!(reader.metadata().source_id, row.source_id());
        assert_eq!(reader.metadata().run_id, row.run_id());
        assert_eq!(reader.direction(), row.direction());
        assert_eq!(reader.condition_names().len(), 1);
        assert_eq!(
            reader.condition_names().first().ok_or("original name")?.bit,
            0
        );
        assert!(reader.admitted_bytes() <= reader.observation_byte_limit());
        if let Some(trade) = row.trades().first() {
            let window = reader.window(0, 0, 0).map_err(display)?;
            assert_eq!(window.trade, *trade);
            assert_eq!(window.first_bar, trade.entry_bar);
            assert_eq!(
                window.candles.len() as u64,
                trade.exit_bar - trade.entry_bar + 1
            );
            let start = usize::try_from(trade.entry_bar).map_err(display)?;
            let end = usize::try_from(trade.exit_bar + 1).map_err(display)?;
            assert_eq!(
                window.candles,
                loaded
                    .data
                    .signal
                    .bars
                    .get(start..end)
                    .ok_or("original execution window")?
            );
            assert_eq!(reader.window(0, 0, 0).map_err(display)?, window);
        } else {
            assert!(reader.window(0, 0, 0).is_err());
        }
        reader.require_current().map_err(display)?;
    }
    let mut cached = open(&fixture, &saved, 0, bounds())?;
    let cold_bytes = cached.admitted_bytes();
    cached.select_setting(1).map_err(display)?;
    assert_eq!(cached.metadata().setting, 1);
    assert_eq!(cached.direction(), Direction::Short);
    assert_eq!(
        cached.metadata().run_id,
        saved.evaluations().get(1).ok_or("short setting")?.run_id()
    );
    assert_eq!(cached.admitted_bytes(), cold_bytes);
    cached.select_setting(2).map_err(display)?;
    let selection = cached.metadata().clone();
    assert!(cached.select_setting(usize::MAX).is_err());
    assert_eq!(cached.metadata(), &selection);
    assert!(cached.window(0, 0, 0).is_err());
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_costs_cover_actual_catalog_and_refuse_before_publication() -> Result<(), String>
{
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let context = loaded.prepare(limits())?;
    let programs = programs()?;
    let minimum = minimum_observation(&loaded, programs.len() as u64)?;
    let allowed = minimum.required_bytes.max(minimum.required_memory_bytes);
    assert!(require_minimum_observation(&loaded, programs.len() as u64, allowed).is_ok());
    assert!(require_minimum_observation(&loaded, programs.len() as u64, allowed - 1).is_err());
    let original = publish(&fixture.output, &loaded, &context, bounds().bytes, allowed)?;
    let Err(why) = crate::index_stop::produce_catalog_with_context(
        &fixture.output,
        &loaded,
        &context,
        &programs,
        loaded.days(),
        limits(),
        &original,
    ) else {
        return Err("catalog larger than source-view admission was published".into());
    };
    assert!(why.contains("conservative reconstruction bytes"));
    assert!(
        !fixture
            .output
            .join(crate::index_stop_store::NAMESPACE)
            .exists()
    );
    original.require_current()?;
    assert!(ObservationCost::new(u64::MAX, 1, 1).is_err());
    assert!(ObservationCost::new(1, u64::MAX, 1).is_err());
    assert!(ObservationCost::new(1, 1, u64::MAX / 2).is_err());
    Ok(())
}

#[test]
fn original_context_budget_resolver_is_exact_and_context_binding_rejects_foreign_pins()
-> Result<(), String> {
    assert_eq!(observation_budget_value(None)?, DEFAULT_OBSERVATION_BYTES);
    for raw in [
        "0",
        "00",
        "01",
        "-1",
        "+1",
        "1 ",
        "1e6",
        "18446744073709551615",
    ] {
        assert!(observation_budget_value(Some(std::ffi::OsStr::new(raw))).is_err());
    }
    assert_eq!(
        observation_budget_value(Some(std::ffi::OsStr::new("123")))?,
        123
    );
    let (fixture, _, saved) = saved()?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    let link = Link {
        identity: reader.metadata().source_context_identity,
        completion: reader.metadata().source_context_completion,
    };
    let before = inventory(&fixture.root)?;
    let binding = open_catalog_context(
        &fixture.output,
        saved.identity(),
        saved.completion_digest(),
        link,
        CATALOG_LINK_BYTES,
    )?;
    assert_eq!(binding.link(), link);
    assert_eq!(binding.admitted_bytes(), CATALOG_LINK_BYTES);
    binding.require_current()?;
    assert!(
        open_catalog_context(
            &fixture.output,
            saved.identity(),
            saved.completion_digest(),
            Link {
                identity: [9; 32],
                ..link
            },
            CATALOG_LINK_BYTES
        )
        .is_err()
    );
    assert!(
        open_catalog_context(
            &fixture.output,
            saved.identity(),
            saved.completion_digest(),
            link,
            CATALOG_LINK_BYTES - 1
        )
        .is_err()
    );
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_is_independent_of_changed_current_market_files() -> Result<(), String> {
    let (fixture, loaded, saved) = saved()?;
    let reference = open(&fixture, &saved, 0, bounds())?
        .window(0, 0, 0)
        .map_err(display)?;
    let path = store::path::StorePath::for_key(
        brutex_core::vendor::Vendor::Zerodha,
        &loaded.data.signal.key,
        store::path::Timeframe::MINUTE_1,
        store::path::YearMonth::new(2025, 5).map_err(display)?,
        store::path::FileKind::Bars,
    )
    .map_err(display)?
    .to_path_buf(&fixture.root);
    fs::write(path, b"generated current source is now unavailable").map_err(display)?;
    assert!(loaded.require_current().is_err());
    let before = inventory(&fixture.root)?;
    assert_eq!(
        open(&fixture, &saved, 0, bounds())?
            .window(0, 0, 0)
            .map_err(display)?,
        reference
    );
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_names_are_pinned_not_substituted_by_current_vocabulary() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let context = loaded.prepare(limits())?;
    let mut body = codec::encode(&loaded, &context, bounds().bytes)?;
    let current = vocab::table::TABLE
        .first()
        .ok_or("current fixture name")?
        .name;
    let original = "x".repeat(current.len());
    assert_ne!(original, current);
    replace_bytes(&mut body, current.as_bytes(), original.as_bytes())?;
    let captured = publish_body(
        &fixture.output,
        &loaded,
        &context,
        &body,
        bounds().bytes,
        bounds().bytes,
    )?;
    let saved = crate::index_stop::produce_catalog_with_context(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
        &captured,
    )?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    assert_eq!(
        reader
            .condition_names()
            .first()
            .ok_or("original fixture name")?
            .name,
        original
    );
    assert_ne!(
        reader
            .condition_names()
            .first()
            .ok_or("original fixture name")?
            .name,
        current
    );
    Ok(())
}

#[test]
fn original_context_legacy_missing_refuses_without_changing_existing_catalog() -> Result<(), String>
{
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let context = loaded.prepare(limits())?;
    let saved = crate::index_stop::produce_catalog(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
    )?;
    let before = inventory(&fixture.root)?;
    let Err(why) = Reader::open(
        &fixture.output,
        saved.identity(),
        saved.completion_digest(),
        0,
        bounds(),
    ) else {
        return Err("legacy context was invented".into());
    };
    assert_eq!(why.code, "original_context_unavailable");
    assert!(why.message.contains("no current source or vocabulary"));
    let catalog = Catalog::open(
        &fixture.output,
        saved.identity(),
        bounds().bytes,
        bounds().records,
    )?;
    assert_eq!(
        catalog.record(saved.completion_digest(), 0)?.trades(),
        saved
            .evaluations()
            .first()
            .ok_or("legacy generated run")?
            .trades()
    );
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_foreign_relation_and_changed_build_are_rejected_after_valid_sealing()
-> Result<(), String> {
    let (fixture, loaded, saved) = saved()?;
    let original = open(&fixture, &saved, 0, bounds())?;
    let context = loaded.prepare(limits())?;
    let mut body = codec::encode(&loaded, &context, bounds().bytes)?;
    replace_bytes(
        &mut body,
        b"generated-index-stop-source-fixture",
        b"xenerated-index-stop-source-fixture",
    )?;
    let forged = publish_body(
        &fixture.output,
        &loaded,
        &context,
        &body,
        bounds().bytes,
        bounds().bytes,
    )?;
    let forged_catalog = crate::index_stop::produce_catalog_with_context(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        loaded.days(),
        limits(),
        &forged,
    )?;
    assert!(open(&fixture, &forged_catalog, 0, bounds()).is_err());
    let relation = catalog_link_identity(saved.identity(), saved.completion_digest());
    let path = fixture
        .output
        .join(CATALOG_NAMESPACE)
        .join(crate::identity_hex(&relation))
        .join("body.bin");
    let mut link_body = fs::read(path).map_err(display)?;
    let link = forged.link();
    link_body
        .get_mut(72..104)
        .ok_or("generated link identity")?
        .copy_from_slice(&link.identity);
    link_body
        .get_mut(104..136)
        .ok_or("generated link completion")?
        .copy_from_slice(&link.completion);
    reseal(&fixture.output, CATALOG_NAMESPACE, relation, &link_body)?;
    assert!(original.require_current().is_err());
    assert!(open(&fixture, &saved, 0, bounds()).is_err());
    Ok(())
}

#[test]
fn original_context_partial_torn_and_changed_archives_never_supply_candles() -> Result<(), String> {
    let (fixture, _, saved) = saved()?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    let directory = fixture.output.join(NAMESPACE).join(crate::identity_hex(
        &reader.metadata().source_context_identity,
    ));
    let path = directory.join("body.bin");
    let mut body = fs::read(&path).map_err(display)?;
    let last = body.last_mut().ok_or("generated source body")?;
    *last ^= 1;
    fs::write(path, body).map_err(display)?;
    assert!(reader.require_current().is_err());
    assert!(reader.window(0, 0, 0).is_err());
    assert!(open(&fixture, &saved, 0, bounds()).is_err());
    fs::remove_file(directory.join("complete.bin")).map_err(display)?;
    assert!(open(&fixture, &saved, 0, bounds()).is_err());
    Ok(())
}

#[test]
fn original_context_independent_bounds_and_exact_window_extents_refuse_without_prefixes()
-> Result<(), String> {
    let (fixture, _, saved) = saved()?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    let trade = reader.window(0, 0, 0).map_err(display)?.trade;
    let before = inventory(&fixture.root)?;
    assert!(Reader::open(&fixture.output, saved.identity(), [9; 32], 0, bounds()).is_err());
    assert!(open(&fixture, &saved, usize::MAX, bounds()).is_err());
    for limits in [
        ReadBounds {
            bytes: 1,
            ..bounds()
        },
        ReadBounds {
            records: 1,
            ..bounds()
        },
        ReadBounds {
            source_records: 1,
            ..bounds()
        },
        ReadBounds {
            memory_bytes: 1,
            ..bounds()
        },
        ReadBounds {
            page_records: 0,
            ..bounds()
        },
    ] {
        assert!(open(&fixture, &saved, 0, limits).is_err());
    }
    assert!(reader.window(usize::MAX, 0, 0).is_err());
    assert!(reader.window(0, trade.entry_bar + 1, 0).is_err());
    assert!(reader.window(0, 0, u64::MAX).is_err());
    assert!(reader.window(0, 0, reader.total_bars()).is_err());
    let tight = open(
        &fixture,
        &saved,
        0,
        ReadBounds {
            page_records: 1,
            ..bounds()
        },
    )?;
    assert!(tight.window(0, 0, 1).is_err());
    assert_eq!(inventory(&fixture.root)?, before);
    Ok(())
}

#[test]
fn original_context_codec_rejects_tails_lengths_versions_and_foreign_loader_before_publication()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let loaded = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let duplicate = source(&fixture, "NSE-NIFTY", 5, 5)?;
    let context = loaded.prepare(limits())?;
    let body = codec::encode(&loaded, &context, bounds().bytes)?;
    assert_eq!(encoded_bytes(&loaded)?, body.len() as u64 + 112);
    let before = inventory(&fixture.root)?;
    assert!(
        publish(
            &fixture.output,
            &duplicate,
            &context,
            bounds().bytes,
            bounds().bytes
        )
        .is_err()
    );
    assert!(
        publish(
            &fixture.output,
            &loaded,
            &context,
            body.len() as u64 + 111,
            bounds().bytes
        )
        .is_err()
    );
    assert_eq!(inventory(&fixture.root)?, before);
    assert!(
        codec::decode(
            body.get(..body.len() - 1).ok_or("generated truncate")?,
            bounds()
        )
        .is_err()
    );
    let mut trailing = body.clone();
    trailing.push(0);
    assert!(codec::decode(&trailing, bounds()).is_err());
    let mut version = body.clone();
    version
        .get_mut(..8)
        .ok_or("generated version")?
        .copy_from_slice(b"BRISSC00");
    assert!(codec::decode(&version, bounds()).is_err());
    let mut vocabulary = body.clone();
    vocabulary
        .get_mut(104..108)
        .ok_or("generated vocabulary")?
        .copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(codec::decode(&vocabulary, bounds()).is_err());
    let mut huge_text = body;
    huge_text
        .get_mut(164..172)
        .ok_or("generated text extent")?
        .copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(codec::decode(&huge_text, bounds()).is_err());
    Ok(())
}

fn write_generated_five_minute_source(fixture: &Fixture, minute: &Loaded) -> Result<(), String> {
    let mut coarse = Vec::new();
    for group in minute.data.signal.bars.chunks(5) {
        let first = group.first().ok_or("generated five-minute start")?;
        let last = group.last().ok_or("generated five-minute end")?;
        coarse.push(store::format::Bar {
            ts_micros: first.ts_micros,
            open: first.open,
            high: group
                .iter()
                .map(|bar| bar.high)
                .max()
                .ok_or("generated high")?,
            low: group
                .iter()
                .map(|bar| bar.low)
                .min()
                .ok_or("generated low")?,
            close: last.close,
            volume: group.iter().map(|bar| bar.volume).sum(),
            open_interest: last.open_interest,
        });
    }
    let path = store::path::StorePath::for_key(
        brutex_core::vendor::Vendor::Zerodha,
        &minute.data.signal.key,
        store::path::Timeframe::MINUTE_5,
        store::path::YearMonth::new(2025, 5).map_err(display)?,
        store::path::FileKind::Bars,
    )
    .map_err(display)?;
    let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    let symbol = u32::from_le_bytes(
        hash.get(..4)
            .ok_or("generated symbol")?
            .try_into()
            .map_err(display)?,
    );
    let mut writer =
        store::file::BarFile::open_or_create(&fixture.root, path, symbol).map_err(display)?;
    writer.append(&coarse).map_err(display)?;
    Ok(())
}

#[test]
fn original_context_separate_execution_and_later_measurement_keep_original_causal_prefix()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let minute = source(&fixture, "NSE-NIFTY", 5, 5)?;
    write_generated_five_minute_source(&fixture, &minute)?;
    let strict = crate::audited_range_command::StrictConfig::from_values(
        Some(fixture.root.as_os_str().to_owned()),
        Some(bounds().bytes.to_string().into()),
        Some(bounds().source_records.to_string().into()),
    )
    .map_err(display)?;
    let loaded = Loaded::load_identified(
        crate::index_stop::Request {
            store: &fixture.root,
            vendor: "zerodha",
            underlying: "NSE-NIFTY",
            rung: "5min",
            from: (2025, 5),
            to: (2025, 5),
            strict: &strict,
        },
        "generated-index-stop-source-fixture",
    )?;
    let context = loaded.prepare(limits())?;
    let original = publish(
        &fixture.output,
        &loaded,
        &context,
        bounds().bytes,
        bounds().bytes,
    )?;
    let days = (loaded.days().0 + 7, loaded.days().1);
    let saved = crate::index_stop::produce_catalog_with_context(
        &fixture.output,
        &loaded,
        &context,
        &programs()?,
        days,
        limits(),
        &original,
    )?;
    let reader = open(&fixture, &saved, 0, bounds())?;
    assert_eq!(reader.metadata().timeframe, "5min");
    assert_eq!(reader.metadata().source_first_day, loaded.days().0);
    assert_eq!(reader.metadata().measurement_first_day, days.0);
    assert!(reader.metadata().measurement_first_day > reader.metadata().source_first_day);
    assert_eq!(reader.total_bars(), minute.data.signal.bars.len() as u64);
    let window = reader.window(0, 1, 1).map_err(display)?;
    assert_eq!(
        window.candles.len() as u64,
        window.trade.holding_minutes + 2
    );
    assert_eq!(
        window.trade.signal_close_micros - window.trade.signal_micros,
        5 * 60_000_000
    );
    assert_eq!(window.trade.entry_micros, window.trade.signal_close_micros);
    assert_eq!(
        window
            .candles
            .get(1)
            .ok_or("generated exact entry")?
            .ts_micros,
        window.trade.entry_micros
    );
    Ok(())
}
