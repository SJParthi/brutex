#![cfg(test)]
//! Dispatch and refusal spelling of the batch verb (D-0696).

#![allow(clippy::expect_used, clippy::unwrap_used)]

/// The same malformed generated store reaches both verbs through dispatch.
/// A clean build checks the real exit; an unstamped build checks its refusal.
#[test]
fn sweep_all_and_pool_dispatch_refuse_an_all_misfiled_store()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "BRUTEX_TEST_BATCH_VERB";
    if std::env::var_os(CHILD).is_some() {
        return dispatch_refusals();
    }
    let root = std::env::temp_dir().join(format!("brutex-batch-verb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for dir in ["BSE/CASH/RELIANCE", "NSE/INDEX/RELIANCE"] {
        let at = root.join(format!("bars/zerodha/{dir}/60min"));
        std::fs::create_dir_all(&at)?;
        for month in ["2026-06.bin", "2026-07.bin"] {
            std::fs::write(at.join(month), b"generated unreadable month")?;
        }
    }
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child.args([
        "--exact",
        "batch::verb_tests::sweep_all_and_pool_dispatch_refuse_an_all_misfiled_store",
        "--nocapture",
        "--test-threads=1",
    ]);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("BRUTEX_") {
            child.env_remove(name);
        }
    }
    let output = child
        .env(CHILD, "generated")
        .env("BRUTEX_STORE", &root)
        .env("BRUTEX_LOG_DIR", root.join("logs"))
        .output()?;
    std::fs::remove_dir_all(root)?;
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    Ok(())
}

fn dispatch_refusals() -> Result<(), Box<dyn std::error::Error>> {
    let root = crate::store_root()?;
    let holdings = store::catalog::walk(&root)?;
    assert_eq!(
        holdings.held.len(),
        4,
        "the generated store offers four months"
    );
    for words in [
        vec!["sweep-all", "zerodha", "60min", "100"],
        vec!["pool", "zerodha", "60min", "2026", "6", "2026", "7", "auto"],
    ] {
        let args: Vec<String> = words.iter().map(ToString::to_string).collect();
        let mut page = String::new();
        let status = crate::dispatch(&args, &mut page);
        assert_eq!(status, crate::MISUSED, "{words:?}: {page}");
        assert!(page.starts_with("refused: "), "{page}");
        assert!(crate::carries_refusal(&page), "{page}");
        assert!(!page.contains(crate::STORED_PROVENANCE), "{page}");
        if crate::commit_stamp().is_none() {
            assert!(page.starts_with("refused: this build carries no verified commit stamp"));
        } else {
            for dir in ["BSE/CASH/RELIANCE", "NSE/INDEX/RELIANCE"] {
                assert!(page.contains(dir), "{dir}: {page}");
            }
            if words.first() == Some(&"sweep-all") {
                assert!(
                    page.starts_with("refused: none of the 4 month(s) offered"),
                    "{page}"
                );
                assert_eq!(page.matches("\n  REFUSED  ").count(), 4, "{page}");
            } else {
                assert!(
                    page.starts_with("refused: no instrument is on the surface"),
                    "{page}"
                );
                assert!(
                    page.contains("NOT ON THE SURFACE: 2 held director(ies)"),
                    "{page}"
                );
            }
        }
    }
    assert!(!crate::results::Results::path(&root).exists());
    for words in [
        vec!["sweep-all", "groww", "60min", "100"],
        vec!["pool", "groww", "60min", "2026", "6", "2026", "7", "auto"],
    ] {
        let args: Vec<String> = words.iter().map(ToString::to_string).collect();
        let mut page = String::new();
        let status = crate::dispatch(&args, &mut page);
        if crate::commit_stamp().is_some() {
            assert_eq!(status, crate::OK, "a feed with no holdings: {page}");
            assert!(!crate::carries_refusal(&page), "{page}");
        } else {
            assert_eq!(status, crate::MISUSED, "{page}");
            assert!(page.starts_with("refused: this build carries no verified commit stamp"));
        }
    }
    Ok(())
}

/// Mutation copies are unstamped, so pin the real wrapper even in that build.
#[test]
fn sweep_all_keeps_the_column_zero_refusal_and_dispatch_status() {
    let source = include_str!("batch.rs");
    let body = source.split_once("pub fn sweep_all(").expect("wrapper").1;
    let body = body.split_once("\n}\n").expect("wrapper end").0;
    assert!(body.ends_with(
        "    match run(vendor_word, rung, min_hits) {\n        Ok(text) => text,\n        Err(why) => format!(\"refused: {why}\\n\"),\n    }"
    ), "{body}");
    let lib = include_str!("lib.rs");
    let arm = lib.split_once("fn sweep_all_arm(").expect("arm").1;
    let arm = arm.split_once("\n}\n").expect("arm end").0;
    assert!(
        arm.contains("let text = batch::sweep_all(vendor, rung, h);"),
        "{arm}"
    );
    assert!(arm.contains("let refused = carries_refusal(&text);\n            out.push_str(&text);\n            if refused { MISUSED } else { OK }"), "{arm}");
}

/// W2-cli1-5, D-0968: a batch month's identity is NOT a `sweep-stored`
/// identity, and the two facts that make it different are pinned here so the
/// comment on `id` in `one` cannot drift back into promising equal hex.
#[test]
fn the_batch_identity_is_built_from_terms_sweep_stored_does_not_use() {
    let source = include_str!("batch.rs");
    let one = source.split_once("\nfn one(").expect("one").1;
    let one = one.split_once("\n}\n").expect("one end").0;
    assert!(one.contains("crate::stored_anchored_digest(&loaded.bars, &exact_minute, &daily)"));
    assert!(!one.contains("stored_executed_digest("), "{one}");
    assert!(one.contains(".with_ceiling(BATCH_CEILING)"), "{one}");
    assert!(one.contains("NOT THE SAME RUN AS `sweep-stored`"), "{one}");
    let promise = ["same 64 hex", " characters"].concat();
    assert!(
        !source.contains(&promise),
        "the false parity promise is gone"
    );

    let lib = include_str!("lib.rs");
    let kernel = lib.split_once("fn stored_month_kernel(").expect("kernel").1;
    let kernel = kernel.split_once("\n}\n").expect("kernel end").0;
    assert!(
        kernel.contains("let ladder = ladder_for(min_hits)?;"),
        "{kernel}"
    );
    assert!(
        kernel.contains("stored_executed_digest(&loaded.bars"),
        "{kernel}"
    );
}
