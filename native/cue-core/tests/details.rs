use cue_core::details::view;
use stremio_core::models::{
    common::{Loadable, ResourceLoadable},
    meta_details::{MetaDetails, Selected},
};

fn series() -> MetaDetails {
    let selected: Selected = serde_json::from_value(serde_json::json!({"metaPath":{"resource":"meta","type":"series","id":"owned-series","extra":[]},"streamPath":null,"guessStream":false})).unwrap();
    let videos: Vec<_> = (1..=30).map(|n| serde_json::json!({"id":format!("owned-series:1:{n}"),"title":format!("Episode {n}"),"season":1,"episode":n})).collect();
    MetaDetails {
        selected: Some(selected),
        meta_items: vec![ResourceLoadable {
            request: serde_json::from_value(serde_json::json!({"base":"https://example.test/manifest.json","path":{"resource":"meta","type":"series","id":"owned-series","extra":[]}})).unwrap(),
            content: Some(Loadable::Ready(serde_json::from_value(serde_json::json!({"id":"owned-series","type":"series","name":"Owned series","videos":videos})).unwrap())),
        }],
        ..Default::default()
    }
}

#[test]
fn episode_windows_preserve_video_identity_and_reach_the_last_episode() {
    let model = series();
    let first = view(&model, 0).unwrap();
    assert_eq!(first.video_total, 30);
    assert_eq!(first.videos.len(), 24);
    assert_eq!(first.videos[0].id, "owned-series:1:1");
    assert_eq!(first.videos[0].season, Some(1));
    assert_eq!(first.videos[0].episode, Some(1));
    let last = view(&model, 24).unwrap();
    assert_eq!(last.video_offset, 24);
    assert_eq!(last.videos.len(), 6);
    assert_eq!(last.videos[5].id, "owned-series:1:30");
    assert_eq!(last.videos[5].title, "Episode 30");
}

#[test]
fn mismatched_metadata_never_supplies_episode_choices() {
    let mut model = series();
    model.selected.as_mut().unwrap().meta_path.id = "another-series".into();
    let state = view(&model, 0).unwrap();
    assert_eq!(state.status, "failed");
    assert_eq!(state.video_total, 0);
    assert!(state.videos.is_empty());
}

#[test]
fn selecting_an_episode_uses_the_full_video_id_from_the_current_window() {
    let model = series();
    let selected = cue_core::sources::selection(&model, 24, Some(5)).unwrap();
    assert_eq!(selected.stream_path.unwrap().id, "owned-series:1:30");
    assert!(cue_core::sources::selection(&model, 24, Some(6)).is_none());
    assert!(cue_core::sources::selection(&model, 0, None).is_none());
}

#[test]
fn an_empty_provider_does_not_hide_other_pending_source_results() {
    use stremio_core::{models::common::ResourceError, types::addon::ResourcePath};
    let mut model = series();
    model.selected.as_mut().unwrap().stream_path = Some(ResourcePath::without_extra(
        "stream",
        "series",
        "owned-series:1:1",
    ));
    let request = model.meta_items[0].request.clone();
    model.streams = vec![
        ResourceLoadable {
            request: request.clone(),
            content: Some(Loadable::Err(ResourceError::EmptyContent)),
        },
        ResourceLoadable {
            request,
            content: Some(Loadable::Loading),
        },
    ];
    let pending = cue_core::sources::view(&model, 0).unwrap();
    assert_eq!(pending.status, "loading");
    assert!(pending.loading);
    assert_eq!(pending.total, 0);
    model.streams[1].content = Some(Loadable::Err(ResourceError::EmptyContent));
    let empty = cue_core::sources::view(&model, 0).unwrap();
    assert_eq!(empty.status, "ready");
    assert!(!empty.loading);
    assert!(empty.items.is_empty());
}
