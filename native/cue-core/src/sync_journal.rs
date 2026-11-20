//! Pending revisions refer to full state in the same atomic library write.
//! This module plans mutations only; callers must persist library and journal together.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const STORAGE_KEY: &str = "cue_sync_intents_v1";

const MAX_TITLES: usize = 256;
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Identity,
    Capacity,
    TooLarge,
    Invalid,
    RevisionExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    kind: String,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    schema: u8,
    uid: String,
    last_revision: u64,
    entries: BTreeMap<String, Pending>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

impl Journal {
    pub(crate) fn new(uid: &str) -> Result<Self, Error> {
        if !valid_id(uid) {
            return Err(Error::Identity);
        }
        Ok(Self {
            schema: 1,
            uid: uid.into(),
            last_revision: 0,
            entries: BTreeMap::new(),
        })
    }
    fn check_owner(&self, uid: &str) -> Result<(), Error> {
        if self.uid != uid {
            return Err(Error::Identity);
        }
        Ok(())
    }
    pub(crate) fn with_intent(&self, uid: &str, kind: &str, id: &str) -> Result<Self, Error> {
        self.check_owner(uid)?;
        if !valid_id(id) || !matches!(kind, "movie" | "series") {
            return Err(Error::Invalid);
        }
        if let Some(previous) = self.entries.get(id) {
            if previous.kind != kind {
                return Err(Error::Identity);
            }
        } else if self.entries.len() == MAX_TITLES {
            return Err(Error::Capacity);
        }
        let mut next = self.clone();
        next.last_revision = self
            .last_revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        next.entries.insert(
            id.into(),
            Pending {
                kind: kind.into(),
                revision: next.last_revision,
            },
        );
        next.encode()?;
        Ok(next)
    }
    pub(crate) fn revision(&self, id: &str) -> Option<u64> {
        self.entries.get(id).map(|entry| entry.revision)
    }
    pub(crate) fn acknowledge(&self, uid: &str, id: &str, revision: u64) -> Result<Self, Error> {
        self.check_owner(uid)?;
        let mut next = self.clone();
        if next.revision(id) == Some(revision) {
            next.entries.remove(id);
        }
        Ok(next)
    }
    pub(crate) fn validate_library(
        &self,
        library: &stremio_core::types::library::LibraryBucket,
    ) -> Result<(), Error> {
        let uid = library
            .uid
            .as_ref()
            .map(|uid| uid.0.as_str())
            .ok_or(Error::Identity)?;
        Self::decode(uid, &self.encode()?)?;
        for (id, pending) in &self.entries {
            if !library
                .items
                .get(id)
                .is_some_and(|item| item.id == *id && item.r#type == pending.kind)
            {
                return Err(Error::Identity);
            }
        }
        Ok(())
    }
    pub(crate) fn reconcile(
        &self,
        local: &stremio_core::types::library::LibraryBucket,
        remote: Vec<stremio_core::types::library::LibraryItem>,
    ) -> Result<stremio_core::types::library::LibraryBucket, Error> {
        self.validate_library(local)?;
        let mut merged = local.clone();
        let mut seen = std::collections::BTreeSet::new();
        for item in remote {
            if !valid_id(&item.id)
                || !matches!(item.r#type.as_str(), "movie" | "series")
                || !seen.insert(item.id.clone())
            {
                return Err(Error::Invalid);
            }
            if local
                .items
                .get(&item.id)
                .is_some_and(|local| local.r#type != item.r#type)
            {
                return Err(Error::Identity);
            }
            if !self.entries.contains_key(&item.id) {
                // Timestamp ordering is not a remote acknowledgement. With no
                // pending local intent the server record is authoritative.
                merged.items.insert(item.id.clone(), item);
            }
        }
        Ok(merged)
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, Error> {
        let bytes = serde_json::to_vec(self).map_err(|_| Error::Invalid)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::TooLarge);
        }
        Ok(bytes)
    }
    pub(crate) fn decode(uid: &str, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::TooLarge);
        }
        let journal: Self = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        journal.check_owner(uid)?;
        if journal.schema != 1 || !valid_id(&journal.uid) || journal.entries.len() > MAX_TITLES {
            return Err(Error::Invalid);
        }
        for (id, entry) in &journal.entries {
            if !valid_id(id)
                || !matches!(entry.kind.as_str(), "movie" | "series")
                || entry.revision == 0
                || entry.revision > journal.last_revision
            {
                return Err(Error::Invalid);
            }
        }
        Ok(journal)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_local_state_wins_but_remote_state_wins_without_pending_intent() {
        use stremio_core::types::library::{LibraryBucket, LibraryItem};
        let item = |id: &str, position: u64, date: &str| -> LibraryItem {
            serde_json::from_value(serde_json::json!({"_id":id,"type":"movie","name":"Fixture",
                "removed":false,"temp":false,"_mtime":date,"state":{
                "timeWatched":0,"timeOffset":position,"overallTimeWatched":0,"timesWatched":0,
                "flaggedWatched":0,"duration":1000}}))
            .unwrap()
        };
        let local = LibraryBucket::new(
            Some("owner-a".parse().unwrap()),
            vec![
                item("pending", 200, "2026-01-02T00:00:00Z"),
                item("clean", 300, "2026-01-02T00:00:00Z"),
            ],
        );
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "pending")
            .unwrap();
        let remote = vec![
            item("pending", 900, "2026-01-03T00:00:00Z"),
            item("clean", 100, "2026-01-01T00:00:00Z"),
        ];
        let merged = journal.reconcile(&local, remote).unwrap();
        assert_eq!(merged.items["pending"].state.time_offset, 200);
        assert_eq!(merged.items["clean"].state.time_offset, 100);
        let equal_time = journal
            .reconcile(&local, vec![item("pending", 800, "2026-01-02T00:00:00Z")])
            .unwrap();
        assert_eq!(equal_time.items["pending"].state.time_offset, 200);
        let duplicate = item("clean", 900, "2026-01-03T00:00:00Z");
        assert!(matches!(
            journal.reconcile(&local, vec![duplicate.clone(), duplicate]),
            Err(Error::Invalid)
        ));
        let mut conflicting = item("pending", 900, "2026-01-03T00:00:00Z");
        conflicting.r#type = "series".into();
        assert!(matches!(
            journal.reconcile(&local, vec![conflicting]),
            Err(Error::Identity)
        ));
        let mut foreign = local.clone();
        foreign.uid = Some("owner-b".parse().unwrap());
        assert!(matches!(
            journal.reconcile(&foreign, vec![]),
            Err(Error::Identity)
        ));
        assert_eq!(
            journal.revision("pending"),
            Some(1),
            "Pulls never acknowledge pending writes"
        );
        assert_eq!(
            local.items["clean"].state.time_offset, 300,
            "Planning must leave canonical state unchanged"
        );
    }

