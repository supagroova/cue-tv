use cue_core::persistence::{EncryptedStore, StoreError};
use std::{collections::BTreeMap, path::PathBuf};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cue-core-store-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn encrypted_store_roundtrip_randomizes_ciphertext_and_detects_tampering() {
    let directory = Directory::new();
    let (store, initial) = EncryptedStore::open(&directory.0, &[7; 32]).unwrap();
    assert!(initial.is_empty());
    let values = BTreeMap::from([(
        "profile".into(),
        br#"{"url":"https://example.test/private-token/manifest.json"}"#.to_vec(),
    )]);
    store.save(&values).unwrap();
    let first = std::fs::read(directory.0.join("state.bin")).unwrap();
    assert!(!String::from_utf8_lossy(&first).contains("private-token"));
    drop(store);
    let (store, restored) = EncryptedStore::open(&directory.0, &[7; 32]).unwrap();
    assert_eq!(restored, values);
    store.save(&values).unwrap();
    drop(store);
    let mut second = std::fs::read(directory.0.join("state.bin")).unwrap();
    assert_ne!(first, second);
    assert!(matches!(
        EncryptedStore::open(&directory.0, &[8; 32]),
        Err(StoreError::Corrupt)
    ));
    let last = second.len() - 1;
    second[last] ^= 1;
    std::fs::write(directory.0.join("state.bin"), &second).unwrap();
    assert!(matches!(
        EncryptedStore::open(&directory.0, &[7; 32]),
        Err(StoreError::Corrupt)
    ));
    assert_eq!(
        std::fs::read(directory.0.join("state.bin")).unwrap(),
        second
    );
}
#[test]
fn failed_write_preserves_the_previous_encrypted_value() {
    let directory = Directory::new();
    let (store, _) = EncryptedStore::open(&directory.0, &[9; 32]).unwrap();
    let original = BTreeMap::from([("value".into(), b"42".to_vec())]);
    store.save(&original).unwrap();
    let oversized = BTreeMap::from([(
        "value".into(),
        serde_json::to_vec(&"x".repeat(17 * 1024 * 1024)).unwrap(),
    )]);
    assert!(matches!(store.save(&oversized), Err(StoreError::TooLarge)));
    drop(store);
    assert_eq!(
        EncryptedStore::open(&directory.0, &[9; 32]).unwrap().1,
        original
    );
}

#[tokio::test]
async fn real_core_guest_settings_survive_an_encrypted_relaunch() {
    use cue_core::{environment::Environment, session::CoreSession};
    let directory = Directory::new();
    let mut first = CoreSession::guest(Environment::persistent(&directory.0, &[3; 32]).unwrap())
        .await
        .unwrap();
    first.set_interface_language("fra").await.unwrap();
    assert!(first.settled().await.unwrap().error.is_none());
    first.stop().await;
    drop(first);
    let mut second = CoreSession::guest(Environment::persistent(&directory.0, &[3; 32]).unwrap())
        .await
        .unwrap();
    assert_eq!(second.snapshot().interface_language, "fra");
    second.stop().await;
}

#[tokio::test]
async fn unavailable_directory_does_not_commit_in_memory() {
    use cue_core::environment::{Environment, NativeEnv};
    use stremio_core::runtime::Env;
    let directory = Directory::new();
    let env = Environment::persistent(&directory.0, &[4; 32]).unwrap();
    env.scope(async {
        NativeEnv::set_storage("value", Some(&1)).await.unwrap();
    })
    .await;
    let moved = directory.0.with_extension("moved");
    std::fs::rename(&directory.0, &moved).unwrap();
    let write = env
        .scope(async { NativeEnv::set_storage("value", Some(&2)).await })
        .await;
    let value = env
        .scope(async { NativeEnv::get_storage::<u8>("value").await })
        .await;
    std::fs::rename(&moved, &directory.0).unwrap();
    assert!(write.is_err());
    assert_eq!(value.unwrap(), Some(1));
    env.shutdown().await;
    drop(env);
    assert_eq!(
        EncryptedStore::open(&directory.0, &[4; 32]).unwrap().1["value"],
        b"1"
    );
}

#[test]
fn a_storage_directory_has_only_one_live_owner() {
    let directory = Directory::new();
    let (first, _) = EncryptedStore::open(&directory.0, &[1; 32]).unwrap();
    assert!(matches!(
        EncryptedStore::open(&directory.0.join("."), &[1; 32]),
        Err(StoreError::Unavailable)
    ));
    drop(first);
    assert!(EncryptedStore::open(&directory.0, &[1; 32]).is_ok());
}
