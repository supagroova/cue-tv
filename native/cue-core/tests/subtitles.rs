use cue_core::subtitles::choices;
use stremio_core::types::resource::Stream;

#[test]
fn subtitles_keep_signed_urls_private_and_bound_unsupported_tracks() {
    let stream: Stream = serde_json::from_value(serde_json::json!({
        "url":"https://video.example/movie.mp4", "subtitles":[
            {"id":"english", "lang":"eng", "url":"https://captions.example/a.vtt?token=owned"},
            {"id":"local", "lang":"eng", "url":"file:///private/a.vtt"},
            {"id":"credentials", "lang":"eng", "url":"https://user:password@captions.example/a.vtt"},
            {"id":"fragment", "lang":"eng", "url":"https://captions.example/a.vtt#secret"},
            {"id":"font-dependent", "lang":"eng", "url":"https://captions.example/a.ass", "fonts":["https://captions.example/font.ttf"]}
        ]
    })).unwrap();
    let json = serde_json::to_value(choices(&stream.subtitles)).unwrap();
    assert_eq!(
        json,
        serde_json::json!([{ "id":"english", "language":"eng", "label":null, "url":"https://captions.example/a.vtt?token=owned" }])
    );
    let mut tracks = stream.subtitles.clone();
    tracks = std::iter::repeat_n(tracks.remove(0), 100).collect();
    assert_eq!(choices(&tracks).len(), 32);
}
