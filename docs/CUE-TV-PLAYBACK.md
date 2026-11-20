# Cue TV playback and service catalogues

Cue TV now has source implementations for its own episode/source chooser and internal players. Installation and hardware acceptance are tracked separately in `CUE-TV-DELIVERY.md`.

## Playback

Only configured Real-Debrid Torrentio stream providers are queried. The phone and TV browser receive display fields and opaque, expiring choices. The chosen episode and exact source remain bound to the requesting client and Stremio account. The Android host keeps the add-on configuration and private media URL.

Android uses Media3 ExoPlayer 1.11.1 inside a non-exported activity. The URL is delivered through a private in-memory handle, not an exported intent. Playback remains in front; leaving the player saves progress, releases the player/session and returns to browsing. A foreground media session supports the existing remote media keys. Codec support depends on the television's decoders.

Apple TV embeds the official VLCKit player. A private playback ticket is available only to a paired native Apple TV client. The normal source chooser can request its own selected media directly without relaunching the app. The phone relay presents the same internal player. Back dismisses that player over the existing browser. An AVAudioSession playback session respects the Apple TV's selected audio output. AirPods routing and format compatibility still require household hardware testing.

The current official player package is pinned to VideoLAN commit `3d0f0fa7031eab1a67c817c98a495e1c68fc998c`. Its manifest pins the binary checksum. The player is a VLCKit 4 preview dependency and must be tested on the household Apple TV before calling it accepted. Corresponding upstream sources and build instructions are at https://github.com/videolan/vlckit/tree/3d0f0fa7031eab1a67c817c98a495e1c68fc998c. Its LGPL 2.1 licence is bundled in `apple-tv/Resources/VLC-LGPL-2.1.txt`; preserve it and the upstream source/build reference when redistributing this app. Cue TV's own source and the upstream Couch TV MIT notice retain their existing licences.

## Progress

Cue progress is stored on the household host under the current Stremio account fingerprint. It contains title/episode metadata, position, duration and timestamps, never media URLs, add-on credentials or playback-session handles. Short failed starts do not create Continue Watching entries. Completed videos do not resume at their end. Existing Stremio history is read and merged without overwriting that external account.

Existing Stremio resume metadata uses the upstream `video_id` field. New Cue playback first uses its own saved progress for that exact episode, with existing Stremio offsets as a migration fallback. Switching accounts separates progress.

## Service rows

Catalogue requests use OmniCatalogs for Netflix, Prime Video and Apple TV+, in the existing setup country. The three categories request separate upstream `tnd`, `pop` and `new` catalogues. No category is filled with results from another sort. Individual failures preserve a previously cached matching shelf, marked stale internally; an empty list remains empty.

These are provider-labelled discovery charts, not a claim to private viewing figures from every service. OmniCatalogs uses JustWatch data and, for the first unfiltered Netflix Popular page, can substitute Netflix's own weekly Top 10. That distinction is described in the upstream source: https://github.com/careway/stremio-justwatch-addon/blob/beamup/src/domain/catalog.js. The catalogue integration never installs an add-on or changes the household's stream providers. Netflix catalogue inclusion does not itself guarantee that a playable source exists.

Useful upstream references: https://developer.android.com/media/media3/exoplayer/supported-formats and https://github.com/videolan/vlckit.
