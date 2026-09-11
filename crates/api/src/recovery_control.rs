//! Durable, per-plan STOP intent, separate from the locked recovery work journal.
//!
//! `audit/recovery-v1/<plan-id>.stop.bin` uses the existing 1024-byte versioned
//! journal. Its sole key is the plan ID; its immutable body is
//! `recovery-stop-v1:<plan-id>`. `Blocked` means STOP and `Queued` means an
//! explicitly requested clear. All quantity fields remain zero. Repeated writes
//! of the same state are idempotent; clearing never deletes stop history.
//!
//! The active-ID mutex serializes stop/clear with activation and idle changes.
//! A successful stop has synced both the record and its directory ancestry.
//! Opening replays O(control events), with O(1) distinct-key state; device sync
//! latency and total history replay are not claimed to be constant-time.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::recovery_journal::{Journal, Record, Status};
use crate::server::Site;

/// Publish the recovery ID under `site.run` while claiming its progress slot,
/// before seeding can yield. This is STOP routing, not permission to pull.
/// This does not clear an earlier STOP; only an explicit start may do that.
pub(crate) fn activate(site: &Site, id: [u8; 32]) {
    *site
        .recovery_active
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(id);
}

/// Clear the owner under `site.run` together with publishing finished progress,
/// without changing durable STOP history. Lock order is always run then active.
pub(crate) fn idle(site: &Site) {
    *site
        .recovery_active
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Append a clear only for an explicitly requested reactivation, never at boot.
/// Refuses an unavailable store, a different active plan, or uncertain I/O.
pub(crate) fn clear_stop(site: &Site, id: [u8; 32]) -> Result<(), String> {
    let active = site
        .recovery_active
        .lock()
        .map_err(|_| "recovery active-plan lock is poisoned".to_owned())?;
    if active.is_some_and(|current| current != id) {
        return Err("cannot clear STOP for a different active recovery plan".to_owned());
    }
    persist(site, id, Status::Queued)
}

/// Check persisted intent before resuming. Missing STOP history means false;
/// missing stores, empty existing journals, corruption and busy locks refuse.
pub(crate) fn is_stopped(site: &Site, id: [u8; 32]) -> Result<bool, String> {
    let _active = site
        .recovery_active
        .lock()
        .map_err(|_| "recovery active-plan lock is poisoned".to_owned())?;
    require_store(site)?;
    let path = path(site, id);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err("recovery STOP path is not a regular file".to_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    }
    // Reopening also synchronizes a complete record left by an uncertain sync.
    // An empty/ragged journal never becomes permission to resume.
    let journal = Journal::open_existing(&path).map_err(|error| error.to_string())?;
    state(&journal, id)?
        .map(|status| status == Status::Blocked)
        .ok_or_else(|| "existing recovery STOP journal is empty; intent is unverified".to_owned())
}

/// Durably stop the active recovery. A normal pull has no active recovery ID,
/// so this is a no-op with no filesystem access, including on a missing store.
pub(crate) fn stop(site: &Site) -> Result<(), String> {
    let active = site
        .recovery_active
        .lock()
        .map_err(|_| "recovery active-plan lock is poisoned".to_owned())?;
    match *active {
        Some(id) => persist(site, id, Status::Blocked),
        None => Ok(()),
    }
}

fn hex(id: [u8; 32]) -> String {
    use core::fmt::Write as _;
    id.iter().fold(String::with_capacity(64), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn path(site: &Site, id: [u8; 32]) -> PathBuf {
    site.store_root
        .join("audit/recovery-v1")
        .join(format!("{}.stop.bin", hex(id)))
}

fn require_store(site: &Site) -> Result<(), String> {
    let metadata = fs::metadata(&site.store_root).map_err(|error| {
        format!("configured source store is unavailable; no replacement created: {error}")
    })?;
    if !metadata.is_dir() {
        return Err(
            "configured source store is not a directory; no replacement created".to_owned(),
        );
    }
    Ok(())
}

fn record(id: [u8; 32], status: Status) -> Record {
    Record {
        key: id,
        body: format!("recovery-stop-v1:{}", hex(id)),
        status,
        attempts: 0,
        unchanged: 0,
        committed: 0,
        diagnostics: 0,
        missing: 0,
        unverified: 0,
        http_status: 0,
    }
}

fn state(journal: &Journal, id: [u8; 32]) -> Result<Option<Status>, String> {
    if journal.latest.is_empty() {
        return Ok(None);
    }
    if journal.latest.len() != 1 {
        return Err("recovery STOP journal contains foreign plan keys".to_owned());
    }
    let row = journal
        .latest
        .get(&id)
        .ok_or("recovery STOP journal plan identity mismatch")?;
    if !matches!(row.status, Status::Queued | Status::Blocked) || *row != record(id, row.status) {
        return Err("recovery STOP journal has invalid control state or identity".to_owned());
    }
    Ok(Some(row.status))
}

fn persist(site: &Site, id: [u8; 32], status: Status) -> Result<(), String> {
    require_store(site)?;
    // Create each child non-recursively. If the mounted/configured store
    // disappears after the check, this must fail rather than recreate it.
    create_child(&site.store_root.join("audit"))?;
    let directory = site.store_root.join("audit/recovery-v1");
    create_child(&directory)?;
    let mut journal = Journal::open(&path(site, id)).map_err(|error| error.to_string())?;
    if state(&journal, id)? != Some(status) {
        journal
            .append(record(id, status))
            .map_err(|error| error.to_string())?;
    }
    // Journal::open synced its own directory entry. These parents may have
    // been created above; a durable file in a lost directory is not a STOP.
    fs::File::open(site.store_root.join("audit"))
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    fs::File::open(&site.store_root)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())
}

fn create_child(path: &std::path::Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if fs::metadata(path)
                .map_err(|error| error.to_string())?
                .is_dir()
            {
                Ok(())
            } else {
                Err(format!("{} is not a directory", path.display()))
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "assertions over private, temporary fixtures only"
)]
mod tests {
    use super::*;
    use crate::pullrun::Progress;
    use crate::server::Loaded;
    use axum::extract::State;
    use axum::http::StatusCode;

    fn fixture(name: &str) -> (PathBuf, Site) {
        let root = crate::scratch::path(name);
        fs::create_dir_all(&root).unwrap();
        let site = Site::load(&root.join("missing-masters"), &root);
        (root, site)
    }

    #[test]
    fn stop_and_explicit_clear_survive_new_site_and_preserve_history() {
        let (root, site) = fixture("recovery-control-persistence");
        let id = [0xabu8; 32];
        assert!(!is_stopped(&site, id).unwrap());
        assert!(!path(&site, id).exists());
        clear_stop(&site, id).unwrap();
        activate(&site, id);
        stop(&site).unwrap();
        stop(&site).unwrap();
        let history = path(&site, id);
        assert_eq!(fs::metadata(&history).unwrap().len(), 2 * 1024);
        assert_eq!(
            history.file_name().unwrap().to_str().unwrap(),
            format!("{}.stop.bin", "ab".repeat(32))
        );
        idle(&site);
        let restarted = Site::load(&root.join("missing-masters"), &root);
        assert!(is_stopped(&restarted, id).unwrap());
        assert!(!is_stopped(&restarted, [0xacu8; 32]).unwrap());
        clear_stop(&restarted, id).unwrap();
        assert!(!is_stopped(&restarted, id).unwrap());
        let events = crate::recovery_journal::tail(&history, 3).unwrap();
        assert_eq!(
            events,
            vec![
                record(id, Status::Queued),
                record(id, Status::Blocked),
                record(id, Status::Queued)
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_active_recovery_is_no_write_even_with_a_missing_store() {
        let root = crate::scratch::path("recovery-control-no-active");
        assert!(!root.exists());
        let site = Site::load(&root.join("missing-masters"), &root);
        stop(&site).unwrap();
        activate(&site, [1; 32]);
        idle(&site);
        stop(&site).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn missing_store_refuses_without_creating_a_replacement() {
        let root = crate::scratch::path("recovery-control-missing-store");
        assert!(!root.exists());
        let site = Site::load(&root.join("missing-masters"), &root);
        let id = [2; 32];
        activate(&site, id);
        assert!(stop(&site).is_err());
        assert!(clear_stop(&site, id).is_err());
        assert!(is_stopped(&site, id).is_err());
        assert!(!root.exists());
    }

    #[test]
    fn stop_is_independent_of_the_locked_plan_and_other_plan_clear_refuses() {
        let (root, site) = fixture("recovery-control-independent");
        let id = [3; 32];
        clear_stop(&site, id).unwrap();
        let plan_path = root
            .join("audit/recovery-v1")
            .join(format!("{}.bin", hex(id)));
        let plan = Journal::open(&plan_path).unwrap();
        activate(&site, id);
        stop(&site).unwrap();
        assert!(is_stopped(&site, id).unwrap());
        assert!(clear_stop(&site, [4; 32]).is_err());
        assert!(!path(&site, [4; 32]).exists());
        let held = Journal::open(&path(&site, id)).unwrap();
        assert!(stop(&site).is_err());
        assert!(clear_stop(&site, id).is_err());
        assert!(is_stopped(&site, id).is_err());
        drop(held);
        drop(plan);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_or_empty_stop_history_refuses_and_is_never_truncated() {
        use std::io::Write as _;
        let (root, site) = fixture("recovery-control-corrupt");
        let id = [5; 32];
        clear_stop(&site, id).unwrap();
        activate(&site, id);
        stop(&site).unwrap();
        let history = path(&site, id);
        fs::OpenOptions::new()
            .append(true)
            .open(&history)
            .unwrap()
            .write_all(&[1])
            .unwrap();
        let before = fs::read(&history).unwrap();
        assert!(is_stopped(&site, id).is_err());
        assert!(stop(&site).is_err());
        assert!(clear_stop(&site, id).is_err());
        assert_eq!(fs::read(&history).unwrap(), before);
        let empty_id = [6; 32];
        fs::File::create(path(&site, empty_id)).unwrap();
        assert!(is_stopped(&site, empty_id).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn valid_crc_does_not_let_foreign_identity_or_status_clear_a_stop() {
        let (root, site) = fixture("recovery-control-invalid-state");
        let id = [7; 32];
        clear_stop(&site, id).unwrap();
        let history = path(&site, id);
        let mut journal = Journal::open(&history).unwrap();
        journal.append(record(id, Status::Verified)).unwrap();
        drop(journal);
        assert!(is_stopped(&site, id).is_err());
        assert!(clear_stop(&site, id).is_err());
        let other = [8; 32];
        let mut journal = Journal::open(&path(&site, other)).unwrap();
        journal.append(record(id, Status::Blocked)).unwrap();
        drop(journal);
        assert!(is_stopped(&site, other).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn handler_acknowledges_only_durable_recovery_stop() {
        let (root, site) = fixture("recovery-control-handler-success");
        let id = [9; 32];
        let site = Loaded::new(site);
        *site.run.lock().unwrap() = Some(Progress::claimed());
        activate(&site, id);
        let (status, _, body) = crate::server::pull_run_stop(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "{\"stopping\":true}");
        assert!(site.run.lock().unwrap().as_ref().unwrap().stopping);
        let restarted = Site::load(&root.join("missing-masters"), &root);
        assert!(is_stopped(&restarted, id).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn handler_io_refusal_returns_503_and_retains_memory_stop() {
        let (root, site) = fixture("recovery-control-handler-failure");
        fs::write(root.join("audit"), b"not a directory").unwrap();
        let site = Loaded::new(site);
        *site.run.lock().unwrap() = Some(Progress::claimed());
        activate(&site, [10; 32]);
        let (status, _, body) = crate::server::pull_run_stop(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["stop_persisted"], false);
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("not durably recorded")
        );
        assert!(site.run.lock().unwrap().as_ref().unwrap().stopping);
        assert_eq!(fs::read(root.join("audit")).unwrap(), b"not a directory");
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn normal_pull_stop_and_idle_handler_remain_filesystem_free() {
        let root = crate::scratch::path("recovery-control-handler-normal");
        assert!(!root.exists());
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        let (status, _, body) = crate::server::pull_run_stop(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "{\"stopping\":false}");
        *site.run.lock().unwrap() = Some(Progress::claimed());
        let (status, _, body) = crate::server::pull_run_stop(State(Loaded::clone(&site))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "{\"stopping\":true}");
        assert!(!root.exists());
    }
}
