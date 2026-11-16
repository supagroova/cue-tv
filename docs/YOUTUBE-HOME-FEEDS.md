# YouTube Home feeds on each iPhone

## Sign-in loop repair, native build 4

On 16 November, Rupert confirmed that the embedded Home renders but sign-in loops or returns to YouTube without signing in. Native **1.0 (4)** is now installed in place on DeveloperPhone, with the installed version independently confirmed. The phone was locked when remote launch was attempted, so the owner was asked to unlock, open TV and retry sign-in. Real Google authentication, recommendations and recommendation-to-TV playback are still unverified.

The preceding build silently cancelled Google/YouTube destinations outside a short host list, including the YouTube account subdomain, and rejected normal blank subframes. It also replaced newly opened website windows with a load into the original page, losing the original window relationship. The repair allows ordinary HTTPS navigation on Google/YouTube domains, preserves browser subframes and uses WebKit's supplied configuration for account windows. Those windows open through the root view controller at full screen with a Back action.

The regression test also exposed a separate lifecycle defect: returning from the account window reloaded the connected remote, resetting Watch to Remote. A ready connection now preserves its existing page. The final test verifies an actual touch in a synthetic account window, the message back to its opener, window closure and the original feed remaining visible. The about:blank test fixture uses a coordinate touch at its visibly rendered button; this does not establish VoiceOver acceptance for real Google content.

Twenty-one unit checks and all three native UI checks passed in `youtube-signin-final-tests.xcresult`. The signed Release build and strict signature/Keychain checks passed. Earlier failing runs are retained as diagnostics. No household credentials were used in the simulator. Google may still restrict full sign-in in an embedded browser, so these checks do not prove a real account can sign in. No cookie export, browser impersonation, OAuth-token injection or tracking-protection change was used.

Signed app: `dist/ios-youtube-signin-device/Build/Products/Release-iphoneos/CouchRemote.app`. IPA: `dist/CouchTV-Remote-YouTube-SignIn.ipa`. Current hashes and delivery state are in `dist/apple-tv-research/youtube-signin-release.json`. The preceding build 3 app and IPA are retained. No TV installation/input action was needed and SecondPhone has not been updated.

The earlier delivery history follows.

16 November 2026. The TV interface is installed. Native iPhone **1.0 (3)** is installed in place and launched successfully on DeveloperPhone after Rupert connected it by USB. The installed app version was independently confirmed. SecondPhone remains on its preceding binary. Real-account inline Home and the new video handoff remain household checks.

## Use after the phone update

Open **Watch → YouTube**. With no search entered, YouTube's own Home website occupies the former empty-search area. The feed dropdown selects a saved sign-in on this iPhone. Search retains its existing results; clearing search returns to Home.

**Settings → Manage YouTube feeds** adds, renames or removes feeds. Each phone starts with one **My YouTube** feed and can hold up to eight. Rupert can rename his first feed Rupert and add Ursula; Ursula's phone can keep only her own. Adding opens YouTube's account page for that feed's sign-in. Rename keeps the sign-in. Removal requires confirmation, clears only that feed's website data and leaves at least one feed. No household login has been copied or preconfigured.

Each phone has its own list. A stable UUID selects a separate persistent WebKit website store for each feed. Google sign-ins remain inside those stores. The app does not read or export Google cookies, passwords or recommendation lists. Only feed names, IDs and selection are passed to the first-party remote. Pairing retains its original separate store and Keychain group.

Home feeds are separate from **Subscriptions, Playlists and Liked**, which retain the existing single optional API-library connection per phone. Changing a Home feed does not change that connection or the account selected in YouTube on the television. Playback uses the TV's selected account.

Selecting a supported video validates its HTTPS URL and sends only the video ID using this phone's pairing. Acknowledged handoff returns to Remote. Errors remain visible with retry; an unpaired phone is told to pair. Google website content has no native message handler. The Safari Home shortcut remains available; its earlier physical TV-button failure is not newly accepted or fixed by these tests.

## TV delivery

Large Home and account cards are replaced by compact Home and Settings controls. Account notices are inside Settings. Stremio/YouTube app-shortcut rows are removed from both normal Watch panels; Main Remote shortcuts and kids-only allowed-app access remain.

After Rupert explicitly confirmed TV availability, the guarded installer updated `dist/couch-tv-sitting-room-youtube-feeds.apk` in place. SHA-256: `cdcc712598033468fa2cb4f7d31c04fa7557fd25518d0ab6915ee2fbb44a8851`. Transfer checksum and package-update guards passed. Six pairings, the bound home watch, Google setup and Stremio link remain. The final read returned eleven Stremio library items and five separate local history records. The custom Android home is visible. Served JS, CSS and both information pages match the production build byte for byte. Older phone binaries retain search and the small external Home shortcut until updated. No always-on Mac is required.

## Native artifact and checks

- Signed app: `dist/ios-youtube-inline-device/Build/Products/Release-iphoneos/CouchRemote.app`.
- IPA: `dist/CouchTV-Remote-YouTube-Feeds.ipa`, SHA-256 `051fe9ebbe4c7a6e74694b693e80eb9e0fad68c08be530327b7eea17b1c0167d`.
- Bundle ID and Keychain group are preserved. The development profile covers both phones and expires 15 November 2027 at 02:30 UTC. Deep strict signature verification passed.
- Seventeen native unit checks passed, covering persistence, validation, rename/remove, geometry, playback failures and separate stores with synthetic cookies. That earlier mixed run also had a failing UI test and is not recorded as an overall pass.
- Five browser checks and two final native UI checks passed. The final UI tests cover add/rename/remove and inline Home/search/Remote transitions. Exported native screenshots were visually reviewed. Preview accounts are synthetic.
- Frontend build, Android release packaging/lint and generic signed iPhone Release build passed. The unchanged backend's preceding suite had 128 checks, with one optional public YouTube check skipped.

Evidence under `dist/apple-tv-research/`: `youtube-feeds-tv-install.json`, `youtube-feeds-live-check.json`, `youtube-inline-signing.json`, `youtube-multiple-feeds-final-tests.log`, `youtube-multiple-feeds-browser-check.log`, `youtube-feeds-final-ui.xcresult` and `youtube-feeds-final-ui-attachments/`.

## Remaining checks

DeveloperPhone installation is complete. Evidence: `iphone-youtube-feeds-delivery.json`, `iphone-youtube-feeds-install.json`, `iphone-youtube-feeds-installed-app.json` and `iphone-youtube-feeds-launch.json`. The original signing identity and Keychain group remain; no uninstall or trust reset was used. The TV still reports six clients, its bound home watch and Stremio connection. This is not proof of the phone's authenticated UI, so the owner was asked to confirm the feed view on the physical phone.

Install SecondPhone in place when it is connected and unlocked. Sign into each named feed, confirm real recommendations persist after relaunch/switching, and verify a recommendation plays on the intended TV platform. Google processed a reserved invalid identifier in the isolated WebKit probe, which does not establish successful full real-account sign-in. Account-specific Google authentication restrictions remain possible.

The accepted Stremio phone flow is unchanged. The owner's earlier acceptance of his real Home feed in Safari does not establish the new inline flow or multiple real-account sessions.

The existing public [app information and privacy pages](https://sitting-room-tv-information.rupslay.chatgpt.site) were updated in place to explain inline Home and separate website sessions. Site version 3 deployed successfully, preserving its existing public audience. No Google scopes or account registration changed. Source and deployment evidence: `youtube-feeds-information-site.json`.

Rollback artifacts remain available. Use the guarded in-place TV updater with the preceding working APK; do not clear app data. The preceding signed iPhone build 2 is retained separately.
