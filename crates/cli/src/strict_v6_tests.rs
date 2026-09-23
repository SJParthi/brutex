#[cfg(test)]
mod strict_v6_fixture_tests {
    use super::*;
    // Included in the existing stored fixture module so no fabricated source or
    // build-authority constructor escapes to production callers.

    fn corrupt_strict_fixture_byte(path: &Path, index: usize) -> Result<(), String> {
        let mut bytes = fs::read(path).map_err(|why| why.to_string())?;
        *bytes
            .get_mut(index)
            .ok_or("fixture byte outside acknowledged source")? ^= 1;
        fs::write(path, bytes).map_err(|why| why.to_string())
    }

    fn strict_fixture_family(
        fixture: &StoredSuccessFixture,
        family: &str,
        evaluated: bool,
    ) -> Result<family_v6::StoredFamilyV6, String> {
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let diagnostic = Sweeper::new(engine::Ladder::with_min_hits(1));
        let support = if evaluated {
            maximum_fixture_singleton_support(&fixture_request(
                &fixture.source,
                family,
                &diagnostic,
                &long,
                &short,
            )?)?
        } else {
            1_000_000
        };
        strict_fixture_family_at(fixture, family, support, evaluated)
    }

    fn strict_fixture_family_at(
        fixture: &StoredSuccessFixture,
        family: &str,
        support: u64,
        evaluated: bool,
    ) -> Result<family_v6::StoredFamilyV6, String> {
        let config = strict_fixture_config(fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(support));
        let produced = commit_family_with_inputs_v6(
            fixture_request(&fixture.source, family, &sweeper, &long, &short)?,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
            Some(&config),
            true,
        )?;
        assert_eq!(produced.evaluated().is_some(), evaluated);
        assert_eq!(produced.candidate_audit().row_count() != 0, evaluated);
        produced.require_current()?;
        Ok(produced)
    }

