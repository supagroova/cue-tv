# Compact YouTube controls

16 November 2026. Implemented from Rupert's physical iPhone screenshot and request to make connection details and YouTube Home much less prominent. Source checkout base `9fade67`, with existing household changes retained. The interface is the React view served by Sitting Room TV inside the native iPhone app.

| Before | After | Why |
| --- | --- | --- |
| A full YouTube Home card, two explanatory paragraphs and a large button above results | One YouTube Home shortcut alongside Settings | Recommendations remain available without pushing videos below the screen. |
| A persistent account card with connection, disconnect and privacy information | A small Settings disclosure, closed initially | Account management is available when needed and stays out of everyday browsing. |
| Privacy links run together, with another API limitation note below the library | Separate wrapping links and the API note inside Settings | Connection information is grouped with the action it explains. |

## Implementation

`remote/src/YouTubeBrowser.jsx` retains the same native Home bridge and browser fallback. `YouTubeLibrary.jsx` owns a compact utilities row and an inline settings panel. It preserves the existing account operations, polling, independent-phone identity, subscriptions, playlists, liked videos and errors. Settings closes when leaving YouTube or choosing a library tab; disconnecting leaves Settings open for reconnecting. Escape closes the panel and restores focus to its button. Search text and results are retained.

The two utilities have 44 CSS pixel high touch targets, restrained text and existing icons. The row wraps at larger text sizes. The existing settings card is only mounted after an explicit tap; connection errors and setup notices are also inside that panel. No Stremio or YouTube backend, Google account, TV player or native iPhone binary changed.

## Verification and fresh review

- Frontend production build, Android packaging and release lint passed. No new backend unit suite was required for this presentation-only change.
- Four Home/settings browser checks, eight existing account checks and six existing search/routing checks passed. The account checks include device-code cancellation, reconnect states, personal list navigation, pagination, independent accounts on two sessions, and disconnect affecting only its own phone.
- Compact controls remain accessible and tappable; privacy links remain available in Settings. Escape restores focus and retains the search. The native bridge and older-client link still have their existing destinations.
- Browser layouts from 320 to 900 pixels and a deliberately long account name were exercised. A 320 pixel preview with enlarged utility and account text had no horizontal overflow. These are browser checks, not physical WKWebView or spoken accessibility acceptance.
- Fresh visual review accepted the settled connected-account view, expanded settings, disconnected native entry and narrow enlarged-text view. One early connected-account screenshot caught an unsettled paint; the settled view and computed control visibility were checked separately. No further supported in-scope defect remained in those views.
- Existing gestures, transitions, colours and playback were preserved. No new motion or service calls were added by opening Settings.

Evidence: `output/playwright/youtube-compact-connected-settled.png`, `youtube-compact-connected-settings.png`, `youtube-compact-large-text.png`, the `dist/apple-tv-research/youtube-compact-*-check.log` files and `youtube-compact-reaudit.log`. Previews use synthetic accounts and videos. The supplied screenshot remains private.

## Delivery

Prepared APK: `dist/couch-tv-sitting-room-youtube-compact.apk`. Hash and bundled asset names are recorded in `dist/apple-tv-research/youtube-compact-release.json`. The preceding working APK remains `dist/couch-tv-sitting-room-stremio-phone.apk`.

The initial delivery check found Apple TV playback, so installation was deferred. Rupert later explicitly confirmed the TV was free and authorised switching and installation. The compact release was installed, then superseded by the final `dist/couch-tv-sitting-room-youtube-feeds.apk`. Served assets match; six pairings, the home watch, Google setup and Stremio link remain. The custom Android home is visible.

The subsequent native iPhone update adds inline Home and separate named feeds. It was subsequently installed in place and launched on DeveloperPhone after Rupert connected it by USB. SecondPhone remains on its preceding native version. See [the current feed notes](YOUTUBE-HOME-FEEDS.md) for artifacts, checks and remaining real-account/playback acceptance. The original presentation-only review above remains evidence for the compact controls.
