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
