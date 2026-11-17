//! Admission data for a future playback handoff, never a presentation snapshot.
//! An absent file index still requires metadata resolution and file selection.
use reqwest::Url;
use stremio_core::types::resource::{Stream, StreamSource};

#[derive(serde::Serialize)]
pub struct TorrentRequest {
    pub magnet: String,
    pub file_index: Option<u16>,
    pub file_must_include: Vec<String>,
}
fn valid_hash(hash: &str) -> bool {
    (hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        || (hash.len() == 32
            && hash
                .bytes()
                .all(|byte| matches!(byte.to_ascii_uppercase(), b'A'..=b'Z' | b'2'..=b'7')))
}
fn tracker(value: &str) -> Result<(), &'static str> {
    if value.len() > 2048 {
        return Err("invalid_tracker");
    }
    let url = Url::parse(value).map_err(|_| "invalid_tracker")?;
    if !matches!(url.scheme(), "http" | "https" | "udp")
        || url.host_str().is_none()
        || url.fragment().is_some()
    {
        return Err("invalid_tracker");
    }
    Ok(())
}
pub fn torrent(stream: &Stream) -> Result<TorrentRequest, &'static str> {
    let request = match &stream.source {
        StreamSource::Torrent {
            info_hash,
            file_idx,
            announce,
            file_must_include,
        } => {
            if announce.len() > 32
                || file_must_include.len() > 16
                || file_must_include.iter().any(|value| value.len() > 512)
            {
                return Err("source_limit");
            }
            let hash: String = info_hash.iter().map(|byte| format!("{byte:02x}")).collect();
            let mut url = Url::parse("magnet:?").map_err(|_| "invalid_magnet")?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("xt", &format!("urn:btih:{hash}"));
                for value in announce {
                    // DHT hints are not tracker endpoints. Discovery is an engine policy.
                    if value.starts_with("dht:") {
                        continue;
                    }
                    let value = value.strip_prefix("tracker:").unwrap_or(value);
                    tracker(value)?;
                    query.append_pair("tr", value);
                }
            }
            TorrentRequest {
                magnet: url.into(),
                file_index: *file_idx,
                file_must_include: file_must_include.clone(),
            }
        }
        StreamSource::Url { url } if url.scheme() == "magnet" => {
            if url.as_str().len() > 16384 {
                return Err("source_limit");
            }
            let mut hashes = 0;
            let mut trackers = 0;
            for (key, value) in url.query_pairs() {
                if key == "xt" && value.starts_with("urn:btih:") {
                    if !valid_hash(&value[9..]) {
                        return Err("invalid_info_hash");
                    }
                    hashes += 1;
                }
                if key == "tr" {
                    trackers += 1;
                    tracker(&value)?;
                }
            }
            if hashes != 1 {
                return Err("unsupported_info_hash");
            }
            if trackers > 32 {
                return Err("source_limit");
            }
            TorrentRequest {
                magnet: url.to_string(),
                file_index: None,
                file_must_include: vec![],
            }
        }
        _ => return Err("unsupported_source"),
    };
    if request.magnet.len() > 16384 {
        return Err("source_limit");
    }
    Ok(request)
}
