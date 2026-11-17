//! Source choices expose presentation only; configured URLs stay in Core.
use serde::Serialize;
use stremio_core::{
    models::{
        common::{Loadable, ResourceError},
        meta_details::{MetaDetails, Selected},
    },
    types::{addon::ResourcePath, resource::StreamSource},
};

pub fn selection(
    model: &MetaDetails,
    video_offset: usize,
    index: Option<usize>,
) -> Option<Selected> {
    let view = crate::details::view(model, video_offset)?;
    if view.status != "ready" {
        return None;
    }
    let id = match index {
        Some(index) => view.videos.get(index)?.id.clone(),
        None if view.kind == "movie" && view.video_total == 0 => view.id,
        _ => return None,
    };
    let mut selected = model.selected.clone()?;
    selected.stream_path = Some(ResourcePath::without_extra("stream", &view.kind, &id));
    selected.guess_stream = false;
    Some(selected)
}
#[derive(Clone, Debug, Serialize)]
pub struct Item {
    pub index: usize,
    pub kind: &'static str,
    pub name: Option<String>,
    pub description: Option<String>,
    pub size_bytes: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub status: &'static str,
    pub loading: bool,
    pub video_id: String,
    pub items: Vec<Item>,
    pub offset: usize,
    pub total: usize,
}
pub fn view(model: &MetaDetails, offset: usize) -> Option<View> {
    let selected = model.selected.as_ref()?.stream_path.as_ref()?;
    let pages = || model.meta_streams.iter().chain(&model.streams);
    let streams = || {
        pages()
            .filter_map(|page| page.content.as_ref()?.ready())
            .flatten()
    };
    let total = streams().count();
    let loading = pages().any(|page| matches!(page.content, None | Some(Loadable::Loading)));
    let successful = pages().any(|page| {
        matches!(
            page.content,
            Some(Loadable::Ready(_)) | Some(Loadable::Err(ResourceError::EmptyContent))
        )
    });
    Some(View {
        status: if total > 0 {
            "ready"
        } else if loading {
            "loading"
        } else if successful {
            "ready"
        } else {
            "failed"
        },
        loading,
        video_id: selected.id.clone(),
        offset,
        total,
        items: streams()
            .enumerate()
            .skip(offset)
            .take(24)
            .map(|(index, stream)| Item {
                index,
                kind: match &stream.source {
                    StreamSource::Torrent { .. } => "torrent",
                    StreamSource::Url { url } if url.scheme() == "magnet" => "torrent",
                    StreamSource::Url { url } if matches!(url.scheme(), "https" | "http") => "http",
                    _ => "unsupported",
                },
                name: stream
                    .name
                    .as_ref()
                    .map(|text| text.chars().take(128).collect()),
                description: stream
                    .description
                    .as_ref()
                    .map(|text| text.chars().take(512).collect()),
                size_bytes: stream.behavior_hints.video_size,
            })
            .collect(),
    })
}
