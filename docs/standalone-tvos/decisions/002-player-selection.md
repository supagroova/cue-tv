# 002 — use VLCKit for continued standalone player integration

Status: selected for implementation, conditional on remaining resource and device qualification. This decision does not close Phase 2.

## Compared revisions

- VLCKit `3d0f0fa7031eab1a67c817c98a495e1c68fc998c`, exact binary checksum in DEPENDENCIES.md.
- MPVKit `d9194243bdc68b87d8989376ab52aee7da6c8e0b`, `MPVKit` product and exact binary checksums in its pinned Package.swift. The separate reproducible probe is under probes/mpv-player and excluded from routine CI.

Both were exercised on the tvOS 26.5 simulator using the same project-owned media. This is not physical Apple TV evidence.

| Fixture/behavior | VLCKit | MPVKit |
| --- | --- | --- |
| H.264 High/AAC LC MP4 | Torrent adapter playback clock advances | Local clock advances; video screenshot inspected |
| H.264 High/AAC LC MKV | Local clock advances; video screenshot inspected | Same |
| HEVC Main 8-bit/AAC LC MKV | Local clock advances; video screenshot inspected | Same |
| Two AAC tracks | Remote selection of track 2 confirmed by player API | Track 2 requested at start and confirmed by player API |
| Embedded SRT | Rendered text visually confirmed | Rendered text visually confirmed |
| Embedded WebVTT | Track switch followed by new cue; rendered text confirmed | Track 2 selected at start; rendered text confirmed |
| Pause/resume/local seek | Remote-driven clock assertions pass | Not implemented in the probe |
| Audible output, hardware decode mode, CPU/memory/energy | Not qualified | Not qualified |

The final MPV probe also built successfully for unsigned arm64 tvOS. The matrix test passed four launches in a 26.133-second test session. Its initial C interop compile errors were corrected before runtime testing. Screenshot evidence includes the HEVC video, WebVTT text and selected audio/subtitle IDs of 2. Fixture hashes and exact profiles are in the checked-in media manifests.

## Rationale and limits

VLCKit already connects to the embedded engine through the loopback adapter and integrates with the typed lifecycle coordinator. Its drawable/track APIs require less custom rendering integration here than MPVKit's patched MoltenVK/CAMetalLayer context. The [pinned MPVKit README](https://github.com/mpvkit/MPVKit/blob/d9194243bdc68b87d8989376ab52aee7da6c8e0b/README.md) explicitly describes the package as learning-oriented with infrequent maintenance. That is a maintenance concern for this particular distribution, not a claim about upstream mpv quality.

No speed, energy or picture-quality advantage is claimed. Keep the MPV probe as a fallback reference. Revisit the selection if VLCKit fails required physical-device decoding, resource limits, uncached network seeks or complete dependency/license review. Do not drop MKV or subtitle requirements to preserve this decision.

Remaining acceptance includes HEVC MP4 coverage, live alternate-audio verification, decode-mode measurement, sustained resource measurements, app-wide cache limits, streaming MKV/repeated uncached VLC seeks, remaining failure paths and physical qualification.
