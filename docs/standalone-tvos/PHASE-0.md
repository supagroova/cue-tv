# Phase 0 — Playback Lab and fixture baseline

Status: implementation verified in simulator; physical-device gate pending.

## Delivered

Debug builds expose Playback Lab without an Android connection. Launch with `CUE_PLAYBACK_LAB=1` to open it immediately, or use the Playback Lab button. AVPlayer plays the generated local control clips; Stop, dismissal and backgrounding release the player item. The screen explicitly says the torrent engine is not connected. Release builds exclude the lab interface.

Three 12-second 1080p SDR H.264/AAC clips are generated from solid colors and sine audio. One has tail metadata; the other two use fast-start MP4. `scripts/fixtures/generate.py` regenerates media, SHA-256 checksums and BitTorrent v1 metadata. `fixtures/torrents/manifest.json` records file indexes, hashes, generator version and permission. The multi-file case has two episode clips plus a tail-index control. There are no web seeds or embedded trackers; discovery and controlled seeding belong to Phase 1. The short clips establish decoder/metadata controls, not the later ten-minute/one-hour acceptance runs.

## Verification

The new fixture tests initially failed because PlaybackFixture did not exist. After implementation, seven Swift Testing tests pass on tvOS 26.5 simulator, including the existing five tests. Path traversal in fixture filenames is rejected and every bundled manifest entry resolves to a resource. The simulator UI test also passes (one test): it launches the lab without pairing, starts a fixture using the remote and stops it. CI runs that test alongside the unit suite. Existing harness tests remain required in CI. Generated project files and local signing settings are not committed.

## Physical gate

The available Mac has Xcode 26.6 (17F113), and signing identities exist. Device listing showed no paired Apple TV. The user explicitly chose simulator-first implementation and deferred physical pairing. Simulator checks may advance the development PR stack; physical tests remain release qualifications. Its actual OS/build, team/profile provisioning, signed installation, detached launch and storage remain unverified. Do not mark G0 complete or this stack release-ready from CI alone.

## Next

Phase 1 can prepare and compile the native engine, but production integration must wait for direct peer playback evidence. Use the same fixtures to distinguish decoder success from byte transport. No changes to the Pi or existing installed Apple TV apps have been made.
