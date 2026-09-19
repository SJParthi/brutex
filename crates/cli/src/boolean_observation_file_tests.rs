#![cfg(test)]
//! Shared sealed-byte observation faults; fixtures grant no authoring capability.
#![allow(
    clippy::indexing_slicing,
    reason = "small fixed generated alias fixtures select known array positions"
)]
use super::Observation;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use std::fs;
use std::path::Path;

const NAMESPACES: [&str; 4] = [
    "boolean-candidates-v1",
    "boolean-statistics-v1",
    "boolean-admission-v1",
    "boolean-oos-v1",
];
const ID: [u8; 32] = [71; 32];
const BODY: &[u8] = b"explicit projection fixture";

fn open(root: &Path, namespace: &str) -> Result<(Observation, Vec<u8>), String> {
    Observation::open(root, namespace, ID, 112 + BODY.len() as u64, |body| {
        Ok(body.to_vec())
    })
}

#[test]
fn every_namespace_observer_waits_for_publication_and_idle_cache_allows_rerun() -> Result<(), String>
{
    let fixture = Fixture::new()?;
    for namespace in NAMESPACES {
        let pending = super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY)?;
        let pin = pending.finish(ID, brutex_core::blake3::hash(BODY), BODY.len() as u64)?;
        assert!(open(&fixture.output, namespace).is_err());
        drop(pending);
        let owner_path = fixture
            .output
            .join(namespace)
            .join(crate::identity_hex(&ID))
            .join("owner.lock");
        let (view, decoded) = Observation::open(
            &fixture.output,
            namespace,
            ID,
            112 + BODY.len() as u64,
            |body| {
                let owner = crate::readonly_file::open(&owner_path).map_err(super::display)?;
                assert!(
                    owner.try_lock().is_err(),
                    "the whole cold decoder must hold the publication barrier"
                );
                Ok(body.to_vec())
            },
        )?;
        assert_eq!(decoded, BODY);
        assert_eq!(view.identity(), ID);
        assert_eq!(view.body_bytes(), BODY.len() as u64);
        assert_eq!(view.completion_digest(), pin);
        view.with_current(|| {
            let owner = crate::readonly_file::open(&owner_path).map_err(super::display)?;
            assert!(
                owner.try_lock().is_err(),
                "the whole warm projection must hold the publication barrier"
            );
            Ok(())
        })?;
        let pending = super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY)?;
        assert_eq!(
            pending.finish(ID, brutex_core::blake3::hash(BODY), BODY.len() as u64)?,
            pin
        );
        assert!(view.require_current().is_err());
        drop(pending);
        view.require_current()?;
    }
    Ok(())
}

#[test]
fn observation_rejects_caps_unknown_namespaces_corruption_and_foreign_generation()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    for namespace in NAMESPACES {
        let pending = super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY)?;
        pending.finish(ID, brutex_core::blake3::hash(BODY), BODY.len() as u64)?;
        let directory = pending.directory().to_path_buf();
        drop(pending);
        for bound in [0, 111, 111 + BODY.len() as u64] {
            assert!(Observation::open(&fixture.output, namespace, ID, bound, |_| Ok(())).is_err());
        }
        let (view, _) = open(&fixture.output, namespace)?;
        let replacement = directory.join("replacement");
        fs::write(&replacement, BODY).map_err(super::display)?;
        fs::rename(&replacement, directory.join("body.bin")).map_err(super::display)?;
        let mut invoked = false;
        assert!(
            view.with_current(|| {
                invoked = true;
                Ok(())
            })
            .is_err()
        );
        assert!(!invoked);
        let (view, _) = open(&fixture.output, namespace)?;
        let mut corrupt = BODY.to_vec();
        *corrupt.first_mut().ok_or("fixture body empty")? ^= 1;
        fs::write(directory.join("body.bin"), corrupt).map_err(super::display)?;
        assert!(view.require_current().is_err());
        assert!(open(&fixture.output, namespace).is_err());
        fs::write(directory.join("body.bin"), BODY).map_err(super::display)?;
        let receipt_path = directory.join("complete.bin");
        let receipt = fs::read(&receipt_path).map_err(super::display)?;
        let mut corrupt = receipt.clone();
        *corrupt.last_mut().ok_or("fixture receipt empty")? ^= 1;
        fs::write(&receipt_path, corrupt).map_err(super::display)?;
        assert!(open(&fixture.output, namespace).is_err());
        fs::write(
            &receipt_path,
            receipt.get(..111).ok_or("fixture receipt short")?,
        )
        .map_err(super::display)?;
        assert!(open(&fixture.output, namespace).is_err());
        fs::remove_file(&receipt_path).map_err(super::display)?;
        assert!(open(&fixture.output, namespace).is_err());
    }
    assert!(open(&fixture.output, "../boolean-candidates-v1").is_err());
    assert!(open(&fixture.output, "unknown").is_err());
    Ok(())
}

