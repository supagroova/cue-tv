# YouTube accounts on each phone

**Latest update, 16 November:** compact Home/Settings controls and removal of app-shortcut rows from both normal Watch panels are installed. Native iPhone 1.0 (3), with inline Home and separate named feeds per phone, is installed in place and launched on DeveloperPhone. SecondPhone has not yet received this native update. Real-account sign-in and recommendation playback checks remain pending. Each Home feed has its own website sign-in; the API library account and TV playback account remain separate. See [the current delivery and verification notes](YOUTUBE-HOME-FEEDS.md). The earlier release history below is retained.

Installed on Sitting Room TV on 16 November 2026. The live phone assets match the tested build. Google registration and real user authorisation are separate completion gates; do not describe an individual phone as connected until its authorisation passes.

The TV-hosted remote can connect a different YouTube account for each paired phone. This adds subscriptions, videos from subscribed channels, playlists created by that account, and liked videos. Public YouTube search remains available without connecting. The native iPhone binary does not change.

## Household flow

Open **Watch → YouTube → Connect YouTube** on each phone. The TV requests a Google device code, and the phone shows that code and Google's own verification link. Choose **Continue to Google**, enter the code exactly as displayed, select the intended Google/YouTube account and approve read-only access. Return to TV. Each phone displays its own connected account and personal lists.

The existing television YouTube app still uses the account selected inside that app. Connecting the phone's browsing account does not switch the television's account or transfer YouTube Premium benefits to a different TV account. This distinction appears in the interface. No account-switch automation is included.

## Actual personalised YouTube Home

The Google Data API does not supply personalised Home recommendations. Rupert specifically requested the real Home feed, so native iPhone build 1.0 (2) adds **Open YouTube Home**, backed by `SFSafariViewController`. This shows YouTube's own website with a browser-managed sign-in on that phone. It does not reconstruct a feed from subscriptions or use the API token in the browser. Account changes happen in YouTube's own account menu, independently of the optional API library connection.

The browser's TV icon invokes the bundled `YouTubeAction` extension. Safari supplies the current page URL. `YouTubeVideo` validates HTTPS, exact YouTube hosts, supported video paths and one 11-character video ID. Extra tracking and playlist parameters are discarded. The action reads only this phone's existing app Keychain pairing and current target, then uses the platform-aware `/api/youtube/open` route. Redirects are refused; errors do not report success. On success Safari's finalisation pauses the phone video. The app does not read Google cookies or scrape the recommendation feed.

The native app and extension use the app's original Keychain access group, so existing tokens remain readable. The current TV target is stored in that same private group for the extension. No Mac token or Google user token is bundled. Earlier native versions and ordinary browsers get an explicit link to YouTube on the phone; they do not claim the new native toolbar action is available.

DeveloperPhone's build is installed and launched. Its 11 unit tests and the Home/video action UI checks passed. Signed-in feed rendering and physical TV-button playback await the owner's acceptance; simulator checks prove the browser/action integration only. The signed profiles cover both household phones, but SecondPhone was unavailable and has not received this native update yet.

**Subsequent acceptance:** the owner confirmed that his real Home feed appears, but the physical TV button failed. He considers the browser flow clunky and asked to focus next on Stremio's show, episode and stream choices on the phone. Do not describe the Home playback route as working or continue redesigning YouTube without renewed direction. The optional API library connection was separately confirmed working.

Google's Data API explicitly does not expose Watch History or Watch Later. Personalised recommendations and playlists saved from other creators are not implemented here. The Playlists list uses `mine=true` and shows playlists the connected account owns.

## Private registration

A dedicated Google Cloud project, **Sitting Room TV**, was created under the owner's account. Its project ID is `fluid-cosmos-514901-q6`. Existing Gmail and website projects were not modified. No billing upgrade was accepted. The owner approved the YouTube API terms, and the console confirms **YouTube Data API v3: Enabled**. The owner completed Google's separate User Data Policy agreement. Google Auth Platform branding is Sitting Room TV, with an External audience so household accounts outside the business Workspace can connect. The TV client is named **Sitting Room TV household**, type **TVs and Limited Input devices**.

