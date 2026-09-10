//! Generated publication resources, never statistical or market authority.
use super::*;
use crate::boolean_qualified_journal::{Reader, Slot};
use brutex_core::blake3::hash;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    descriptor: [u8; 32],
    units: [[u8; 32]; 8],
}
impl Fixture {
    fn new() -> Result<Self, String> {
        let sequence = FIXTURE_SEQUENCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| "generated fixture sequence exhausted")?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|why| why.to_string())?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "brutex-qualified-publication-fixture-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&root)
            .map_err(|why| format!("generated fixture root {}: {why}", root.display()))?;
        Ok(Self {
            root,
            descriptor: hash(b"generated qualified publication scope"),
            units: std::array::from_fn(|index| {
                hash(format!("generated publication unit {index}").as_bytes())
            }),
        })
    }
    fn path(&self, index: usize) -> PathBuf {
        self.root.join(format!("generated-child-{index}.bin"))
    }
    fn publish_child(&self, index: usize) -> Result<Link, String> {
        let body = format!("generated committed child {index}");
        std::fs::write(self.path(index), body.as_bytes()).map_err(|why| why.to_string())?;
        let identity = hash(body.as_bytes());
        Ok(Link {
            identity,
            pin: hash(&identity),
        })
    }
    fn check_link(&self, index: usize, link: Link) -> Result<(), String> {
        let body = std::fs::read(self.path(index))
            .map_err(|why| format!("generated child {index} unavailable: {why}"))?;
        if hash(&body) != link.identity || hash(&link.identity) != link.pin {
            return Err(format!("generated child {index} changed"));
        }
        Ok(())
    }
    fn check(&self, index: usize, slot: &Slot) -> Result<(), String> {
        if let Some(link) = slot.complete {
            self.check_link(index, link)?;
        }
        Ok(())
    }
    fn writer(&self) -> Result<Writer, String> {
        Writer::open(
            &self.root,
            self.descriptor,
            self.units,
            64 * 1024,
            64,
            |index, _, link| self.check_link(index, link),
        )
    }
    fn first_seven(&self, journal: &mut Writer) -> Result<(), String> {
        for index in 0..7 {
            journal.begin(index)?;
            let link = self.publish_child(index)?;
            finish_checked(index, journal, link, |index, slot| self.check(index, slot))?;
        }
        assert_eq!(
            journal
                .slots()
                .iter()
                .filter(|slot| slot.complete.is_some())
                .count(),
            7
        );
        assert!(!journal.completed());
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn final_publication_rechecks_all_saved_children_and_reopens_the_same_eight_links()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.writer()?;
    fixture.first_seven(&mut journal)?;
    journal.begin(7)?;
    let link = fixture.publish_child(7)?;
    let mut checks = [0_u64; 8];
    finish_checked(7, &mut journal, link, |index, slot| {
        fixture.check(index, slot)?;
        if slot.complete.is_some() {
            let count = checks.get_mut(index).ok_or("test check slot absent")?;
            *count += 1;
        }
        Ok(())
    })?;
    assert!(journal.completed());
    assert!(checks.iter().all(|count| *count > 0));
    assert!(checks.iter().take(7).all(|count| *count >= 2));
    let slots = journal.slots().clone();
    let pin = journal.pin()?;
    drop(journal);
    let reopened = fixture.writer()?;
    assert!(reopened.completed());
    assert_eq!(reopened.slots(), &slots);
    assert_eq!(reopened.pin()?, pin);
    Ok(())
}

#[test]
fn deleted_earlier_child_blocks_final_publication_and_preserves_a_durable_refusal()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.writer()?;
    fixture.first_seven(&mut journal)?;
    journal.begin(7)?;
    let link = fixture.publish_child(7)?;
    std::fs::remove_file(fixture.path(0)).map_err(|why| why.to_string())?;
    let Err(why) = finish_checked(7, &mut journal, link, |index, slot| {
        fixture.check(index, slot)
    }) else {
        return Err("deleted earlier child permitted final publication".into());
    };
    assert!(why.contains("generated child 0 unavailable"), "{why}");
    let final_slot = journal.slots().last().ok_or("final slot absent")?;
    assert!(final_slot.started && final_slot.complete.is_none());
    assert!(!journal.completed());
    let reported = settle_refusal(7, "60min", &mut journal, &why);
    assert!(reported.starts_with(&why), "{reported}");
    assert!(reported.contains("refusal recorded"), "{reported}");
    let id = journal.identity();
    drop(journal);
    let saved = Reader::open(&fixture.root, id, 1024 * 1024)?;
    assert_eq!(
        saved
            .slots()
            .iter()
            .filter(|slot| slot.complete.is_some())
            .count(),
        7
    );
    let last = saved.slots().last().ok_or("saved final slot absent")?;
    assert!(last.complete.is_none());
    assert!(last.reason.contains("generated child 0 unavailable"));
    assert!(
        fixture.writer().is_err(),
        "reopen must recheck lost original child"
    );
    Ok(())
}

