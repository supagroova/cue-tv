# Stremio episode and source selection on the phone

16 November 2026. The phone interface is hosted by Sitting Room TV. The Mac is only used to build and install updates.

## Household flow

Open **TV → Watch → Stremio** and choose a title. Shows open a season selector and episode list. Films go directly to sources. Choose an episode, filter the source list if useful, then tap the desired source. Browsing does not start or interrupt playback. The final choice follows the television's observed Apple TV or Android TV input.

At Rupert's request, the phone source list is restricted to **Torrentio with Real-Debrid**. Other add-on stream providers are not queried. Raw torrent/magnet sources and other debrid providers are excluded before playback handles are created. The restriction applies to our phone chooser; it does not uninstall or rewrite the household's Stremio add-on collection. Opening the title directly in Stremio remains an explicit fallback.

Both `[RD+]` and `[RD download]` results are allowed when they have Torrentio's HTTPS Real-Debrid resolver address. The latter may need time to become available. A cache label is not a guarantee that playback will start. An empty list offers another load and a reminder to check Torrentio's Real-Debrid connection in Stremio. No new Stremio or Real-Debrid key is requested or copied to a phone.

## Handoff and credentials

- The TV reads the linked account's installed add-on manifests through Stremio's account API. Metadata comes from Cinemeta, with installed metadata providers as a fallback. Literal pipe separators in Torrentio configuration paths are URI encoded without altering their values.
- Phones receive bounded display fields and random source handles. Private add-on configuration, media addresses and torrent payloads are retained on the TV. Handles belong to the paired client, expire after ten minutes and become invalid when the Stremio account changes.
- The player URI follows the official Stremio Core protocol: stream JSON, uncompressed zlib, Base64, URL components, stream manifest, metadata manifest, title type, title ID and exact episode ID. Playback remains in the installed Stremio app, including its player and account history behaviour.
- Android opens the URI in `com.stremio.one`. Apple TV opens the existing custom TV app, receives a short-lived opaque request and fetches the player URI over its paired connection. The ordinary phone token cannot read that private handoff route. The native TV app validates the URI scheme and player route before opening Stremio, then acknowledges the request.
- The Apple TV client label identifies the existing paired native app; it is not platform attestation. Paired household clients are trusted, and the service stays on the home network.
- Add-on requests require HTTPS and public resolved addresses, check redirects, and bound time, concurrency and response size. DNS is checked rather than pinned. Private URLs must never appear in errors, screenshots, logs or documentation.

## Verification

Android: 128 tests, zero failures or errors, one existing optional live YouTube check skipped. This includes independent-phone source ownership, expiry, exact player decoding, torrent payload preservation, partial provider failures, episode membership, and exclusion of other providers, other debrid services and raw torrents. Release build and lint passed.

Apple TV: five unit checks passed. The updated app is signed and installed in place using the existing identity and connection. Physical Apple TV playback of the exact chosen Real-Debrid source was visually confirmed. The relay acknowledged in 0.54 seconds, opened Magic Xylophone (1×1), resumed at 03:03 from the Android test, then displayed actual video. The earlier system profile chooser was resolved before the accepted test.

Android physical check: the exact Bluey S1 E1 source selected from the linked account played through Torrentio Real-Debrid. The player image was visually confirmed, then the TV was returned to custom Android Home. The first inspection coincided with an Android 14 accessibility-focus framework crash; a fresh handoff without the accessibility dump played successfully. The initial attempt is retained as a failure, not counted as playback acceptance. Media-session position reported zero even while video was visibly playing, so it is not used as progress evidence. The same episode was then verified playing on Apple TV with the selected source and a 03:03 resume position. Physical iPhone tapping, AirPods audio and long-standby behaviour remain household acceptance checks.

Phone browser: six flow checks passed, covering show/season/episode/source choice, quality filtering, failed-playback retry, specials, retained search, metadata/source errors, empty/partial results, films and 320/393/852 pixel layouts. Eight existing platform-routing checks also passed. Fixtures use synthetic data; their screenshots do not establish physical playback.

Current device evidence and final installation details are in `dist/apple-tv-research/stremio-phone-live-check.json`, `stremio-phone-install.json`, `stremio-phone-tvos-install.json` and `stremio-phone-release.json`. The earlier stopped install is retained as `stremio-phone-install-focus-transition.json`; it did not modify the installed package.

## Recovery

Fully close and reopen the iPhone TV app to load a newly installed TV-hosted interface. No phone cable or fresh pairing is needed for this feature. If sources expire, use **Load sources again**. If playback fails, the source chooser stays open for another attempt. The fallback opens the title in Stremio on the active platform.

The preceding Android APK is retained as `dist/couch-tv-sitting-room-youtube-home.apk`. Restore only through an in-place update signed with the existing keystore and the guarded installer, with Android custom home confirmed in front. Do not uninstall, clear data or replace pairing credentials. The installed Apple TV update also supports the earlier title and YouTube handoff routes.

Primary references: [Stremio Core deep links](https://github.com/Stremio/stremio-core/blob/development/src/deep_links/mod.rs), [Stremio stream response](https://stremio.github.io/stremio-addon-sdk/api/responses/stream.html), [Stremio metadata response](https://stremio.github.io/stremio-addon-sdk/api/responses/meta.html), [Torrentio's manifest](https://torrentio.strem.fun/manifest.json), [Torrentio's Real-Debrid implementation](https://github.com/TheBeastLT/torrentio-scraper/blob/master/addon/crates/addon/src/providers/realdebrid.rs).

## Source basis

The custom home and remote started from [Seagey/couch-tv](https://github.com/Seagey/couch-tv). They call Stremio account, catalogue and add-on APIs, then hand playback to the installed official Stremio app. The custom Android service and native Apple TV interface do not compile or embed the Rust `stremio-core` engine. Its [deep-link implementation](https://github.com/Stremio/stremio-core/blob/development/src/deep_links/mod.rs) was used as a protocol reference for the selected-source playback link. `dist/stremio-core-reference/` is a local reference checkout, not a build dependency. No migration to Stremio Core was requested or performed.