Complete these steps in that project:

1. Enable **YouTube Data API v3** after the owner approves its terms.
2. Configure Google Auth Platform branding and the external audience appropriate for the household's Google accounts. Use only the read-only scope `https://www.googleapis.com/auth/youtube.readonly`.
3. Create an OAuth client of type **TVs and Limited Input devices**. This is the TV application's OAuth flow, approved in each phone's external browser. It is not an embedded Google login in a WebView.
4. Download its JSON into ignored `dist/private/` with mode600. Keep the client secret out of source, screenshots, logs and chat. An iOS, desktop, web or borrowed third-party client is not a substitute.
5. Provision the already-paired TV using the script below. It reads the existing setup token from its private file; no token is entered in a command argument or printed.

```sh
.venv/bin/python scripts/youtube-account-setup.py \
  --client dist/private/youtube-oauth-client.json \
  --tv-url http://TV_ADDRESS:5400
```

The address may change. Check the current TV rather than assuming this address remains correct. Do not replace an existing app registration or clear phone pairings to get setup to pass.

Google issues refresh tokens that expire after seven days when an external OAuth app remains in **Testing**, except for basic profile-only scopes. YouTube access is outside that exception. Household test users must be explicitly configured in Google while the app is in Testing. Moving the consent configuration to Production and any required verification are distinct Google-console actions; do not silently describe a test registration as permanent sign-in or a public release. Each user still approves their own Google access.

**Current registration:** the owner approved publication of the information pages and moving to normal use. Google now confirms **In production**, External, with a 100-user cap for this unverified personal household app. New authorisations do not require a maintained test-email list or inherit Testing's routine seven-day lifetime. An earlier grant issued while Testing may require one fresh connection. Google can still revoke or expire grants for other reasons. No public Google verification claim is made. The owner confirmed his API connection and personal lists; his wife's authorisation is still pending.

The approved public homepage, privacy notice and use terms are at `https://sitting-room-tv-information.rupslay.chatgpt.site/`. Sites project `appgprj_6afa6564c988819199ad0541bfc0204c` has Public access; saved source commit `a4f6fb0d92cd1b16a26a16d9dbc99bfa2628b817`. The homepage and notices are the only published content. Browser access is confirmed; a Python HTTP client received Cloudflare 1010, which is not evidence of Google's crawler acceptance.

## Account isolation and storage

`Server` derives the connection namespace from the authenticated remote token using `Auth.clientIdentity`. Requests cannot specify another phone's account ID. No relationship is inferred from the household's selected Watch profile or a phone's display name.

`YouTubeAccounts` implements Google's device-code exchange, interval enforcement, slow-down, expiry, denial, access-token refresh and revoked-grant recovery. Credentials stay on the television. Only public connection state, the user verification code, account display fields and list display data are returned to the phone. The private device code, Google client secret, access token and refresh token are never returned. Every account endpoint needs the existing remote authentication and is blocked in kids mode.

`AndroidYouTubeVault` encrypts records with AES-256-GCM using an Android Keystore key. Each record name is authenticated as additional data, so copying one phone's encrypted record to another phone fails decryption. Each write uses a fresh IV. Private storage changes are committed before the HTTP response reports success. The app already disables Android backup. Network requests carrying Google tokens use fixed Google HTTPS endpoints, do not follow redirects and have bounded time and response size. Exceptions are sanitised before they can reach general request logging.

Personal list caches include the client identity and are short-lived and bounded. Disconnect removes that phone's token record, pending sign-in and cached private lists. It does not revoke the whole Google project grant, which would also affect other devices using that same Google account. **Manage Google access** opens Google's own connections page, where the owner can revoke the broader grant. A later revoked/expired grant clears the local credentials and offers reconnect. Existing remote pairing and other phones' Google connections remain intact.

