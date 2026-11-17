# Phase 4 — standalone Stremio journey

Status: in progress; depends on unfinished Phase 3. This is not an exit-gate claim.

The first slice adds `TorrentPlaybackView`, accepting a prepared Core source identity. It offers explicit original-index file selection with pagination, in-app VLC rendering, pause/resume, relative seeking, audio selection and subtitle cycling/off. Decoder shutdown runs before native/cache retirement; callback identity rejects retired generations. Backgrounding and Close stop the source.

Guest entry and basic catalog/details/episode/source navigation are now connected. Account sign-in, search, add-on subtitles, focus restoration and full journey validation remain outstanding. Physical playback is unverified.

Verification: unsigned device build passes (`phase-4-ui/a59869c0a1774927885d58cb079db89b`); simulator unit/integration suite passes (`phase-4-ui/249b0a3859314138a3539b685a7f516d`). Those establish compilation and existing integration contracts, not visual/focus acceptance of the new screen. The existing lab UI suite passed on the shared renderer before the controller extraction; new screen UI coverage remains required.

## Guest entry and native browsing

The default app entry now creates the persistent native guest Core service. Basic catalog/title/detail/episode/source windows and add-on preview/install use revision-checked Core commands; selecting a supported torrent opens the source playback screen. Android pairing/remote observation are no longer started by the app entry point. Unsupported non-torrent sources are explicitly disabled in this initial slice; compatible HTTPS remains an outstanding requirement.

The model integration test proves real Core startup, snapshot updates, a language action and joined shutdown. Simulator suite passes (`phase-4-ui/b413d2ac2da049d5b55e7f13bf4f4753`). Visual inspection of the actual simulator guest entry caught white-on-white inherited tint; removing the tint restored readable catalog buttons. Screenshot evidence remains local at `.agent-work/standalone-guest-fixed.png`. Full navigation/focus and player-screen UI tests are still required. Catalog labels also include media kind to distinguish same-name film/series catalogs.

Latest unsigned device build passes: `phase-4-ui/9eda0cfd6157432ea364fddb7e77f16e`.