#[test]
fn refused_decoder_and_projection_release_the_publication_lease() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let namespace = "boolean-statistics-v1";
    let pending = super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY)?;
    pending.finish(ID, brutex_core::blake3::hash(BODY), BODY.len() as u64)?;
    let owner_path = pending.directory().join("owner.lock");
    drop(pending);
    let refusal = Observation::open(
        &fixture.output,
        namespace,
        ID,
        112 + BODY.len() as u64,
        |_| Err::<(), _>("decoder refused".to_owned()),
    );
    assert!(matches!(refusal, Err(why) if why == "decoder refused"));
    let owner = crate::readonly_file::open(&owner_path).map_err(super::display)?;
    owner.try_lock().map_err(super::display)?;
    owner.unlock().map_err(super::display)?;
    let (view, _) = open(&fixture.output, namespace)?;
    assert_eq!(
        view.with_current(|| Err::<(), _>("projection refused".to_owned())),
        Err("projection refused".to_owned())
    );
    owner.try_lock().map_err(super::display)?;
    owner.unlock().map_err(super::display)?;
    view.require_current()?;
    Ok(())
}

fn publish(root: &Path, namespace: &str) -> Result<[u8; 32], String> {
    let pending = super::super::prepare_in_namespace(root, namespace, ID, BODY)?;
    pending.finish(ID, brutex_core::blake3::hash(BODY), BODY.len() as u64)
}

