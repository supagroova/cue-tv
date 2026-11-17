//! Redacted title details from Core's metadata model.
use serde::Serialize;
use stremio_core::models::{common::Loadable, meta_details::MetaDetails};
#[derive(Clone, Debug, Serialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub status: &'static str,
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub videos: Vec<Video>,
    pub video_total: usize,
    pub video_offset: usize,
}
pub fn view(model: &MetaDetails, offset: usize) -> Option<View> {
    let selected = model.selected.as_ref()?;
    let ready = model
        .meta_items
        .iter()
        .filter_map(|page| page.content.as_ref()?.ready())
        .find(|item| {
            item.preview.id == selected.meta_path.id
                && item.preview.r#type == selected.meta_path.r#type
        });
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
    Some(View {
        video_total: valid_videos().count(),
        video_offset: offset,
        videos: valid_videos()
            .skip(offset)
            .take(24)
            .map(|video| Video {
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
