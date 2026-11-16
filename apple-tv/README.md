# TV on Apple TV

## Phone-side episode and source selection, 16 November 2026

Build 1.0 (2) adds the selected-source relay used by the updated iPhone Watch interface. A phone can choose the show, season, episode and Torrentio Real-Debrid source. Android playback of an actual selected source has been visually confirmed. The signed native Apple TV update is installed and five unit checks passed, and actual selected-source playback has now been visually confirmed on Apple TV, including the correct episode and resume position from Android. Physical iPhone tapping and AirPods audio remain household checks. Existing title and YouTube requests remain supported. See [the flow, protocol and recovery notes](../docs/STREMIO-PHONE-PLAYBACK.md).

The household's native tvOS browsing app, installed on Living room on 16 November 2026. This is separate from the upstream `tvos/` bridge app. It shows the real Stremio catalogue and library through Sitting Room TV, with native focus, sidebar, search, title details and Continue Watching. Official Stremio handles streaming playback; the installed YouTube app handles YouTube videos.

The Android television hosts the API, account history, phone interface and paired Apple TV remote connection. No Mac service is required during everyday use. The Android service must be awake, including while the panel shows Apple TV over HDMI3. The tvOS app listens for phone requests only while it is in front. It does not depend on background execution.

## Using it

1. Fully close and reopen **TV** on both iPhones to refresh the TV-hosted interface. No iPhone reinstall or new phone pairing is needed.
2. Use the **Apple TV / Android TV** dropdown to select the screen. It follows the television's observed input when that input changes elsewhere.
3. In **Watch**, choose **Stremio** or **YouTube** and search. Each provider keeps its own query and results. Tap a result to open it on the selected platform and return the phone to Remote.
4. **Home** opens the custom home on the selected platform. The YouTube shortcut opens the selected platform's YouTube app.
5. Open **Stremio** on Apple TV and sign in with the same account used on Android. This is still pending. The custom home's library already has its own working connection to the Android service.

Apple TV showed its system profile chooser after waking. Selecting the highlighted existing profile and letting the transition finish returned to the custom home. YouTube also requested its first system Open confirmation; it was accepted during setup. A subsequent phone video request opened directly and was acknowledged. Do not automatically choose a household profile during normal use.

## Installed artifacts and signing

| Component | Artifact | SHA-256 |
| --- | --- | --- |
| TV-hosted interface and hybrid remote | `dist/couch-tv-sitting-room-youtube-hybrid-final.apk` | `7599fe68718e9fc0c6a47115b81777b4e456834d4accae29f1b062701e7aa800` |
| Native tvOS home and video relay | `dist/TV-AppleTV-YouTube.ipa` | `7dcf18ac6dcd2bb45b288a2a2af2d9ee80de7d8142617d8b2d5b4ac98562aada` |
| Official Stremio 2.1.1 (25), personally signed | `dist/Stremio-AppleTV-personal.ipa` | See its retained signing/install evidence in `dist/apple-tv-research/` |

The native app is `com.rupertlaycock.couchtv.appletv`, displayed as **TV**. Official Stremio's personalised bundle is `com.rupertlaycock.stremio.full`. YouTube is `com.google.ios.youtube`.

The existing paid development profile expires **15 November 2027 at 21:17:42 UTC** (16 November in Melbourne). These are development installs, not App Store releases. The signing team is kept in ignored `Config/Signing.mine.xcconfig`; `Signing.example.xcconfig` is the template. Keep the existing signing identity, bundle identifiers and Android keystore for compatible updates.

TV address configuration is in ignored `TVConfig.plist`. The tvOS app pairs once using a six-digit single-use code and saves its own token in Keychain. No Mac token or Stremio account key is embedded in the app or IPA. The Android service stores the Apple TV pairing privately, verifies its server identity and never returns the private keys to phones.

## Build and verify

Use Xcode 27.2 at `/Applications/Xcode.app`, with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`. Generate the project with `xcodegen generate --spec apple-tv/project.yml` after creating the local configuration files.

The currently paired physical Apple TV has UDID `0209415071066f8408e5cd794c2dd673b8d23b43`. Use that UDID for the Xcode build destination:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV \
  -destination 'id=0209415071066f8408e5cd794c2dd673b8d23b43' \
  -configuration Release -derivedDataPath dist/tvos-install-build \
  -allowProvisioningUpdates build
```

Refresh `xcrun devicectl list devices` before future installations. Its current CoreDevice UUID is `AC59F008-6D46-5D90-9987-70B28FC02C4D`. The older advertised identifier `6DE3F69E-4461-45DA-8787-E06F9534BE06` gave a misleading tunnel error. The correct paired identity supports installation, process launch and screenshots on this tvOS 26.6.1 device.

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcrun devicectl device install app --device AC59F008-6D46-5D90-9987-70B28FC02C4D \
  dist/tvos-install-build/Build/Products/Release-appletvos/SittingRoomTV.app
```

Normal Home actions launch the app bundle and send an authenticated foreground event. Do not launch `couchtv://home` from Companion or `--payload-url` during ordinary use: that raises an unnecessary system Open confirmation. The `couchtv://pair` URL is only for explicit initial setup. Device Hub's screen-sharing feature requires tvOS 27; the SDK screenshot command works on this tvOS 26 device without an OS update.

