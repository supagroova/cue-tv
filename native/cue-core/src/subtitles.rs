//! Private playback handoff only: configured subtitle URLs never enter UI snapshots.
use serde::Serialize;
use stremio_core::types::resource::Subtitles;

#[derive(Serialize)]
pub struct Choice {
    pub id: String,
    pub language: String,
    pub label: Option<String>,
    pub url: String,
}

pub fn choices(tracks: &[Subtitles]) -> Vec<Choice> {
    tracks
        .iter()
        .filter(|track| {
            let url = &track.url;
            matches!(url.scheme(), "https" | "http")
                && url.host_str().is_some()
                && url.as_str().len() <= 2048
                && url.port() != Some(0)
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
                && !track.id.is_empty()
                && track.id.len() <= 512
                && !track.lang.is_empty()
                && track.lang.len() <= 64
                && !track.id.chars().any(char::is_control)
                && !track.lang.chars().any(char::is_control)
                && track
                    .label
                    .as_ref()
                    .is_none_or(|label| label.len() <= 128 && !label.chars().any(char::is_control))
                && track.fonts.is_empty()
        })
        .take(32)
        .map(|track| Choice {
            id: track.id.clone(),
            language: track.lang.clone(),
            label: track.label.clone(),
            url: track.url.to_string(),
        })
        .collect()
}

/// Plan private resource requests using Core's manifest compatibility rules.
/// No hash/size hints are sent until they are known from the selected media.
pub fn plan(
    addons: &[stremio_core::types::addon::Descriptor],
    kind: &str,
    video_id: &str,
) -> Result<Vec<stremio_core::types::addon::ResourceRequest>, &'static str> {
    use stremio_core::types::addon::{AggrRequest, ResourcePath};
    if kind.is_empty()
        || kind.len() > 64
        || kind.chars().any(char::is_control)
        || video_id.is_empty()
        || video_id.len() > 2048
        || video_id.chars().any(char::is_control)
    {
        return Err("invalid_subtitle_identity");
    }
    Ok(
        AggrRequest::AllOfResource(ResourcePath::without_extra("subtitles", kind, video_id))
            .plan(addons)
            .into_iter()
            .take(32)
            .map(|(_, request)| request)
            .collect(),
    )
}

#[derive(Serialize)]
pub struct Discovery {
    pub tracks: Vec<Choice>,
    pub failed_providers: usize,
}

/// Fetch independently of playback. Dropping this future drops all in-flight
/// transports; no detached tasks can publish results after caller retirement.
pub async fn discover(
    environment: std::sync::Arc<crate::environment::Environment>,
    requests: Vec<stremio_core::types::addon::ResourceRequest>,
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<Discovery, &'static str> {
    use crate::environment::NativeEnv;
    use futures::{Stream, StreamExt, stream};
    use std::time::Duration;
    use stremio_core::{runtime::Env, types::addon::ResourceResponse};
    environment.scope(async {
        let mut pending = stream::iter(requests.into_iter().take(32).map(|request| async move {
            let response = NativeEnv::addon_transport(&request.base).resource(&request.path);
            tokio::time::timeout(Duration::from_secs(3), response).await
        })).buffer_unordered(4);
        let deadline = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(deadline);
        let mut result = Discovery { tracks: vec![], failed_providers: 0 };
        let mut seen = std::collections::HashSet::new();
        loop {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Err("subtitle_discovery_cancelled"),
                _ = environment.cancelled() => return Err("subtitle_discovery_cancelled"),
                _ = &mut deadline => {
                    result.failed_providers += pending.size_hint().0;
                    break;
                }
                response = pending.next() => match response {
                    None => break,
                    Some(Ok(Ok(ResourceResponse::Subtitles { subtitles }))) => {
                        for choice in choices(&subtitles) {
                            if result.tracks.len() < 32 && seen.insert((choice.url.clone(), choice.language.clone())) {
                                result.tracks.push(choice);
                            }
                        }
                    }
                    Some(_) => result.failed_providers += 1,
                }
            }
        }
        Ok(result)
    }).await
}