## API and verification

The account routes are `/api/youtube/account`, `/api/youtube/link`, `/api/youtube/link/cancel`, `/api/youtube/disconnect` and `/api/youtube/library`. The private provisioning route is `/api/youtube/setup`. Library kinds are `subscriptions`, `playlists`, `liked`, `playlist` and `channel`. IDs and pagination are validated before sending a request to Google. Advertising/page parsing in public search is unchanged.

Tests are in `YouTubeAccountsTest.kt`, `YouTubeAccountStorageTest.kt` and `tests/youtube-accounts-phone-check.js`. Account tests use disposable fake credentials and an independent transport fixture. Browser checks use two separate local browser contexts with synthetic accounts. They exercise isolation, device-code cancellation, personal-list navigation and pagination, clear/error/retry states, reconnect notices, existing video dispatch, and mobile layout. These establish implementation behaviour, not acceptance of two real Google accounts.

Earlier working release artifacts and pairings are retained. Package and install the current build through the existing guarded installer only after tests pass and the custom Android home is confirmed in front. Verify that the live phone assets match the build and that Stremio history and pairings remain. Do not uninstall or clear data as an update shortcut.

## Installed release and checks

- Android APK: `dist/couch-tv-sitting-room-youtube-accounts.apk`, SHA-256 `fe3eeec3f6946bb2c490e69da056df2522f2853e6d0c91e307404dd8a326736c`. Release build, lint, signing and all 113 Android tests passed. Eight account-browser checks, six existing YouTube checks and eight hybrid-control checks passed. Browser account checks use synthetic accounts.
- The guarded installer verified the transfer checksum, installed with `pm install -r`, and removed the temporary APK. Evidence: `dist/apple-tv-research/youtube-accounts-install.json`.
- After installation: six paired clients, home watch bound, Stremio linked, eight history items with six currently in progress. The TV returned to the Apple TV input. Public phone assets `index-B3QU8uZX.js` and `index-BnQTDI7p.css` match the build byte for byte. Evidence: `youtube-accounts-live-check.json`.
- Native iPhone and tvOS apps are unchanged. Each iPhone receives this interface from the TV when reopened. No Mac service is required.
- Google client provisioning is confirmed. The real Google device-code request and cancellation passed on the setup Mac’s independent client, without authorising a Google user. The final publishing status and authorisation on both physical phones still require confirmation. This release is not evidence of actual Google account access.

The subsequent installed release is `dist/couch-tv-sitting-room-youtube-home.apk`, SHA-256 `1f36bd8e40adc361a3b1646e13cd0da6649887201d3254af0bfcc2fd13997f1d`. Live assets `index-CCBvShEL.js`, `index-BnQTDI7p.css` and the bundled privacy page match. Its native companion is `dist/CouchTV-Remote-YouTube-Home.ipa`, SHA-256 `b047dfdc03d6cb1e3d3d45d72df8e8888622852aa503e27dc2f3dd73aa78dd37`. See the parent SETUP-STATUS.md for the current acceptance gates; the earlier registration/deployment bullets above describe that preceding build.

## Primary references

- [Google's TV device OAuth flow](https://developers.google.com/identity/protocols/oauth2/limited-input-device)
- [Subscriptions API](https://developers.google.com/youtube/data/reference/rest/v3/subscriptions/list)
- [Owned playlists API](https://developers.google.com/youtube/data/reference/rest/v3/playlists/list)
- [Playlist items and Watch History/Watch Later limits](https://developers.google.com/youtube/data/reference/rest/v3/playlistItems/list)
- [Liked videos API](https://developers.google.com/youtube/data/reference/rest/v3/videos/list)
- [Google refresh-token expiration](https://developers.google.com/identity/protocols/oauth2#expiration)