    fn reset_strict_fixture_pattern(
        fixture: &StoredSuccessFixture,
        family: &str,
        flat: bool,
    ) -> Result<u64, String> {
        let key = crate::stored::swept_index(family)?;
        let symbol = brutex_core::universe::fnv1a(family).to_le_bytes();
        let symbol_id = u32::from_le_bytes(
            symbol
                .get(..4)
                .ok_or("fixture symbol prefix")?
                .try_into()
                .map_err(|_| "fixture symbol width")?,
        );
        for month in [FIXTURE_WARM_MONTH, FIXTURE_FROM] {
            let (mut minute, mut daily) = fixture_month_bars(month.0, month.1, 2_000_000)?;
            if !flat {
                // A real nonconstant body-up condition with one opposite body;
                // prices, wick ranges and actual session returns remain varied.
                let opposite = minute.len() / 2;
                for (index, bar) in minute.iter_mut().enumerate() {
                    bar.close = bar.open + if index == opposite { -1 } else { 1 };
                }
                for bar in &mut daily {
                    let day = indicators::ist_day(bar.ts_micros);
                    bar.close = minute
                        .iter()
                        .rfind(|minute| indicators::ist_day(minute.ts_micros) == day)
                        .ok_or("fixture daily close has no exact minute")?
                        .close;
                }
            }
            for (rung, mut bars) in [(Timeframe::MINUTE_1, minute), (Timeframe::DAY_1, daily)] {
                let path = StorePath::for_key(
                    Vendor::Zerodha,
                    &key,
                    rung,
                    YearMonth::new(month.0, month.1).map_err(|why| why.to_string())?,
                    FileKind::Bars,
                )
                .map_err(|why| why.to_string())?;
                fs::remove_file(path.to_path_buf(&fixture.source)).map_err(|why| why.to_string())?;
                if flat {
                    for bar in &mut bars {
                        bar.open = 2_000_000;
                        bar.close = 2_000_000;
                        bar.high = 2_000_100;
                        bar.low = 1_999_900;
                    }
                }
                let mut file = BarFile::open_or_create(&fixture.source, path, symbol_id)
                    .map_err(|why| why.to_string())?;
                file.append(&bars).map_err(|why| why.to_string())?;
            }
        }
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1));
        maximum_fixture_singleton_support(&fixture_request(
            &fixture.source,
            family,
            &sweeper,
            &exit_policy(Side::Long)?,
            &exit_policy(Side::Short)?,
        )?)
    }

    // Both families use ONE ladder policy. Only generated stored OHLC inputs
    // differ, which is the actual mixed-family situation the adapter must handle.
    fn strict_fixture_shape(
        nifty: bool,
        banknifty: bool,
    ) -> Result<(StoredSuccessFixture, u64), String> {
        let fixture = StoredSuccessFixture::new()?;
        let first = (
            false,
            reset_strict_fixture_pattern(&fixture, "NIFTY", false)?,
        );
        let other = (
            true,
            reset_strict_fixture_pattern(&fixture, "BANKNIFTY", true)?,
        );
        if first.1 == other.1 {
            return Err(
                "bounded generated fixture could not form different singleton maxima".to_owned(),
            );
        }
        let (high, low) = if first.1 > other.1 {
            (first, other)
        } else {
            (other, first)
        };
        let actual_nifty =
            reset_strict_fixture_pattern(&fixture, "NIFTY", if nifty { high.0 } else { low.0 })?;
        let actual_banknifty = reset_strict_fixture_pattern(
            &fixture,
            "BANKNIFTY",
            if banknifty { high.0 } else { low.0 },
        )?;
        assert_eq!(actual_nifty == high.1, nifty);
        assert_eq!(actual_banknifty == high.1, banknifty);
        Ok((fixture, high.1))
    }

    #[test]
    fn strict_v6_both_evaluated_and_both_mixed_shapes_reach_real_selection() -> Result<(), String> {
        for (nifty_evaluated, banknifty_evaluated) in [(true, true), (true, false), (false, true)] {
            let (fixture, support) = strict_fixture_shape(nifty_evaluated, banknifty_evaluated)?;
            let nifty = strict_fixture_family_at(&fixture, "NIFTY", support, nifty_evaluated)?;
            let banknifty =
                strict_fixture_family_at(&fixture, "BANKNIFTY", support, banknifty_evaluated)?;
            let expected = (nifty.candidate_audit(), banknifty.candidate_audit());
            let mut selection = crate::ledger_v6::strict_fixture_selection(
                &fixture.base,
                nifty,
                banknifty,
                &population_admission_policy()?,
            )?;
            let snapshot = selection.snapshot()?;
            assert_eq!(
                snapshot,
                selection.snapshot()?,
                "fresh authentication is deterministic"
            );
            assert_eq!(snapshot.rung_seconds, 60);
            assert_ne!(snapshot.identity, [0; 32]);
            assert_ne!(expected.0.universe_id(), expected.1.universe_id());
            let pre_admission = fixture.source.join("pre-admission-data-v2.bin");
            let original = fs::read(&pre_admission).map_err(|why| why.to_string())?;
            corrupt_strict_fixture_byte(
                &pre_admission,
                original
                    .len()
                    .checked_sub(1)
                    .ok_or("missing V2 completion")?,
            )?;
            assert!(
                selection.snapshot().is_err(),
                "projected family must retain V2 authority"
            );
            fs::write(&pre_admission, original).map_err(|why| why.to_string())?;
            assert_eq!(snapshot, selection.snapshot()?);
            // The family without selected rows remains protected too. Its raw
            // source may not detach when the later selected prefix is projected.
            let key = crate::stored::swept_index("BANKNIFTY")?;
            let path = StorePath::for_key(
                Vendor::Zerodha,
                &key,
                Timeframe::DAY_1,
                YearMonth::new(2025, 8).map_err(|why| why.to_string())?,
                FileKind::Bars,
            )
            .map_err(|why| why.to_string())?
            .to_path_buf(&fixture.source);
            corrupt_strict_fixture_byte(&path, 700)?;
            assert!(
                selection.snapshot().is_err(),
                "shape {nifty_evaluated}/{banknifty_evaluated}"
            );
        }
        Ok(())
    }

    #[test]
    fn strict_v6_retained_empty_and_evaluated_families_reopen_every_saved_support_record()
    -> Result<(), String> {
        for evaluated in [false, true] {
            let fixture = StoredSuccessFixture::new()?;
            let family = strict_fixture_family(&fixture, "NIFTY", evaluated)?;
            let mut files = vec![
                "candidate-universe-completions-v1.bin",
                "base-evidence-completions-v2.bin",
                "pre-admission-data-v2.bin",
            ];
            if evaluated {
                files.push("pre-admission-data-v1.bin");
            }
            for file in files {
                let path = fixture.source.join(file);
                let original = fs::read(&path).map_err(|why| why.to_string())?;
                let last = original
                    .len()
                    .checked_sub(1)
                    .ok_or("empty acknowledged ledger")?;
                corrupt_strict_fixture_byte(&path, last)?;
                assert!(
                    family.require_current().is_err(),
                    "cached {file} authority must be freshly checked, evaluated={evaluated}"
                );
                fs::write(&path, original).map_err(|why| why.to_string())?;
                family.require_current()?;
            }
        }
        Ok(())
    }

    #[test]
    fn strict_v6_oos_witness_keeps_actual_later_source_guard_after_cohort_is_dropped()
    -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        seed_stored_family_month(
            &fixture.source,
            Vendor::Zerodha,
            "NIFTY",
            FIXTURE_OOS_MONTH,
            2_000_000,
        )?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let diagnostic = Sweeper::new(engine::Ladder::with_min_hits(1));
        let support = maximum_fixture_singleton_support(&fixture_request(
            &fixture.source,
            "NIFTY",
            &diagnostic,
            &long,
            &short,
        )?)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(support));
        let committed = commit_stored_with_inputs_v1(
            fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
            Some(&config),
        )?;
        let disposition = first_selected_disposition(&committed)?;
        let cohort = committed.stored_post_training_oos_cohort(fixture_oos_request()?)?;
        let witness = cohort.mint_witness_recorded(&disposition, &mut |_| Ok(()))?;
        let held = strict::Guards::from_sources([witness.strict_inputs()]);
        drop(cohort);
        let (_, _, runner_witness) = witness.into_parts();
        runner_witness
            .require_integrity()
            .map_err(|why| format!("fixture witness: {why:?}"))?;
        held.require_current()?;
        let key = crate::stored::swept_index("NIFTY")?;
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::DAY_1,
            YearMonth::new(2025, 10).map_err(|why| why.to_string())?,
            FileKind::Bars,
        )
        .map_err(|why| why.to_string())?
        .to_path_buf(&fixture.source);
        corrupt_strict_fixture_byte(&path, 700)?;
        assert!(
            held.require_current().is_err(),
            "actual later daily source cannot detach at witness projection"
        );
        Ok(())
    }

    #[test]
    fn strict_v6_extinct_selection_keeps_both_family_sources_through_final_reauthentication()
    -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
        let mut families = Vec::new();
        for family in ["NIFTY", "BANKNIFTY"] {
            families.push(commit_family_with_inputs_v6(
                fixture_request(&fixture.source, family, &sweeper, &long, &short)?,
                VerifiedBuildCommitV1(FIXTURE_COMMIT),
                &|_, _, _| {},
                Some(&config),
                true,
            )?);
        }
        let [nifty, banknifty]: [_; 2] = families
            .try_into()
            .map_err(|_| "exact two fixture families")?;
        let mut selection = crate::ledger_v6::strict_fixture_selection(
            &fixture.base,
            nifty,
            banknifty,
            &population_admission_policy()?,
        )?;
        assert!(
            selection.snapshot()?.winners.is_empty(),
            "actual empty selected prefix"
        );
        let key = crate::stored::swept_index("BANKNIFTY")?;
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::DAY_1,
            YearMonth::new(2025, 8).map_err(|why| why.to_string())?,
            FileKind::Bars,
        )
        .map_err(|why| why.to_string())?
        .to_path_buf(&fixture.source);
        corrupt_strict_fixture_byte(&path, 700)?;
        assert!(
            selection.snapshot().is_err(),
            "empty BANKNIFTY's consumed daily source remains authoritative"
        );
        Ok(())
    }

    fn strict_fixture_config(
        fixture: &StoredSuccessFixture,
    ) -> Result<crate::audited_range_command::StrictConfig, String> {
        let receipts = fixture.base.join("strict-receipts");
        fs::create_dir_all(&receipts).map_err(|why| why.to_string())?;
        crate::audited_range_command::StrictConfig::from_values(
            Some(receipts.into()),
            Some("4194304".into()),
            Some("100000".into()),
        )
        .map_err(|why| why.to_string())
    }

    #[test]
    fn strict_v6_independent_role_caps_refuse_before_candidate_computation() -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
        for role in 0..3 {
            let mut request = fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?;
            let one = StoredSpanLoadBoundV1::new(1)?;
            match role {
                0 => request.bounds.signal_records = one,
                1 => request.bounds.minute_records = one,
                _ => request.bounds.daily_records = one,
            }
            let invoked = std::cell::Cell::new(false);
            let result = commit_stored_with_inputs_v1(
                request,
                VerifiedBuildCommitV1(FIXTURE_COMMIT),
                &|_, _, _| {
                    invoked.set(true);
                },
                Some(&config),
            );
            assert!(
                matches!(result, Err(why) if why.contains("raw-record ceiling before allocation")),
                "role {role}"
            );
            assert!(!invoked.get());
            assert!(
                !fixture
                    .source
                    .join("candidate-universe-completions-v1.bin")
                    .exists()
            );
        }
        Ok(())
    }

    #[test]
    fn strict_v6_same_real_source_reuses_and_receipt_policy_rekeys_ordinary_identity()
    -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let diagnostic = Sweeper::new(engine::Ladder::with_min_hits(1));
        let support = maximum_fixture_singleton_support(&fixture_request(
            &fixture.source,
            "NIFTY",
            &diagnostic,
            &long,
            &short,
        )?)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(support));
        let request = fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?;
        let ordinary = commit_stored_with_verified_build_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
        )?;
        let strict = commit_stored_with_inputs_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
            Some(&config),
        )?;
        let repeated = commit_stored_with_inputs_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
            Some(&config),
        )?;
        assert_eq!(strict.identities(), repeated.identities());
        assert_ne!(
            ordinary.identities().candidate_universe_id(),
            strict.identities().candidate_universe_id()
        );
        assert!(strict.identities().candidate_row_count() > 0);
        assert!(strict.strict_inputs().is_some());
        strict.root.require_same("strict fixture terminal")?;
        let changed = crate::audited_range_command::StrictConfig::from_values(
            Some(config.receipt_root().as_os_str().to_owned()),
            Some((config.max_bytes() + 1).to_string().into()),
            Some(config.max_records().to_string().into()),
        )
        .map_err(|why| why.to_string())?;
        let changed = commit_stored_with_inputs_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {},
            Some(&changed),
        )?;
        assert_ne!(
            strict.identities().candidate_universe_id(),
            changed.identities().candidate_universe_id()
        );
        Ok(())
    }

    #[test]
    fn strict_v6_every_consumed_source_and_receipt_replacement_invalidates_retained_authority()
    -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
        let request = fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?;
        let root = AdmittedRootV1::admit(&fixture.source)?;
        for month in [FIXTURE_WARM_MONTH, FIXTURE_FROM] {
            for timeframe in [Timeframe::MINUTE_1, Timeframe::DAY_1] {
                let context = load_bounded_stored_context_v1(&request, &root, Some(&config))?;
                let key = crate::stored::swept_index("NIFTY")?;
                let path = StorePath::for_key(
                    Vendor::Zerodha,
                    &key,
                    timeframe,
                    YearMonth::new(month.0, month.1).map_err(|why| why.to_string())?,
                    FileKind::Bars,
                )
                .map_err(|why| why.to_string())?
                .to_path_buf(&fixture.source);
                let replacement = path.with_extension("replacement");
                let original = path.with_extension("held");
                fs::copy(&path, &replacement).map_err(|why| why.to_string())?;
                fs::rename(&path, &original).map_err(|why| why.to_string())?;
                fs::rename(&replacement, &path).map_err(|why| why.to_string())?;
                assert!(
                    context.require_current().is_err(),
                    "same bytes / different source inode {path:?}"
                );
                drop(context);
                fs::remove_file(&path).map_err(|why| why.to_string())?;
                fs::rename(&original, &path).map_err(|why| why.to_string())?;
            }
        }
        let context = load_bounded_stored_context_v1(&request, &root, Some(&config))?;
        let receipts = config.receipt_root().join("checksum-receipts-v1");
        let receipt = fs::read_dir(receipts)
            .map_err(|why| why.to_string())?
            .find_map(|entry| {
                entry
                    .ok()
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().is_some_and(|extension| extension == "bin"))
            })
            .ok_or("acknowledged source receipt absent")?;
        corrupt_strict_fixture_byte(&receipt, 100)?;
        assert!(context.require_current().is_err());
        Ok(())
    }

    #[test]
    fn strict_v6_source_change_during_actual_ladder_refuses_before_candidate_publication()
    -> Result<(), String> {
        let fixture = StoredSuccessFixture::new()?;
        let config = strict_fixture_config(&fixture)?;
        let long = exit_policy(Side::Long)?;
        let short = exit_policy(Side::Short)?;
        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
        let request = fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?;
        let invoked = std::cell::Cell::new(false);
        let start = std::cell::Cell::new(None);
        let hook_failure = std::cell::RefCell::new(None);
        let key = crate::stored::swept_index("NIFTY")?;
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::DAY_1,
            YearMonth::new(2025, 8).map_err(|why| why.to_string())?,
            FileKind::Bars,
        )
        .map_err(|why| why.to_string())?
        .to_path_buf(&fixture.source);
        let result = commit_stored_with_inputs_v1(
            request,
            VerifiedBuildCommitV1(FIXTURE_COMMIT),
            &|_, _, _| {
                if !invoked.replace(true) {
                    let mutate = || -> Result<(), String> {
                        let evidence =
                            crate::sweep_evidence::latest(&fixture.source, 16 * 1024 * 1024)?
                                .ok_or("no durable start before actual ladder")?;
                        assert_eq!(
                            evidence.completion,
                            crate::sweep_evidence::Completion::Running
                        );
                        start.set(Some((evidence.identity, evidence.attempt)));
                        corrupt_strict_fixture_byte(&path, 700)
                    };
                    *hook_failure.borrow_mut() = mutate().err();
                }
            },
            Some(&config),
        );
        assert!(
            hook_failure.borrow().is_none(),
            "controlled source mutation failed: {:?}",
            hook_failure.borrow()
        );
        assert!(
            invoked.get(),
            "actual production ladder must reach the observer"
        );
        assert!(
            result.is_err(),
            "changed consumed daily data cannot publish success"
        );
        let (identity, attempt) = start.get().ok_or("no attempt captured")?;
        let evidence =
            crate::sweep_evidence::read_attempt(&fixture.source, identity, attempt, 16 * 1024 * 1024)?
                .ok_or("missing terminal")?;
        assert_eq!(
            evidence.completion,
            crate::sweep_evidence::Completion::Refused
        );
        for file in [
            "candidate-universe-rows-v1.bin",
            "candidate-universe-completions-v1.bin",
        ] {
            assert!(
                !fixture.source.join(file).exists(),
                "candidate publication was blocked: {file}"
            );
        }
        Ok(())
    }
}
