# TV for iPhone

A signed native iPhone remote for Sitting Room TV. The TV serves the existing phone interface. Swift supplies an offline power screen and directed Wake-on-LAN, so neither the phone controls nor power-on need a computer left running.

## Installed on DeveloperPhone: inline Home and multiple feeds

The current native update is **1.0 (4)**, installed in place on DeveloperPhone after Rupert reported a YouTube sign-in loop. It corrects account redirects, preserves sign-in windows and retains the connected remote page when a window closes. Twenty-one unit and three UI checks passed. Installation and the installed version are confirmed; remote launch was denied because the phone was locked. The owner was asked to open TV and retry the real sign-in. Authentication and recommendation playback remain unverified. Current app: `../dist/ios-youtube-signin-device/Build/Products/Release-iphoneos/CouchRemote.app`; IPA: `../dist/CouchTV-Remote-YouTube-SignIn.ipa`. See `../dist/apple-tv-research/youtube-signin-release.json`. SecondPhone remains on its preceding build.

The preceding build 3 delivery is recorded below.

Native **1.0 (3)** is installed in place and launched on DeveloperPhone on 16 November 2026. The installed version was confirmed through the device tools. The update preserved the signing identity and original Keychain group, without uninstalling or resetting trust. SecondPhone remains on its previous version. The signed package is `../dist/CouchTV-Remote-YouTube-Feeds.ipa`. Evidence is `../dist/apple-tv-research/iphone-youtube-feeds-delivery.json`.

After installation, **Watch → YouTube** shows YouTube's own Home website in the empty-search area. Its feed dropdown switches between separate sign-ins on this phone. **Settings → Manage YouTube feeds** adds, renames or removes feeds. Rupert can keep Rupert and Ursula, while Ursula's phone can keep just hers. Each feed needs its own sign-in in YouTube. The API library connection and TV playback account remain separate.

Seventeen unit, five browser and two final native UI checks passed. The signed Release build preserves the bundle identity and original Keychain group, with a profile covering both phones. Full real-account sign-in, persisted recommendations and recommendation-to-TV playback remain household checks. See [YouTube Home feeds](../docs/YOUTUBE-HOME-FEEDS.md). Compact controls and removal of app-shortcut rows from both normal Watch panels are already installed on the TV and load when either phone app is reopened.

## Previously installed on DeveloperPhone

The previously delivered native update is **1.0 (2)**, installed and launched on 16 November 2026. It adds the actual YouTube Home website in a Safari browser view, with a TV toolbar button. Open **Watch → YouTube → Open YouTube Home**, sign into YouTube there, choose a video and tap the TV icon. The browser's account menu lets each phone change accounts without an app reinstall or a Google test-email list. This is separate from the optional API connection for subscriptions, playlists and liked videos.

Source includes `YouTubeHomeView.swift`, `Shared/YouTubeVideo.swift` and the bundled `YouTubeAction` extension. The action uses this phone's original Keychain pairing and forwards only a validated video ID to the TV's existing platform-aware route. It never reads Google cookies or copies the recommendation feed. On a successful request it pauses the phone video. Both native targets retain the original Keychain group; legacy-pairing compatibility is tested.

Current signed build: `../dist/ios-youtube-feed-device/Build/Products/Release-iphoneos/CouchRemote.app`; IPA: `../dist/CouchTV-Remote-YouTube-Home.ipa`. Both app/action profiles include DeveloperPhone and SecondPhone and expire 15 November 2027 at 02:30 UTC. Eleven unit tests and Home/video-action UI checks passed. Physical signed-in feed and TV handoff acceptance are pending. SecondPhone was unavailable for installation, so her existing binary is still the previous version. The historical paths and original installation evidence below are preserved.

The development build is `../dist/ios-device-build/Build/Products/Debug-iphoneos/CouchRemote.app`. The packaged copy is `../dist/CouchTV-Remote.ipa`. Both contain this household's TV address, but no pairing credentials. The existing development profile expires on 3 November 2027; this is a development installation, not an App Store release.

On 15 November 2026, `devicectl` confirmed installation and launch on DeveloperPhone. Rupert confirmed that the app turned on the TV, paired using the on-screen code and works well. The TV then reported four paired clients, retaining the three existing pairings. Evidence is in `../dist/iphone-physical-acceptance.json`.

