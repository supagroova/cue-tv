use cue_core::library;
use stremio_core::types::library::{LibraryBucket, LibraryItem};
fn item(index: usize, removed: bool, temp: bool) -> LibraryItem {
    serde_json::from_value(serde_json::json!({"_id":format!("owned-{index:02}"),"type":"movie","name":format!("Title {index:02}"),"removed":removed,"temp":temp,"_mtime":"2026-01-01T00:00:00Z","state":stremio_core::types::library::LibraryItemState::default()})).unwrap()
}
#[test]
fn bounded_library_pages_reach_every_member_and_keep_selection_identity() {
    let mut items: Vec<_> = (0..50).rev().map(|i| item(i, false, false)).collect();
    items.extend([item(50, true, false), item(51, false, true)]);
    let bucket = LibraryBucket::new(None, items);
    let first = library::view(&bucket, 0);
    assert_eq!(first.total, 50);
    assert_eq!(first.items.len(), 24);
    assert_eq!(first.items[0].id, "owned-00");
    let middle = library::view(&bucket, 24);
    assert_eq!(middle.items[0].id, "owned-24");
    assert_eq!(middle.items[23].id, "owned-47");
    let last = library::view(&bucket, 48);
    assert_eq!(
        last.items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["owned-48", "owned-49"]
    );
    let selected = library::selection(&bucket, 48, 1).unwrap();
    assert_eq!(selected.meta_path.id, "owned-49");
    assert_eq!(selected.meta_path.r#type, "movie");
    assert!(selected.stream_path.is_none());
    assert!(!selected.guess_stream);
    assert!(library::selection(&bucket, 48, 2).is_none());
    assert!(library::selection(&bucket, 0, 24).is_none());
}
#[test]
fn shrinking_library_clamps_window_and_never_exposes_removed_or_invalid_items() {
    let mut invalid = item(2, false, false);
    invalid.id = "x".repeat(513);
    let bucket = LibraryBucket::new(
        None,
        vec![item(0, false, false), item(1, true, false), invalid],
    );
    let page = library::view(&bucket, usize::MAX);
    assert_eq!(page.offset, 0);
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, "owned-00");
    assert_eq!(
        library::selection(&bucket, usize::MAX, 0)
            .unwrap()
            .meta_path
            .id,
        "owned-00"
    );
    let empty = library::view(&LibraryBucket::new(None, vec![]), 24);
    assert_eq!(empty.offset, 0);
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
}

#[tokio::test]
async fn session_pages_and_opens_the_selected_library_identity_with_revision_guards() {
    use cue_core::{
        environment::{Environment, NativeEnv},
        session::CoreSession,
    };
    use stremio_core::{constants::*, runtime::Env, types::profile::Profile};
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons.clear();
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
        NativeEnv::set_storage(
            LIBRARY_RECENT_STORAGE_KEY,
            Some(&LibraryBucket::new(
                None,
                (0..50).map(|i| item(i, false, false)).collect(),
            )),
        )
        .await
        .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env).await.unwrap();
    assert_eq!(session.snapshot().library.items[0].id, "owned-00");
    let old_revision = session.snapshot().revision;
    session.show_library(48, old_revision).await.unwrap();
    assert_eq!(session.snapshot().library.items.len(), 2);
    assert!(session.load_library_details(1, old_revision).await.is_err());
    assert!(session.snapshot().details.is_none());
    assert_eq!(
        session.show_library(50, session.snapshot().revision).await,
        Err("invalid_library_offset")
    );
    session
        .load_library_details(1, session.snapshot().revision)
        .await
        .unwrap();
    let state = session.settled().await.unwrap();
    assert_eq!(state.details.unwrap().id, "owned-49");
    assert_eq!(state.library.offset, 48);
    session.stop().await;
}
