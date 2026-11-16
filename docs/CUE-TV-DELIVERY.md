# Cue TV delivery and next work

16 November 2026.

## Installed remote repair

- iPhone 1.0 (5), display name Cue TV. Existing bundle identity and shared Keychain group retained. Strict signing verification passed and the paired phone reports the new name and build.
- Android host update built, signed and installed in place. The centre pad suppresses text selection and the callout, sends one held Select after a deliberate hold, cancels it for swipes and cancellation, and exposes More options to accessibility.
- Apple TV has its own TV button when that input is active. Tap sends the physical TV button; hold sends a full one-second press for Control Centre. Cue TV Home remains its existing separate shortcut. The TV button is refused when Apple TV is not selected.
- tvOS 1.0 (4), display name Cue TV. The sidebar uses Cue TV too. Bundle identity and saved connection retained.
- The real Apple TV Control Centre was opened through the installed host's held TV button and visually confirmed, then dismissed with Back.
- Five Companion interoperability tests, eight browser gesture checks, 21 iPhone unit tests and five iPhone UI tests passed. The native WebKit test holds Select and TV without producing Copy or an extra tap. Release build and Android lint passed.

Physical phone touch feel and third-party apps' individual context menus still need household use. The TV's response to the held TV message was tested on hardware; the iPhone gesture was tested in its simulator using the actual WebKit implementation.

## YouTube build 5

The owner confirmed real signed-in recommendations in build 4. Build 5 compacts standard Home video rows, removes the website's duplicate Home header/footer and pauses preview media. URL observations and navigation no longer hand off automatically: a separate Play on TV action is required, with cancellation and retry. Account windows keep their normal WebKit configuration and opener. No cookies or feed data are exported.

The explicit-action, popup sign-in, per-feed management and inline visibility UI tests passed. Build 5 is installed. Actual personalised layout and preview behaviour need a fresh phone check after this update. The second household phone has not received this native update.

## Collaboration

Private repository: https://github.com/rclaycock/cue-tv. Initial source commit 2b778b6f32e3a85b8704cf20cac1e589297ca7d6. supagroova was invited and last observed awaiting acceptance. The collaboration checkout is a sanitised export; keep household setup, signing, tokens, account data, captures and hosting metadata out. Run privacy checks on both the working tree and Git history before every push. The original upstream checkout and remote are preserved.

## Current requested work

1. Internal Stremio playback, with episode and source selection inside Cue TV, Back returning to the existing browse position, viewing progress and history. YouTube continues in its own app. Real-Debrid Torrentio is the only stream provider.
2. Service catalogue shelves for Netflix, Prime Video and Apple TV+, distinguishing Trending, Popular and New. Respect the existing setup country. Catalogue-only integration must not install stream providers or modify the Stremio account.
3. Three real deployment modes: Android TV only, Apple TV only and Both. Existing hybrid pairings must migrate unchanged. Apple TV only needs independent discovery, pairing, remote control and Stremio access on the phone and Apple TV, with no Android host or Mac required. A picker alone does not satisfy this.

The installed hybrid still depends on the Android host. Standalone Apple mode is unfinished.

## Player and catalogue delivery

- The Android internal player and service catalogue update was installed in place. A selected cached Real-Debrid source played inside Cue TV on the actual Android TV, with a visible video frame and captions. The first resumed-source check left the saved position unchanged. The final fresh-episode progress check is recorded below. The first sample at 00:00 was still buffering and is not the acceptance image.
- Android release, lint and 135 unit checks completed with zero failures and one optional public-network skip. The catalogue and history tests cover independent rankings, stale matching shelves, account separation, completion and exact-episode resume.
- tvOS build 4 was signed, installed and reported by the actual Apple TV. Seven native unit checks passed. A native UI test played an owned generated Matroska sample, recorded positive player position/duration, paused and returned to the same source chooser. The fixture supports byte-range requests; earlier callback-thread and fixture failures are retained as diagnostics.
- The latest physical Apple TV phone-to-player handoff timed out before receipt could be confirmed. Direct Enter was sent as requested for user selection, then Cue Home was requested. A fresh attempt still returned a handoff timeout. Apple's developer screen capture and app launch also timed out, despite device discovery reporting a paired, connected Apple TV. The physical Apple player, AirPods output and a successful phone handoff remain acceptance gates. Do not describe the installation alone as proof of playback.
- Android's PlayerView initially consumed Back to hide controls. The final patch handles Back before PlayerView, saves progress, stops the player and returns to the existing browser. It also exposes its media session to the phone's Now display while retaining Cue's exact episode history.
- The new Apple system TV action has its own `apple_tv` command. Android's pre-existing raw TV key is preserved. The new Apple action is refused when Apple TV is no longer the active input.
- The final native WebKit hold test passed after that command change. Eight browser gesture checks passed again. Browser service/episode/source checks passed with no page errors. A legacy gesture fixture does not expose the new collections endpoint and logs a handled 404; the complete catalogue fixture supplies it.

The final Android patch was installed in place after a paced transfer with a verified checksum. Its SHA-256 is `461b473760b2f05f28c77c0595ca3d917a617d92b7c37d7e5f32c2467fb5b215`. The first transfer timed out and safely stopped before installation; its temporary file was removed. Foreground/package guards and the default Home were retained.

On the installed final patch, a resumed source stayed in buffering during the short test. One Back returned to Cue Home, confirmed through the resumed activity. The phone's Now state now reports buffering. The unchanged history offset is not proof of saved progress or resumed playback; those remain separate from the successful Back check.

## Local evidence

Evidence is intentionally ignored by Git. Remote update: `dist/apple-tv-research/cue-tv-remote-controls-install.json`; prepared rollback: `dist/couch-tv-sitting-room-youtube-feeds.apk`. iPhone test bundle: `dist/apple-tv-research/cue-tv-remote-youtube-tests.xcresult`. Current iPhone build/install records start `cue-tv-iphone-`; tvOS naming records start `cue-tv-tvos-name-`. The private physical Control Centre capture is `dist/private/cue-tv-control-centre.png`.


## Final physical check and shutdown

The final patch then played a different cached Real-Debrid episode inside Cue TV. The Now state changed from buffering to playing and the actual clock advanced beyond seven seconds. The local library recorded the exact second episode, 13,067 ms of 438,036 ms, marked in progress. One Back returned to Cue Home again. The short first resumed-source check did not establish resumed playback; startup on the fresh episode took roughly half a minute before playing. Do not describe source startup as instantaneous or all formats as tested.

The TV was switched off after leaving the player, as requested. The authenticated power response reported off. A separate ADB power query was unavailable in standby; no wake was sent. Final local records: `cue-tv-player-back-install-paced.json`, `cue-tv-android-back-live.json`, `cue-tv-android-final-playback-and-power.json`, `cue-tv-final-power-confirmed.json`, `cue-tv-remote-key-compatibility.xcresult` and the final privacy logs under `dist/apple-tv-research/`. No household captures or build artifacts are shared in Git.
