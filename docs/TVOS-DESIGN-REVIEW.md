# Sitting Room TV design review

15 November 2026. Authorised implementation and one fresh review of the resulting interface. The brief is an easy family TV home with the feel of tvOS and the Apple TV app, Stremio as the main destination, YouTube within one action, and an iPhone remote that can turn the TV on without a computer running.

## Source and installation

Source is the local working tree based on upstream `9fade6757a94f71c77f5f1d8721cf3a026a7501e`. The TV is a Blaupunkt Google TV running Android 14. This is a web interface inside the existing Android home app, with a new SwiftUI/WKWebView iPhone companion. It is not Apple's tvOS software or a modified Stremio player. Stremio opens its own installed Android app for episodes and playback.

The final redesign is installed and inspected on the actual TV: `dist/couch-tv-sitting-room-tvos-final.apk`, SHA-256 `64dbe9d32398a9a152c53977a76032dfa3cc34df73368602b3af0f9204c71bcc`. The focus correction makes the white action surface exclusive to the current focus. Installation, checksum and temporary-file cleanup are confirmed in `dist/tvos-final-update-evidence.json`. Original interface files are retained in `dist/tvos-before/`; the earlier signed APKs remain available for rollback without clearing app data.

## Changes

| Before | After | Why |
| --- | --- | --- |
| The home relied on tabs, compact controls and several competing outlines. | A left navigation rail, large artwork, quiet background and readable rows organise Home, Movies, TV Shows, Library, Apps and Search. | The family can recognise where they are and browse from across the room. |
| Selection competed with persistent white primary buttons. | Focus lifts the selected control and gives it a white surface or a clear card edge. The resting primary action is translucent. | The person holding the remote can see which action will happen when they press OK. |
| App switching dominated the browsing surface. | Stremio titles have artwork and details; Watch in Stremio and YouTube remain prominent. | Choosing a show feels like a TV catalogue while common apps stay easy to reach. |
| Search did not search the custom catalogue. | Movie and series search uses Cinemeta through an authenticated TV endpoint, with empty, loading, failure and retry states. | Someone can find a title without navigating separate apps first. |
| Directional focus and returning from details were fragile. | Spatial directional navigation, explicit sidebar entry/exit, initial focus, modal focus containment and restoration to the original card. | Browsing stays predictable using a TV remote or the iPhone controls. |
| The phone power button was disabled in the TV-hosted mode. A sleeping TV could also stop serving the web page. | Reachable standby can use the existing power endpoint. The native iPhone app has an offline power screen and sends a directed wake packet before reconnecting. | Turning on the TV does not depend on a running Mac or an already reachable web page. |
| Personal rows could only be demonstrated with sample history. | The real TV shows honest popular rows and a Stremio account connection path. Personal history is populated only from the linked account; trending requires an existing compatible add-on. | The home does not imply that popular titles are household viewing history or pretend that account setup is complete. |

At 1920 pixels wide, the TV layout uses a 20-pixel base size, 80-pixel right content padding, a 170-pixel content origin allowing room for the rail, 80-pixel hero titles and 360-pixel landscape cards. Focus changes take 150 to 180 milliseconds. Reduced Motion removes transitions and smooth scrolling. The iPhone's offline power action has a minimum height of 56 points and uses native controls and typography.

## Fresh review and verification

The fresh pass found that a resting white primary action could compete with the focused YouTube action. That finding was fixed in the final CSS and verified on the TV. Details open and Back restores the original MobLand card. The hero action opens that show's Stremio episode selector; the YouTube action opens YouTube; Home returns from both. The TV home watch remains bound, secure self-ADB remains connected over TLS and all three paired clients are retained. Device evidence is `dist/tvos-device-final-check.json`. No further supported finding was identified in the inspected coverage; the physical phone and accessibility gaps below remain open.

| Lens | Coverage and result | Limits |
| --- | --- | --- |
| Task and information | Stremio and YouTube are prominent. Public popular feeds, configured trending feeds and real account history have distinct sources and labels. Rupert has linked his account; the live API returns six library items, including five in progress. Search returns sanitised movie and series results. | Actual playback remains unverified. No existing configured add-on currently supplies a trending row. |
| Platform and input | Browser checks passed for directional navigation, details, Back restoration, sidebar return, Movies, Apps, Settings and search. Actual TV screenshots and API control checks are separate evidence. The native app is installed on DeveloperPhone; Rupert confirmed successful power-on, pairing and that it works well. | Rupert's physical TV remote and detailed phone gesture/recovery checks remain unverified. |
| Accessibility | Semantic controls, descriptive labels, visible focus, modal containment, image fallbacks and Reduced Motion are implemented. | TalkBack, VoiceOver, large iPhone text and measured contrast over every changing artwork image have not been verified. |
| Layout and identity | TV previews checked at 1920 by 1080 and 1280 by 720. Phone checks used 393 by 852 with iPhone safe-area insets. No horizontal page overflow was found. The actual TV and native simulator screens were visually inspected. | Simulator images do not prove physical phone rendering. |
| States and language | Fixture checks covered loading, empty search, failed catalogue, retry, disconnected phone, reachable standby and native wake dispatch. Account linking explains where to sign in. | Real account failure and recovery need the account link. |
| Motion and feedback | Short focus transitions preserve location. Reduced Motion was exercised. The phone keeps its existing gestures; native wake has progress and completion/error feedback. | Real-device frame timing, touch gestures and haptics are unverified. |
| Responsiveness and implementation | Search ignores stale responses. Background requests do not replace the current focus. Swift networking is asynchronous and bounded. The web view handler checks the main frame and exact TV origin. Pairing uses the app's own token and Keychain. Mac wake tests passed and Rupert confirmed native iPhone power-on and pairing. | Longer standby, phone lock/unlock and lifecycle recovery have not been separately verified. |

