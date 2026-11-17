# Phase 2 — playback and resource policy

Status: in progress. This first slice adds native storage admission and write-time guards. It does not complete Phase 2 acceptance.

## Storage behavior

Every playback session now installs a storage wrapper. Before torrent files are created it accounts for all declared files, rounded to filesystem allocation units, plus metadata allowance. Unselected files are included so boundary-piece storage cannot bypass admission. Oversized torrents fail rather than silently exceeding the budget; rolling eviction is not implemented.

The provisional default is 4 GiB per session, a 1 GiB free-space reserve, a 16 MiB metadata allowance and at most 512 files. Before file resizing and each nonempty write, the wrapper rechecks allocated data, directory allocation and available space. It reserves capacity for all remaining declared data because filesystem allocation may exceed a single incoming block. Concurrent writes through the wrapper are serialized. Filesystem errors remain failures.

These are conservative session limits, not a measured final product policy or an app-wide quota. Other processes can consume disk between a check and a write. Cross-session ownership, acknowledged teardown and abandoned-cache cleanup remain required before production use. Metadata allocation is observed and conservatively reserved; this is not an operating-system enforced filesystem quota.

## Evidence

Local Rust suite: 16 tests passed, including four storage tests covering oversized admission, reserve rejection, unselected-file accounting and a simulated disk-space drop during real peer transfer. The latter verifies that later blocks remain unwritten and the session reports failure without filling the host disk. A subsequent admission regression demonstrated that existing directory allocation could create a file before rejection; admission now includes the larger of observed metadata allocation and its allowance. All five storage tests pass after that fix. The existing uncached-seek and blocked-response cancellation regression also passes.

`make apple-build` passed for the unsigned arm64 Apple TV target with the storage wrapper linked. No physical device result is claimed. Sustained playback remains manual-only and is not part of routine push/PR CI.

## Storage shutdown barrier

The pinned upstream rqbit `Session::stop` pauses torrents, cancels tasks and sleeps for one second; it does not join all background tasks. Treating its return alone as proof that cache removal is safe would be incorrect.

The storage wrapper now shares a permanent close barrier across factory/storage clones. Close waits for current synchronous file operations, then rejects later creation, initialization, read, write, resize, removal and storage transfer operations. A session-owned lease closes it on explicit stop, drop and cancelled/failed setup. The C API now offers an owned stop ticket: status 0 means pending, 1 confirms the owner finished and storage is closed, and -1 means unknown/released. Callers must release tickets, including abandoned waits. This acknowledgement does not claim every upstream network task was joined. Swift lab sessions retain their unique cache directory and remove it off the main actor only after status 1. A five-second timeout preserves the directory and records a cleanup failure; stale cleanup captures only its old directory. Abandoned caches after app termination and app-wide capacity accounting remain outstanding. The barrier serializes reads as well as writes, so its performance cost remains part of resource qualification.

A regression exercises a late engine initialization through a closed real storage factory and verifies no files are created. Two native C ABI regressions pass for cancellation during setup and a ready session, including ticket release and repeated-stop behavior. All eight Swift unit tests pass; the real-engine source-switch test verifies removal of the first directory without deleting the replacement, followed by removal of the second directory after Stop (2.342 seconds).

## Candidate decoder evidence

VLCKit is pinned to the full revision and binary checksum documented in DEPENDENCIES.md. A debug-only candidate screen is enabled with CUE_PLAYBACK_LAB=1 and CUE_VLC_LAB=1. The generated MKV fixture is a remux of the owned 1080p H.264 High / AAC LC clip; its source/output hashes, duration and stream profiles are in Resources/PlaybackLab/mkv-manifest.json. Regenerate using scripts/fixtures/generate-mkv.py.

The simulator UI test `PlaybackLabTests/testMKVPlaybackWithVLCCandidate` passed: explicit remote selection, playback clock beyond 500 ms, screenshot attachment and remote Stop returning to the start control. The blue fixture image was visually confirmed. The initial test exposed focus moving to the unrelated AVPlayer lab controls; the candidate now uses its own lab screen. The final test took 7.465 seconds. The candidate also compiles for unsigned arm64 tvOS.

A follow-up simulator test connects VLC to the real embedded torrent engine and controlled local seeder through the loopback HTTP adapter, waits for playback progress, then stops through the remote. All four short Playback Lab UI tests passed in 31.103 seconds, including the existing AVPlayer regressions. Transfer cancellation is requested on Stop, disappearance and backgrounding; the production coordinator still requires lifecycle qualification. The torrent fixture is MP4 and is small, so this does not establish streamed MKV or playback before complete download in VLC.

This proves one local MKV decoding path and the MP4 torrent-to-VLC connection only. Audible audio, HEVC, alternate tracks, subtitles, background teardown and the MPV comparison remain unverified. VLCKit has not yet been selected as the production player.

## Coordinator integration

`PlaybackSessionCoordinator` now serializes retirement, assigns a generation to each accepted start, and rejects events from retired generations. Its typed state vocabulary includes idle, resolving, discovering, buffering, playing, paused, seeking, stopping and categorized failure. A start superseded while waiting for old resource cleanup returns no generation and cannot proceed. The VLC lab uses it for local/torrent start, playback progress and stop; progress callbacks capture their original generation instead of updating a later session.

Validation: all ten Swift unit tests and four simulator Playback Lab UI tests passed. The coordinator tests cover stale failure/playback events and overlapping starts while teardown is held open.

This is initial lifecycle integration, not complete production playback: pause/seek controls, automatic failure teardown, bounded discovery/decoder waits and a stable product snapshot still need implementation. The existing lab's shutdown ticket bounds cache-cleanup waiting separately.

## Remaining work

Select and integrate a player supporting the required MKV/MP4 codec, audio and subtitle matrix; implement the typed playback coordinator and cache lifetime; measure CPU/memory and set justified limits; classify failures with bounded timeouts; qualify simulator and subsequently physical-device behavior. The isolated VLCKit compile probe is candidate evidence only, not production player selection.