The disposable fixture is `tests/hybrid_fixture.py` on localhost8756. Native tests use only its public sample data through a DEBUG-only launch environment. Release builds use the household TV configuration and their own pairing. Never copy setup tokens into a fixture or simulator. `tests/youtube-phone-check.js` and `tests/hybrid-phone-check.js` exercise the production phone build against the fixture.

## Routing and search

`PlatformRules` reads Android's selected TV input session, with CEC routing as a fallback. It does not infer the selected source from an attached cable or an awake Apple TV. A native Android app wins over stale HDMI information. Unknown inputs require an explicit dropdown choice. The saved Apple input family includes HDMI3 and the Apple TV's CEC child input.

Phone video/title requests bring the native app forward, enqueue one expiring request and send it through the foreground event stream. The native app validates the provider and identifier, opens the installed streaming app and acknowledges the launch. Home, input changes and later requests cancel an unhandled request. An old acknowledgement cannot consume a newer request. The service waits outside the action lock, so Home can interrupt a pending request. Playback success is separate from a successful app-open acknowledgement.

YouTube search reads public YouTube search results directly from YouTube. It does not use the household's Google credentials, a third-party proxy or a new API key. Parsing returns only video ID, title, channel, duration and a validated thumbnail address. Advertising blocks and duplicates are skipped. Responses, nesting, result count and concurrent requests are bounded; a short in-memory cache limits repeated requests. This public page format can change. Search errors offer Retry and the normal Open YouTube shortcut. Personalised YouTube subscriptions and watch history are not implemented.

## Validation and limits

- 93 Android tests passed, including live public YouTube search, wire interoperability, selected-input rules, handoff expiry/cancellation and search parsing. Release build, lint and APK signing passed.
- Four native unit tests and one native UI test passed. These cover validated links, native focus/details/Back/search, Home events and a failed handoff acknowledgement when Stremio is absent from the isolated simulator. Release signing and actual device installation passed.
- Fourteen browser checks passed across the existing hybrid controls and new YouTube flow. YouTube checks cover provider state preservation, late responses, loading/empty/error/retry/clear, both dispatch targets and 320 to 900 pixel layouts. No JavaScript page exception or page overflow was found. The first test clicked before the horizontal pager settled; it was corrected to wait for the existing transition. No product change was needed for that timing issue.
- The real TV returned 16 YouTube results for Melbourne photography. The installed phone JavaScript matches the local build exactly. Six pairings, the bound home watch and eight Stremio library items (seven in progress) remain.
- Actual Apple TV YouTube playback was visually confirmed with the short public video `jNQXAC9IVRw`. The second handoff was acknowledged and opened without a prompt. Android YouTube opened from the video link, but the black screen capture and empty accessibility title check did not establish actual playback or the exact displayed title. This remains a household check. Home and Android-to-Apple input switching passed after the TV connection recovered.
- A first platform switch timed out during the post-install transition; it did not report success or route a title to an unconfirmed platform. A settled retry passed. Saved failure evidence is retained. A system profile chooser and first YouTube Open prompt delayed earlier handoffs, which correctly timed out. Those attempts are not counted as successful playback.
- The Android TV became unreachable during a long idle period. Directed wake alone did not restore it in that check. Apple TV wake/Home over the existing HDMI-CEC path restored ADB, and opening the existing Android home restored the API. The existing app and foreground service were still running. No power, CEC, network or debugging policy was changed. Long-standby reliability needs further household testing.
- Official Stremio opens through the native relay but currently shows its sign-in screen. Stremio stream playback and cross-platform progress updates after playback remain unverified. AirPods pairing, audio and volume need household verification. Volume keys currently control the television. The physical Siri remote's system Home behaviour is unchanged.

Current evidence: `dist/apple-tv-research/youtube-release-manifest.json`, `youtube-hybrid-final-install.json`, `native-youtube-install.json`, `final-service-checks.json`, `youtube-device-actions.json`, `native-youtube-tests.xcresult`, and the final browser/Android validation logs. Actual device screenshots are in `output/apple-tv/physical-*.png`; phone images there are fixture previews, not physical iPhone captures. Earlier failure screenshots are retained and should not be used as acceptance evidence.

## Rollback

The accepted pre-hybrid Android APK remains `dist/couch-tv-sitting-room-watch-search.apk`; hybrid v1 and the checked intermediate handoff build are also retained. A deliberate in-place update signed with the same task-local keystore can restore an earlier build without clearing phone/account data. Use the existing guarded installer only when the custom Android home is confirmed in front. Preserve all private pairing files and the keystore. Do not uninstall or clear data as a shortcut.

The Apple TV system home and existing apps remain available. Removing the custom **TV** app from the Apple TV removes only that app and its local library connection; it does not replace the system launcher. Full Android removal is documented in `docs/UNINSTALL.md`, and should only be run when Rupert explicitly asks for it. No rollback or removal was performed during this update.

Primary references: [official Stremio sideloadable release](https://blog.stremio.com/stremio-fully-featured-sideloadable-ipa-release-for-ios-ipados-apple-tv-tvos/), [Apple tvOS design](https://developer.apple.com/design/human-interface-guidelines/designing-for-tvos), [Apple focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection), [Apple app lifecycle](https://developer.apple.com/documentation/uikit/managing-your-app-s-life-cycle), [YouTube Data API search](https://developers.google.com/youtube/data/reference/rest/v3/search/list). The implementation here uses the public YouTube results page, not the keyed Data API.
