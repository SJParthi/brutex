#![cfg(test)]
//! Receipt-last publication: scratch from a cut-short attempt is rewritten,
//! committed history is compared and kept (D-1760).
use super::{committed, lost_owner_race, prepare_in_namespace, read_exact, write_or_equal};
use std::fs;
use std::path::PathBuf;

/// A private root, named for its owner so two tests cannot collide.
fn scratch(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!("brutex-persist-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    assert!(
        fs::create_dir_all(&root).is_ok(),
        "scratch root is creatable"
    );
    root
}

#[test]
fn a_fresh_write_lands_whole_and_an_identical_retry_is_accepted() -> Result<(), String> {
    let root = scratch("fresh");
    let path = root.join("body.bin");
    write_or_equal(&path, b"0123456789")?;
    assert_eq!(read_exact(&path, 10)?, b"0123456789");
    write_or_equal(&path, b"0123456789")?;
    assert_eq!(read_exact(&path, 10)?, b"0123456789");
    Ok(())
}

#[test]
fn an_existing_file_with_other_bytes_is_refused_and_kept() -> Result<(), String> {
    let root = scratch("refuse");
    for (name, existing) in [
        ("prefix", &b"01234"[..]),
        ("diverged", &b"01x34"[..]),
        ("same-length", &b"012345678X"[..]),
        ("longer", &b"0123456789AB"[..]),
        ("one-longer", &b"0123456789A"[..]),
    ] {
        let path = root.join(name);
        fs::write(&path, existing).map_err(|why| why.to_string())?;
        assert!(write_or_equal(&path, b"0123456789").is_err(), "{name}");
        assert_eq!(
            fs::read(&path).map_err(|why| why.to_string())?,
            existing,
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn a_publication_cut_short_after_its_directory_resumes_to_a_receipt() -> Result<(), String> {
    let root = scratch("resume");
    let identity = [7_u8; 32];
    let body = b"the whole intended body".to_vec();
    let directory = root
        .join("index-stop-vix-reference-v1")
        .join(crate::identity_hex(&identity));
    fs::create_dir_all(&directory).map_err(|why| why.to_string())?;
    fs::write(
        directory.join("body.bin"),
        body.get(..9).ok_or("a ten-byte body")?,
    )
    .map_err(|why| why.to_string())?;
    let pending = prepare_in_namespace(&root, "index-stop-vix-reference-v1", identity, &body)?;
    let digest = brutex_core::blake3::hash(&body);
    pending.verify_body(digest, body.len() as u64)?;
    let completion = pending.finish(identity, digest, body.len() as u64)?;
    drop(pending);
    super::verify(&directory, identity, digest, body.len() as u64, completion)
}

fn publication(root: &std::path::Path, identity: [u8; 32]) -> PathBuf {
    root.join("index-stop-vix-reference-v1")
        .join(crate::identity_hex(&identity))
}

fn publish(root: &std::path::Path, identity: [u8; 32], body: &[u8]) -> Result<[u8; 32], String> {
    let pending = prepare_in_namespace(root, "index-stop-vix-reference-v1", identity, body)?;
    let digest = brutex_core::blake3::hash(body);
    pending.verify_body(digest, body.len() as u64)?;
    pending.finish(identity, digest, body.len() as u64)
}

#[test]
fn an_unreceipted_body_with_other_bytes_is_scratch_and_is_rewritten() -> Result<(), String> {
    let root = scratch("scratch-body");
    let identity = [9_u8; 32];
    let directory = publication(&root, identity);
    fs::create_dir_all(&directory).map_err(|why| why.to_string())?;
    // A whole body from an earlier capture, and a receipt torn at 50 bytes.
    fs::write(
        directory.join("body.bin"),
        b"an earlier, longer capture of the store",
    )
    .map_err(|why| why.to_string())?;
    fs::write(directory.join("complete.bin"), [1_u8; 50]).map_err(|why| why.to_string())?;
    assert!(!committed(&directory)?);
    let completion = publish(&root, identity, b"this capture")?;
    assert!(committed(&directory)?);
    assert_eq!(
        read_exact(&directory.join("body.bin"), 64)?,
        b"this capture"
    );
    let digest = brutex_core::blake3::hash(b"this capture");
    super::verify(&directory, identity, digest, 12, completion)
}

#[test]
fn a_committed_body_is_history_and_other_bytes_are_refused() -> Result<(), String> {
    let root = scratch("committed");
    let identity = [5_u8; 32];
    publish(&root, identity, b"published once")?;
    let directory = publication(&root, identity);
    assert!(committed(&directory)?);
    assert!(publish(&root, identity, b"published twice").is_err());
    assert_eq!(
        read_exact(&directory.join("body.bin"), 64)?,
        b"published once"
    );
    publish(&root, identity, b"published once")?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unreceipted_body_link_is_unlinked_never_followed() -> Result<(), String> {
    let root = scratch("link");
    let identity = [3_u8; 32];
    let directory = publication(&root, identity);
    fs::create_dir_all(&directory).map_err(|why| why.to_string())?;
    let target = root.join("outside.bin");
    fs::write(&target, b"must survive").map_err(|why| why.to_string())?;
    std::os::unix::fs::symlink(&target, directory.join("body.bin"))
        .map_err(|why| why.to_string())?;
    publish(&root, identity, b"real body")?;
    assert_eq!(
        fs::read(&target).map_err(|why| why.to_string())?,
        b"must survive"
    );
    assert_eq!(read_exact(&directory.join("body.bin"), 64)?, b"real body");
    Ok(())
}

/// ledgers-1, D-1908: only the owner-lock refusal, which wrote nothing, is
/// the race a committed receipt may answer. A refusal of anything this call
/// did itself is never mistaken for it.
#[test]
fn only_the_owner_lock_refusal_is_a_lost_race() -> Result<(), String> {
    let root = scratch("lost-race");
    let identity = [11_u8; 32];
    let held = prepare_in_namespace(&root, "index-stop-vix-reference-v1", identity, b"winner")?;
    let refusal = prepare_in_namespace(&root, "index-stop-vix-reference-v1", identity, b"loser")
        .err()
        .unwrap_or_default();
    drop(held);
    assert!(lost_owner_race(&refusal), "{refusal}");
    for own in [
        "Input/output error (os error 5)",
        "Boolean evidence already exists with different bytes; history preserved",
        "",
    ] {
        assert!(!lost_owner_race(own), "{own}");
    }
    Ok(())
}

/// expr-2, indexstop-2, D-2620: a reader's shared lease on a committed
/// child's `owner.lock` — what a dashboard projection holds — no longer
/// refuses a re-publication of the same bytes, in both namespaces the
/// findings name. On the old code `prepare_in_namespace` made one `try_lock`
/// and refused with `OWNER_REFUSED` while the reader held it, so the first
/// re-publication below failed. A holder past the bound is still refused, by
/// the prefix `lost_owner_race` reads, and the body is unchanged.
#[test]
fn a_reader_lease_does_not_refuse_a_republish_of_a_committed_identity() -> Result<(), String> {
    for namespace in ["boolean-candidates-v1", "index-stop-vix-reference-v1"] {
        let root = scratch(&format!("reader-{namespace}"));
        let identity = [17_u8; 32];
        let body = b"content-addressed child".to_vec();
        let digest = brutex_core::blake3::hash(&body);
        let republish = || {
            let pending = prepare_in_namespace(&root, namespace, identity, &body)?;
            pending.verify_body(digest, body.len() as u64)?;
            pending.finish(identity, digest, body.len() as u64)
        };
        republish()?;
        let directory = root.join(namespace).join(crate::identity_hex(&identity));
        let reader = fs::File::open(directory.join("owner.lock")).map_err(|why| why.to_string())?;
        reader.try_lock_shared().map_err(|why| why.to_string())?;
        let again = std::thread::scope(|scope| {
            let release = scope.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(20));
                reader.unlock()
            });
            let again = republish();
            let released = release
                .join()
                .map_err(|_| "reader thread panicked".to_owned());
            released.and_then(|unlocked| unlocked.map_err(|why| why.to_string()))?;
            again
        });
        again.map_err(|why| format!("{namespace}: {why}"))?;
        reader.try_lock_shared().map_err(|why| why.to_string())?;
        let refusal = republish().err().unwrap_or_default();
        reader.unlock().map_err(|why| why.to_string())?;
        assert!(lost_owner_race(&refusal), "{namespace}: {refusal}");
        assert!(
            refusal.contains("one-second wait"),
            "{namespace}: {refusal}"
        );
        assert!(committed(&directory)?, "{namespace}");
        assert_eq!(read_exact(&directory.join("body.bin"), 64)?, body);
    }
    Ok(())
}

/// ledgers-2, D-1915: a receipt or body whose barrier failed is withdrawn, so
/// no later run reuses it as committed history; the rerun then commits.
#[test]
fn a_file_whose_barrier_failed_is_withdrawn_and_the_rerun_commits() -> Result<(), String> {
    for name in ["complete.bin", "body.bin"] {
        let root = scratch(&format!("withdrawn-{}", name.replace('.', "-")));
        let identity = [13_u8; 32];
        let body = b"barrier evidence".to_vec();
        let digest = brutex_core::blake3::hash(&body);
        let directory = publication(&root, identity);
        {
            let _armed =
                crate::fixed_tail::fault::Armed::arm(name, crate::fixed_tail::fault::Kind::Sync);
            let attempt =
                prepare_in_namespace(&root, "index-stop-vix-reference-v1", identity, &body)
                    .and_then(|pending| {
                        pending.verify_body(digest, body.len() as u64)?;
                        pending.finish(identity, digest, body.len() as u64)
                    });
            assert!(attempt.is_err(), "{name}");
        }
        assert!(!directory.join(name).exists(), "{name}");
        assert!(!committed(&directory)?, "{name}");
        publish(&root, identity, &body)?;
        assert!(committed(&directory)?, "{name}");
    }
    Ok(())
}