#[test]
fn failure_after_final_acknowledgment_is_explicit_and_never_rewrites_saved_history()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.writer()?;
    fixture.first_seven(&mut journal)?;
    journal.begin(7)?;
    let link = fixture.publish_child(7)?;
    let Err(why) = finish_checked(7, &mut journal, link, |index, slot| {
        if index == 7 && slot.complete.is_some() {
            std::fs::remove_file(fixture.path(index)).map_err(|why| why.to_string())?;
        }
        fixture.check(index, slot)
    }) else {
        return Err("changed child after acknowledgment was silently accepted".into());
    };
    assert!(why.contains("generated child 7 unavailable"), "{why}");
    assert!(journal.slots().iter().all(|slot| slot.complete.is_some()));
    let pin = journal.pin()?;
    let reported = settle_refusal(7, "60min", &mut journal, &why);
    assert!(reported.starts_with(&why), "{reported}");
    assert!(
        reported.contains("completion remains recorded"),
        "{reported}"
    );
    assert!(
        reported.contains("subsequent verification failed"),
        "{reported}"
    );
    assert!(!reported.contains("refusal recorded"), "{reported}");
    assert_eq!(journal.pin()?, pin, "acknowledged history is immutable");
    let id = journal.identity();
    drop(journal);
    let saved = Reader::open(&fixture.root, id, 1024 * 1024)?;
    assert_eq!(saved.pin(), pin);
    assert!(saved.slots().iter().all(|slot| slot.complete.is_some()));
    assert!(
        fixture.writer().is_err(),
        "retry cannot skip the lost child"
    );
    Ok(())
}

#[test]
fn failed_refusal_publication_preserves_original_reason_and_reopens_pending_state()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.writer()?;
    journal.begin(0)?;
    let id = journal.identity();
    let before = Reader::open(&fixture.root, id, 1024 * 1024)?;
    let slots = before.slots().clone();
    let pin = before.pin();
    let sequence = before
        .sequence()
        .checked_add(1)
        .ok_or("test sequence overflow")?;
    // A real exclusive publication directory collision fails the write after
    // its in-memory writer is poisoned. No fake production writer is injected.
    let collision = fixture
        .root
        .join("boolean-qualified-campaign-v1")
        .join(crate::identity_hex(&id))
        .join(format!("{sequence:016x}"));
    std::fs::create_dir(&collision)
        .map_err(|why| format!("publication collision path {}: {why}", collision.display()))?;
    let Err(expected_audit) = std::fs::create_dir(&collision) else {
        return Err("publication collision was not retained".into());
    };
    let original = "generated trade cap refused: need 5388, remaining 4778 · exact cause";
    let reported = settle_refusal(0, "1min", &mut journal, original);
    assert!(reported.starts_with(original), "{reported}");
    assert!(
        reported.contains("refusal publication also failed:"),
        "{reported}"
    );
    assert!(reported.contains(&expected_audit.to_string()), "{reported}");
    assert!(
        reported.contains("refusal persistence is unconfirmed"),
        "{reported}"
    );
    assert!(!reported.contains("refusal recorded"), "{reported}");
    assert_eq!(journal.slots(), &slots);
    assert!(
        journal.pin().is_err(),
        "uncertain writer must remain poisoned"
    );
    drop(journal);
    let reopened = fixture.writer()?;
    assert_eq!(reopened.slots(), &slots);
    assert_eq!(reopened.pin()?, pin);
    assert!(!reopened.completed());
    let saved = Reader::open(&fixture.root, id, 1024 * 1024)?;
    assert_eq!(saved.slots(), &slots);
    assert_eq!(saved.pin(), pin);
    assert_eq!(saved.history_records(), before.history_records());
    Ok(())
}
