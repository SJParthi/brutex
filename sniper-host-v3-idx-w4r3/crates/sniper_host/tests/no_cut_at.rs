//! S2 / mutant m-S16: the host NEVER references `FillPath::cut_at` (MAIN 09:12:00: crate v2's FU1 must be fixed in a crate v3
//! before any host links it). Red build test: the host's sources (the named mutant file excepted, which exists only to prove this
//! test fails, and only while that mutant is OFF: under `--features m_s16` it is read and must fail this test) and the built
//! binary must not name it.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, missing_docs)]

use std::path::Path;

fn needle() -> String {
    ["cut", "_at"].concat()
}

#[test]
fn host_sources_never_name_it() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src];
    let mut hits = Vec::new();
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") && !(p.ends_with("mutant_s16.rs") && !cfg!(feature = "m_s16")) {
                // HOST REPAIR B1 (check-build-host-v1 section 7): the mutant file is skipped ONLY when the mutant is off, so a build
                // with m_s16 FAILS here (the binary check alone misses it: `FillPath::cut_at` is inlined, its name never reaches the
                // binary).
                let t = std::fs::read_to_string(&p).unwrap();
                for (i, l) in t.lines().enumerate() {
                    let code = l.split("//").next().unwrap_or("");
                    if code.contains(&needle()) {
                        hits.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "the host references the forbidden function: {hits:?}");
}

#[test]
fn host_binary_never_links_it() {
    let bin = std::fs::read(env!("CARGO_BIN_EXE_sniper-host")).unwrap();
    let n = needle();
    let found = bin.windows(n.len()).any(|w| w == n.as_bytes());
    assert!(!found, "the host binary carries a symbol with the forbidden name (m-S16)");
}
