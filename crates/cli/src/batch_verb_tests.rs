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
        assert_eq!(status, crate::FAILED, "{words:?}: {page}");
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
            assert_eq!(status, crate::FAILED, "{page}");
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
    // conc13-7 (D-2643): a refused month fails the walk's exit; otherwise the
    // page's own refusal rule decides.
    assert!(
        arm.contains(
            "let code = if batch::refused_months(&text) > 0 {\n                FAILED\n            } else {\n                work_exit(&text)\n            };\n            out.push_str(&text);\n            code"
        ),
        "{arm}"
    );
}

/// conc13-7 (D-2643): the tally line is read at column zero, its refused
/// count only; an indented or forged line, prose, or a page with no tally
/// reads as zero.
#[test]
fn refused_months_reads_the_tally_line_only() {
    use crate::batch::refused_months;
    assert_eq!(
        refused_months("1 swept · 2 refused · 10 bars · 3 combinations kept\n"),
        2
    );
    assert_eq!(
        refused_months("0 swept · 0 refused · 0 bars · 0 combinations kept"),
        0
    );
    assert_eq!(
        refused_months(&format!(
            "{} swept · {} refused · 0 bars",
            u64::MAX,
            u64::MAX
        )),
        u64::MAX
    );
    assert_eq!(
        refused_months("  1 swept · 2 refused · 0 bars"),
        0,
        "indented"
    );
    assert_eq!(
        refused_months("x1 swept · 2 refused · 0 bars"),
        0,
        "not a count"
    );
    assert_eq!(refused_months(" swept · 2 refused · 0 bars"), 0, "no count");
    assert_eq!(
        refused_months("1 swept · -2 refused · 0 bars"),
        0,
        "negative"
    );
    assert_eq!(
        refused_months("1 swept · 2 refused"),
        0,
        "no trailing tally"
    );
    assert_eq!(
        refused_months("  REFUSED  zerodha NIFTY 1min 2025-05  — torn\n"),
        0
    );
    assert_eq!(refused_months(""), 0);
}