#[test]
fn compound_scope_deduplicates_aliases_without_unlocking_other_namespaces() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let names = [NAMESPACES[0], NAMESPACES[1]];
    let mut originals = Vec::new();
    let mut aliases = Vec::new();
    for namespace in names {
        publish(&fixture.output, namespace)?;
        originals.push(open(&fixture.output, namespace)?.0);
        aliases.push(open(&fixture.output, namespace)?.0);
    }
    // Same digest under different namespaces remains two distinct authorities;
    // repeated references and independently opened aliases each remain pinned.
    let observations = [
        &originals[0],
        &aliases[0],
        &originals[0],
        &originals[1],
        &aliases[1],
        &originals[1],
    ];
    let scratch = Observation::projection_scratch_bytes(observations.len())?;
    for fail in [false, true] {
        let mut invoked = false;
        let result = Observation::with_current_many(&observations, scratch, || {
            invoked = true;
            for namespace in names {
                assert!(
                    super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY)
                        .is_err()
                );
            }
            if fail {
                Err("complete comparison refused".to_owned())
            } else {
                Ok(73)
            }
        });
        assert!(invoked);
        assert_eq!(
            result,
            if fail {
                Err("complete comparison refused".to_owned())
            } else {
                Ok(73)
            }
        );
        for (namespace, original) in names.into_iter().zip(&originals) {
            assert_eq!(
                publish(&fixture.output, namespace)?,
                original.completion_digest()
            );
        }
    }
    let mut invoked = false;
    assert!(
        Observation::with_current_many(&observations, scratch - 1, || {
            invoked = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!invoked);
    assert!(Observation::projection_scratch_bytes(usize::MAX).is_err());
    Ok(())
}

#[test]
fn compound_scope_refuses_conflicting_aliases_and_rolls_back_partial_acquisition()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let names = [NAMESPACES[0], NAMESPACES[1]];
    for namespace in names {
        publish(&fixture.output, namespace)?;
    }
    let (first, _) = open(&fixture.output, names[0])?;
    let (second, _) = open(&fixture.output, names[1])?;
    let (mut alias, _) = open(&fixture.output, names[0])?;
    let scratch = Observation::projection_scratch_bytes(2)?;
    for field in [0, 1, 2] {
        let saved = (alias.identity, alias.completion, alias.bytes);
        match field {
            0 => alias.identity[0] ^= 1,
            1 => alias.completion[0] ^= 1,
            _ => alias.bytes += 1,
        }
        let mut invoked = false;
        let result = Observation::with_current_many(&[&first, &alias], scratch, || {
            invoked = true;
            Ok(())
        });
        assert!(result.is_err_and(|why| why.contains("conflicting receipt or generations")));
        assert!(!invoked);
        (alias.identity, alias.completion, alias.bytes) = saved;
    }
    // These names sort in this order, so the first lock has been acquired when
    // the ordinary second publisher forces acquisition to fail.
    assert!(names[0] < names[1]);
    let pending = super::super::prepare_in_namespace(&fixture.output, names[1], ID, BODY)?;
    let mut invoked = false;
    assert!(
        Observation::with_current_many(&[&first, &second], scratch, || {
            invoked = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!invoked);
    assert_eq!(
        publish(&fixture.output, names[0])?,
        first.completion_digest()
    );
    drop(pending);
    Observation::with_current_many(&[&first, &second], scratch, || Ok(()))?;
    // Exact bytes and receipt restored under a new file generation do not make
    // stale and fresh aliases interchangeable within a compound authority set.
    let path = fixture.output.join(names[0]).join(crate::identity_hex(&ID));
    fs::write(path.join("replacement"), BODY).map_err(super::display)?;
    fs::rename(path.join("replacement"), path.join("body.bin")).map_err(super::display)?;
    let (fresh, _) = open(&fixture.output, names[0])?;
    assert!(
        Observation::with_current_many(&[&first, &fresh], scratch, || Ok(()))
            .is_err_and(|why| why.contains("conflicting receipt or generations"))
    );
    let mut invoked = false;
    assert!(
        Observation::with_current_many(&[&first], scratch, || {
            invoked = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!invoked);
    assert_eq!(
        publish(&fixture.output, names[0])?,
        fresh.completion_digest()
    );
    Ok(())
}

#[test]
fn compound_projection_rejects_same_descriptor_reentry_without_releasing_outer_lock()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let namespace = NAMESPACES[0];
    publish(&fixture.output, namespace)?;
    let (observation, _) = open(&fixture.output, namespace)?;
    let scratch = Observation::projection_scratch_bytes(2)?;
    Observation::with_current_many(&[&observation, &observation], scratch, || {
        assert!(
            observation
                .require_current()
                .is_err_and(|why| why.contains("same descriptor"))
        );
        assert!(Observation::with_current_many(&[&observation], scratch, || Ok(())).is_err());
        assert!(super::super::prepare_in_namespace(&fixture.output, namespace, ID, BODY).is_err());
        Ok(())
    })?;
    assert_eq!(
        publish(&fixture.output, namespace)?,
        observation.completion_digest()
    );
    observation.require_current()?;
    Ok(())
}

#[test]
fn compound_scope_revalidates_after_callback_and_releases_every_owner() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let names = [NAMESPACES[0], NAMESPACES[1]];
    for namespace in names {
        publish(&fixture.output, namespace)?;
    }
    let (first, _) = open(&fixture.output, names[0])?;
    let (second, _) = open(&fixture.output, names[1])?;
    let scratch = Observation::projection_scratch_bytes(2)?;
    let path = fixture.output.join(names[1]).join(crate::identity_hex(&ID));
    let mut invoked = false;
    let result = Observation::with_current_many(&[&first, &second], scratch, || {
        invoked = true;
        // A restored identical file still changes its pinned generation; the
        // post-projection check must refuse it despite unchanged receipt bytes.
        fs::write(path.join("replacement"), BODY).map_err(super::display)?;
        fs::rename(path.join("replacement"), path.join("body.bin")).map_err(super::display)?;
        Ok(())
    });
    assert!(invoked);
    assert!(result.is_err());
    for namespace in names {
        publish(&fixture.output, namespace)?;
    }
    let (restored, _) = open(&fixture.output, names[1])?;
    Observation::with_current_many(&[&first, &restored], scratch, || Ok(()))?;
    Ok(())
}
