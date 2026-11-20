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
    selection_filtered(model, video_offset, index, None)
}
pub fn selection_filtered(
    model: &MetaDetails,
    video_offset: usize,
    index: Option<usize>,
    season: Option<u32>,
) -> Option<Selected> {
    let view = crate::details::view_filtered(model, video_offset, season)?;
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
    pub addon_index: Option<usize>,
    pub addon_name: Option<String>,
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
    view_with_addons(model, offset, &[])
}
pub fn view_with_addons(
    model: &MetaDetails,
    offset: usize,
    addons: &[stremio_core::types::addon::Descriptor],
) -> Option<View> {
    let selected = model.selected.as_ref()?.stream_path.as_ref()?;
    let pages = || model.meta_streams.iter().chain(&model.streams);
    let streams = || {
        pages()
            .filter_map(|page| Some((page, page.content.as_ref()?.ready()?)))
            .flat_map(|(page, streams)| streams.iter().map(move |stream| (page, stream)))
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
            .map(|(index, (page, stream))| {
                let provider = addons
                    .iter()
                    .enumerate()
                    .find(|(_, addon)| addon.transport_url == page.request.base);
                Item {
                    addon_index: provider.map(|(index, _)| index),
                    addon_name: provider.map(|(_, addon)| {
                        addon
                            .manifest
                            .name
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(128)
                            .collect()
                    }),
                    index,
                    kind: match &stream.source {
                        StreamSource::Torrent { .. } => "torrent",
                        StreamSource::Url { url } if url.scheme() == "magnet" => "torrent",
                        StreamSource::Url { .. }
                            if crate::playback_source::https(stream).is_ok() =>
                        {
                            "http"
                        }
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
                }
            })
            .collect(),
    })
}

pub(crate) fn prepare(
    model: &MetaDetails,
    index: usize,
) -> Result<crate::playback_source::PreparedTorrent, &'static str> {
    let stream = model
        .meta_streams
        .iter()
        .chain(&model.streams)
        .filter_map(|page| page.content.as_ref()?.ready())
        .flatten()
        .nth(index)
        .ok_or("invalid_source")?;
    let selected = model.selected.as_ref().ok_or("invalid_source")?;
    let video = selected.stream_path.as_ref().ok_or("invalid_source")?;
    Ok(crate::playback_source::PreparedTorrent {
        subtitles: crate::subtitles::choices(&stream.subtitles),
        meta_id: selected.meta_path.id.clone(),
        video_id: video.id.clone(),
        kind: selected.meta_path.r#type.clone(),
        torrent: crate::playback_source::torrent(stream)?,
    })
}

pub(crate) fn prepare_https(
    model: &MetaDetails,
    index: usize,
) -> Result<crate::playback_source::PreparedHttps, &'static str> {
    let stream = model
        .meta_streams
        .iter()
        .chain(&model.streams)
        .filter_map(|page| page.content.as_ref()?.ready())
        .flatten()
        .nth(index)
        .ok_or("invalid_source")?;
    let selected = model.selected.as_ref().ok_or("invalid_source")?;
    let video = selected.stream_path.as_ref().ok_or("invalid_source")?;
    Ok(crate::playback_source::PreparedHttps {
        subtitles: crate::subtitles::choices(&stream.subtitles),
        meta_id: selected.meta_path.id.clone(),
        video_id: video.id.clone(),
        kind: selected.meta_path.r#type.clone(),
        https: crate::playback_source::https(stream)?,
    })
}
