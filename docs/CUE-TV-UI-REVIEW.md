# Cue TV remote and internal browsing review

16 November 2026. This review covers the requested remote controls, internal Stremio flow, service rows and the prepared YouTube refinement.

| Before | After | Why |
| --- | --- | --- |
| Holding the centre pad selected its text | A deliberate hold sends one held Select; swipes, cancellation and release cannot add a tap | Context-menu input behaves like the TV remote. Text selection remains available outside controls. |
| Apple TV had no separate system TV control | Apple-only screen button, tap for TV and hold for Control Centre | Cue Home and Apple's system controls have distinct purposes. |
| A Stremio title could leave Cue TV for another app | Cue's own seasons, episodes, private source choices and full-screen player | Back can return to the existing selection. YouTube keeps its own external app. |
| Only general catalogue shelves | Separate service-labelled Trending, Popular and New rows | These rankings remain distinct, including during upstream failures. |
| Website URL observations could immediately open YouTube on the television | A selected video requires Play on TV, with cancel and retry | Browsing does not itself start household playback. |

## Fresh review and checks

The settled phone remote, phone Watch view, service shelves, TV season picker and source chooser were reviewed. The focused TV episode uses a white surface with dark type; unavailable episodes stay disabled. The remote's pad remains the largest touch area, the TV button has an accessible hold description, and Android retains its existing navigation layout. Existing short focus and screen transitions remain; reduced-motion behaviour is preserved. Captures wait for those transitions to settle, rather than treating an intermediate paint as the final UI.

Eight browser gesture checks and the actual native WebKit hold test pass. Browser checks cover separate service rankings, no playback from browsing alone, future episodes disabled, the exact chosen episode/source, Back retaining the season and no JavaScript page errors. Native tvOS playback tests return to the same source chooser after real generated-sample playback. Real Android playback is confirmed separately; physical Apple handoff is still unresolved.

Local evidence is intentionally excluded from Git: `output/apple-tv/cue-tv-held-controls.png`, `cue-tv-phone-service-shelves.png`, `cue-tv-android-episodes.png`, `cue-tv-android-sources.png`, the native test attachments and `dist/apple-tv-research/cue-tv-final-browse-settled-check.log`.

The personalised YouTube Home is still YouTube's own website in a separate per-feed WebKit session. The compact stylesheet and preview suppression are installed in iPhone build 5, but require a fresh check against actual signed-in recommendations. The website can change its markup; generic fixture tests do not establish every personalised layout. No household recommendation data or website cookies are exported.