    #[test]
    fn older_ack_does_not_clear_a_newer_edit() {
        let first = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "series", "title-1")
            .unwrap();
        let older = first.revision("title-1").unwrap();
        let latest = first.with_intent("owner-a", "series", "title-1").unwrap();
        let newer = latest.revision("title-1").unwrap();
        assert!(newer > older);
        let stale_ack = latest.acknowledge("owner-a", "title-1", older).unwrap();
        assert_eq!(stale_ack.revision("title-1"), Some(newer));
        assert!(
            stale_ack
                .acknowledge("owner-a", "title-1", newer)
                .unwrap()
                .revision("title-1")
                .is_none()
        );
    }

    #[test]
    fn capacity_rejects_new_titles_without_evicting_or_blocking_coalescing() {
        let mut journal = Journal::new("owner-a").unwrap();
        for n in 0..256 {
            journal = journal
                .with_intent("owner-a", "movie", &format!("title-{n}"))
                .unwrap();
        }
        let before = journal.encode().unwrap();
        assert_eq!(
            journal.with_intent("owner-a", "movie", "overflow"),
            Err(Error::Capacity)
        );
        assert_eq!(journal.encode().unwrap(), before);
        let changed = journal.with_intent("owner-a", "movie", "title-0").unwrap();
        assert_eq!(changed.entries.len(), 256);
        assert_eq!(changed.revision("title-0"), Some(257));
        assert_eq!(changed.revision("title-255"), Some(256));
    }

    #[test]
    fn reopen_preserves_revision_counter_even_after_every_ack() {
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "series", "title")
            .unwrap();
        let cleared = journal.acknowledge("owner-a", "title", 1).unwrap();
        let reopened = Journal::decode("owner-a", &cleared.encode().unwrap()).unwrap();
        assert_eq!(
            reopened
                .with_intent("owner-a", "series", "title")
                .unwrap()
                .revision("title"),
            Some(2)
        );
        assert_eq!(
            journal.revision("title"),
            Some(1),
            "Planning a write does not mutate the committed journal"
        );
    }

    #[test]
    fn identity_is_required_for_read_edit_and_acknowledgment() {
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "series", "title")
            .unwrap();
        assert_eq!(
            Journal::decode("owner-b", &journal.encode().unwrap()),
            Err(Error::Identity)
        );
        assert_eq!(
            journal.with_intent("owner-b", "series", "title"),
            Err(Error::Identity)
        );
        assert_eq!(
            journal.acknowledge("owner-b", "title", 1),
            Err(Error::Identity)
        );
        assert_eq!(
            journal.with_intent("owner-a", "movie", "title"),
            Err(Error::Identity)
        );
        assert_eq!(journal.revision("title"), Some(1));
    }

    #[test]
    fn malformed_or_oversized_storage_is_rejected_without_fallback() {
        assert_eq!(
            Journal::decode("owner-a", &vec![b' '; 1024 * 1024 + 1]),
            Err(Error::TooLarge)
        );
        for bytes in [
            r#"{"schema":2,"uid":"owner-a","last_revision":0,"entries":{}}"#,
            r#"{"schema":1,"uid":"owner-a","last_revision":1,"entries":{"title":{"kind":"movie","revision":2}}}"#,
            r#"{"schema":1,"uid":"owner-a","last_revision":1,"entries":{"title":{"kind":"movie","revision":0}}}"#,
            r#"{"schema":1,"uid":"owner-a","last_revision":0,"entries":{},"credentials":"must-not-be-stored"}"#,
        ] {
            assert_eq!(
                Journal::decode("owner-a", bytes.as_bytes()),
                Err(Error::Invalid)
            );
        }
    }

    #[test]
    fn revision_exhaustion_preserves_pending_intent() {
        let journal = Journal::decode("owner-a", br#"{"schema":1,"uid":"owner-a","last_revision":18446744073709551615,"entries":{"title":{"kind":"movie","revision":1}}}"#).unwrap();
        assert_eq!(
            journal.with_intent("owner-a", "movie", "title"),
            Err(Error::RevisionExhausted)
        );
        assert_eq!(journal.revision("title"), Some(1));
    }
}
