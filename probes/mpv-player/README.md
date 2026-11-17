# MPV candidate probe (manual only)

This separate app compares MPVKit with the owned fixtures used by the VLC lab. It is not a dependency of Cue TV and is not run by ordinary CI. MPVKit is pinned to revision `d9194243bdc68b87d8989376ab52aee7da6c8e0b`; that revision's Package.swift pins binary artifact URLs and SHA-256 checksums. It selects the `MPVKit` product, not `MPVKit-GPL`. The upstream package license is retained here; this is not a complete binary redistribution notice bundle.

From the repository root, with Xcode and XcodeGen installed:

```sh
xcodegen generate --spec probes/mpv-player/project.yml
xcodebuild -project probes/mpv-player/MPVProbe.xcodeproj -scheme MPVProbe \
  -configuration Debug -derivedDataPath .agent-work/mpv-derived \
  CODE_SIGNING_ALLOWED=NO -destination 'platform=tvOS Simulator,id=YOUR_SIMULATOR_UDID' test
```

The test launches the same H.264/AAC MP4, H.264/AAC MKV, and HEVC Main/AAC/SRT/WebVTT MKV used in the VLC lab. It waits for the real MPV playback clock and captures screenshots; a fourth launch requests the second audio and subtitle tracks and checks the selected track IDs. Inspect the screenshots for video/subtitle rendering. It does not measure audible output, energy, hardware decode, uncached network seeks or physical-device behavior.

The probe uses the package's tvOS `moltenvk` context with an owned CAMetalLayer. This is deliberately a feasibility app, not production lifecycle architecture: native initialization and destruction are synchronous, and the source is a bundled file. Do not copy it into production without addressing threading, lifecycle and error handling.
