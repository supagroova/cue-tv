//! Redacted title details from Core's metadata model.
use serde::Serialize;
use stremio_core::models::{common::Loadable, meta_details::MetaDetails};
#[derive(Clone, Debug, Serialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub watched: Option<bool>,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub status: &'static str,
    pub watched: Option<bool>,
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub videos: Vec<Video>,
    pub video_total: usize,
    pub video_offset: usize,
    pub seasons: Vec<u32>,
    pub selected_season: Option<u32>,
}
pub fn view(model: &MetaDetails, offset: usize) -> Option<View> {
    view_filtered(model, offset, None)
}
pub fn view_filtered(model: &MetaDetails, offset: usize, season: Option<u32>) -> Option<View> {
    let selected = model.selected.as_ref()?;
    let ready = model
        .meta_items
        .iter()
        .filter_map(|page| page.content.as_ref()?.ready())
        .find(|item| {
            item.preview.id == selected.meta_path.id
                && item.preview.r#type == selected.meta_path.r#type
        });
    // Only matching, ready metadata may expose the retained Core library state.
    let library_item = ready.and_then(|meta| {
        model
            .library_item
            .as_ref()
            .filter(|item| item.id == meta.preview.id && item.r#type == meta.preview.r#type)
    });
    let episode_watched = ready
        .zip(library_item)
        .map(|(meta, item)| item.state.watched_bitfield(&meta.videos));
    let loading = model
        .meta_items
        .iter()
        .any(|page| matches!(page.content, None | Some(Loadable::Loading)));
    let valid_videos = || {
        ready
            .into_iter()
            .flat_map(|item| &item.videos)
            .filter(|video| !video.id.is_empty() && video.id.len() <= 512)
    };
    let seasons = valid_videos()
        .filter_map(|video| video.series_info.as_ref().map(|info| info.season))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .take(128)
        .collect();
    let matching = || {
        valid_videos().filter(|video| {
            season.is_none_or(|season| {
                video
                    .series_info
                    .as_ref()
                    .is_some_and(|info| info.season == season)
            })
        })
    };
    Some(View {
        watched: library_item
            .filter(|item| item.r#type == "movie")
            .map(|item| item.watched()),
        seasons,
        selected_season: season,
        video_total: matching().count(),
        video_offset: offset,
        videos: matching()
            .skip(offset)
            .take(24)
            .map(|video| Video {
                watched: episode_watched
                    .as_ref()
                    .map(|watched| watched.get_video(&video.id)),
                id: video.id.clone(),
                title: video.title.chars().take(128).collect(),
                season: video.series_info.as_ref().map(|info| info.season),
                episode: video.series_info.as_ref().map(|info| info.episode),
            })
            .collect(),
        status: if ready.is_some() {
            "ready"
        } else if loading {
            "loading"
        } else {
            "failed"
        },
        id: selected.meta_path.id.clone(),
        kind: selected.meta_path.r#type.clone(),
        name: ready.map(|item| item.preview.name.chars().take(256).collect()),
        description: ready
            .and_then(|item| item.preview.description.as_ref())
            .map(|text| text.chars().take(4096).collect()),
    })
}

/// Only the selected, successfully loaded metadata can authorize library changes.
pub fn selected_preview(
    model: &MetaDetails,
) -> Option<&stremio_core::types::resource::MetaItemPreview> {
    let selected = model.selected.as_ref()?;
    model
        .meta_items
        .iter()
        .filter_map(|page| page.content.as_ref()?.ready())
        .map(|item| &item.preview)
        .find(|item| {
            item.id == selected.meta_path.id
                && item.r#type == selected.meta_path.r#type
                && !item.id.is_empty()
                && item.id.len() <= 512
                && item.r#type.len() <= 64
        })
}