For a future in-place update, connect and unlock the phone, then use Xcode's device tools. Use the connected phone's identifier from `devicectl list devices`, then:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device install app --device "$IPHONE_DEVICE_ID" \
  ../dist/ios-device-build/Build/Products/Debug-iphoneos/CouchRemote.app
```

Open **TV**, allow Local Network access, and pair this app with the code displayed on the TV. Existing Safari pairing stays in place. If iOS requests Developer Mode or developer trust, complete that on the phone. Keep the phone on home Wi-Fi.

The Watch screen includes **Search films and TV shows**. The TV hosts this interface, so phone-interface updates do not require a new iPhone installation. Fully close and reopen TV to load a newly installed interface if it is already open during a TV update. Stremio is now linked to Rupert's account; real library and in-progress items are available.

## Wife's phone

The same app was built, signed, installed and launched on SecondPhone (2), an iPhone 16 Pro Max on iOS 26.6.2, on 15 November 2026. Rupert enabled Developer Mode on that phone. The refreshed development profile includes it, with no app source changes. The device-specific signed build is `../dist/ios-wife-build/Build/Products/Debug-iphoneos/CouchRemote.app`; the original DeveloperPhone build remains available.

Each phone pairs independently with the TV and retains its own pairing token. Rupert confirmed successful pairing and visible remote controls on her phone. No token or account data from Rupert's phone was copied. Both remotes use the existing Stremio connection stored on the TV, including Watch search and viewing history. Her development profile expires on 15 November 2027. Installation and pairing evidence is `../dist/iphone-wife-install-evidence.json` and `../dist/wife-tv-pairing-after.json`.

## How power-on works

When the TV is asleep, its web server may be unreachable. The native app sends a standard 102-byte magic packet directly to the TV's private IPv4 address on UDP port 9. It then retries the authenticated TV power-on endpoint for a bounded period. No broadcast, multicast entitlement, cloud relay, Mac service or power-policy changes are used.

The TV address and MAC address are in the ignored `TVConfig.plist`. Settings can update them if the home network changes. Pairing happens independently inside this app's web view. Its own pairing token is also stored in the iPhone Keychain for power-on requests; the setup Mac's token is never included in the app.

The directed wake route has worked from this Mac with the TV web service unreachable. A command-line integration check compiled from the exact `TVTarget.swift` and `TVConnection.swift` source also woke the TV and confirmed power on in eight seconds after normal standby. Evidence is in `../dist/native-swift-wake-check.json`. Rupert has since confirmed successful power-on and pairing from the physical iPhone. Longer standby, lock/unlock recovery and detailed touch/haptic checks have not been separately verified. A simulator cannot prove those behaviours.

## Build and checks

Requires Xcode, XcodeGen and the existing Apple development identity. Minimum iOS version is 17. Copy `TVConfig.example.plist` to `TVConfig.plist` for a new local checkout, and fill in that TV's real addresses. Do not commit household configuration or signing material.

```sh
xcodegen generate
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcodebuild -project CouchRemote.xcodeproj -scheme CouchRemote \
  -destination 'generic/platform=iOS' \
  -derivedDataPath ../dist/ios-device-build build
```

The final signed build passed `codesign --verify --strict`. Four simulator unit tests check the packet contents, invalid and broadcast MAC rejection, private IPv4 validation and bundled configuration. A real TV connection also loaded the pairing page in the simulator. See `../docs/TVOS-DESIGN-REVIEW.md` and the parent `SETUP-STATUS.md` for evidence and remaining checks.

## TV-hosted hybrid and YouTube update, 16 November 2026

Watch now has a Stremio / YouTube selector with separate retained searches, and an Apple TV / Android TV dropdown. Home and app/title selections route to the observed platform through the Android-hosted service. The native iPhone binary is unchanged; fully close and reopen TV to load the new interface on each phone. Existing pairings remain. The native Apple TV app and official Stremio are installed; Stremio still needs user sign-in on Apple TV. See [the current Apple TV setup notes](../apple-tv/README.md) for verification, signing, power-recovery limits and rollback.