The integrated RLP design workflow used Apple HIG, Apple interaction and typography guidance, Emil's restraint and feedback principles, Impeccable's hierarchy/states lens, Taste's honest data and density lens, OSXSkills' recovery principles, native iOS craft and UI UX Pro Max's directional focus/recovery guidance. macOS-specific window chrome, desktop hover, App Store publication and new onboarding are not applicable. Impeccable's engine, context command and detector were not run; existing project context was read manually. This is a manual product review, not a certification or numerical quality score.

## Evidence

- Actual TV before: `output/real-tv/after-final-restart.png`.
- Actual final home: `output/real-tv/tvos-final-home.png`.
- Final focus and Back checks: `output/real-tv/tvos-final-youtube-focus.png`, `tvos-final-title-details.png` and `tvos-final-after-back.png`.
- Actual Stremio episode selector: `output/real-tv/tvos-final-stremio-title.png`.
- Browser previews: `output/playwright/`. These use disposable fixtures and may show sample viewing history. They are not Rupert's watch history.
- Native offline interface: `output/real-tv/iphone-native-offline-final.png`, captured in the isolated iOS simulator.
- Native connection: `output/real-tv/iphone-native-live-connected.png`, showing the real TV's pairing page loaded by the simulator. The simulator was not paired and no setup credentials were copied into it.
- Android release build, lint and signing checks passed; 59 Android unit tests passed, including startup and Stremio search rules.
- Four native unit tests passed. The final device build and strict signature verification passed. Logs: `dist/ios-device-final.log`, `dist/ios-unit-tests-delivery.log` and the associated result bundle.
- The exact Swift wake transport was compiled into a command-line integration check on this Mac. After normal standby, the TV web service was unreachable. Directed wake plus authenticated power-on succeeded in eight seconds, with no power-policy changes. Evidence: `dist/native-swift-wake-check.json`. That experiment ran on the Mac; the subsequent physical iPhone result is recorded below.
- Subsequent physical-phone acceptance: installation and launch on DeveloperPhone were confirmed by `devicectl`. Rupert confirmed power-on and pairing from the native app. A live TV API check confirmed four paired clients, the custom home in front and the home watch bound. Evidence: `dist/iphone-physical-acceptance.json`. The legacy screenshot utility cannot capture this iOS version; no physical-phone screenshot or automated physical-phone UI test is claimed.
- Browser network errors generated by deliberately failed API requests, absent fixture SSE endpoints and placeholder images are expected fixture events; no JavaScript page exception was found in the exercised paths.
- The final transfer required a longer bounded operation because the TV link was slow. Each failed temporary APK was removed; no incomplete transfer was installed. The successful file matched the local checksum before replacement. Mac ADB later timed out while reconnecting for extra UI XML, and one eight-second API read timed out during an app transition. Final launch checks used the TV-hosted API with a 30-second read allowance, inspected the actual screen and passed. The original failure logs are retained, not counted as passing checks.

## Remaining checks

Native installation, user-confirmed wake, pairing and Stremio account linking are complete. Longer standby, lock/unlock, app foreground/background recovery and detailed touch/haptic checks remain unverified. Verify the physical TV remote and streams the household intends to use. No streaming subscription has been changed, no production account was reset and no content playback success is claimed.

## Follow-up: Watch search

Rupert requested search inside the iPhone Watch section. A search field now appears below Watch Now, using the TV's existing authenticated Cinemeta search endpoint. Queries are debounced by 300 milliseconds; late responses are discarded. Films and TV shows appear in separate shelves. Clear restores browsing; empty results and errors have explicit messages and retry. Selecting a result opens the same title through Stremio and returns the phone to remote controls. The input uses 16-pixel text, the iPhone Search keyboard action, and a 44-pixel clear target.

The production phone build and Android release/lint/signature checks pass. Browser checks at 393 by 852 passed fast typing, film/series results, stale responses, empty results, failure/retry, clearing, exact title dispatch, keyboard attributes, no horizontal overflow and no JavaScript exceptions. The first automation attempt focused the input before the horizontal page animation settled; the test was corrected to wait for the Watch page to settle, then the full checks passed. No product change was made for that fixture timing issue. `output/playwright/iphone-watch-search-results.png` uses disposable title fixtures, not the physical phone or Rupert's account history.

Installed update: `dist/couch-tv-sitting-room-watch-search.apk`, SHA-256 `32c8617b7c3d56a97181ebc87e7d75f8ac4d4545c95afd17fbabcf772c966ab2`. The first installation preflight stopped because Google TV caught Home before the custom home returned; no package was changed. The custom home was explicitly reopened before the successful guarded retry. Checksum and temporary-file cleanup passed. The TV serves exactly the newly built phone JavaScript; live search for Bluey returns 30 results including the series. Four pairings, the bound home watch, the linked account and all six library items remain. Evidence: `dist/watch-search-update-evidence.json` and `watch-search-live-check.json`. After an initial CoreDevice connection failure, Rupert reconnected the iPhone and `devicectl` confirmed a fresh app launch (`dist/iphone-search-update-launch.json`). Physical search confirmation is still pending.

## Primary references

- [Apple: Designing for tvOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-tvos)
- [Apple: Focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection)
- [Apple: Layout](https://developer.apple.com/design/human-interface-guidelines/layout)
- [Apple: Understanding local network privacy](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy)
- [Apple: Choosing the right networking API](https://developer.apple.com/documentation/technotes/tn3151-choosing-the-right-networking-api)
- [Apple: Multicast networking entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.networking.multicast)
