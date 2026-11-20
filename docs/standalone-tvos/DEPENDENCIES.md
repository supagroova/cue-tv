# Standalone tvOS dependency pins

All new standalone dependencies must use exact versions or full Git revisions. Keep lockfiles committed, build Rust with `--locked`, and make dependency upgrades explicit reviewed changes. Do not use branch heads, floating tags or automatic latest-version resolution.

- Rust toolchain: 1.100.0 (`native/cue-torrent/rust-toolchain.toml`).
- rqbit and its bencode crate: revision `f23bacdddcc5318caa813418b8eb1b1d7307faaa`.
- Registry crates: exact direct versions in Cargo.toml; complete resolved graph and checksums in Cargo.lock. The dualstack socket patch remains vendored with its notices.
- VLCKit candidate: revision `3d0f0fa7031eab1a67c817c98a495e1c68fc998c` in apple-tv/project.yml. That revision declares binary VLCKit-4.0-20261109-1916.zip with SHA-256 `1fec7232f88e706f44d9a015133e08d742166eb2666cb44915c9f570bf339463`. Its LGPL 2.1 license is retained in native/licenses/VLCKit-LGPL-2.1.txt. Selected for continued implementation in decisions/002-player-selection.md; complete binary component/source notices remain Phase 2/7 work.

The inherited Python bridge and JavaScript/Android applications are separate from the standalone runtime. Their existing dependency policies are not claimed to be fully pinned by this work: Python includes floating cryptography/tzdata declarations and JavaScript uses version ranges with checked-in npm lockfiles.

The manual MPV comparison probe separately pins MPVKit to `d9194243bdc68b87d8989376ab52aee7da6c8e0b`. It is not linked into Cue TV. Its pinned package declares exact binary artifact checksums; see probes/mpv-player/README.md.

## Stremio Core environment

- Core: shallow Git submodule `native/stremio-core` at `2c89236b47dfcd5e844c1dfae5737164cb616fbf`, consumed as a path dependency by `native/cue-core`. The parent Git link pins the source; `scripts/native/build-core.sh` verifies it.
- Core's local-search dependency: Cargo.lock revision `74fefe1da2fa17d2b4ef7e3e629da00f3946ee72`.
- Direct crate versions and the complete transitive graph are pinned in `native/cue-core/Cargo.toml` and Cargo.lock. Builds/tests use `--locked`.
- MIT notices: `native/licenses/Stremio-Core-MIT.txt` and `native/licenses/Stremio-Local-Search-MIT.txt`.
- CI cache action: `actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830` (resolved v4 commit).


## Candidate VLCKit provenance audit (20 November 2026)

The [candidate record](evidence/vlckit-provenance-2026-11-20.json) ties the actual
packaged tvOS framework hash to the pinned wrapper revision and declared binary
archive checksum. The wrapper's [build script](https://github.com/videolan/vlckit/blob/3d0f0fa7031eab1a67c817c98a495e1c68fc998c/compileAndBuildVLCKit.sh)
starts at libVLC base [f7caf7922478f6cb7f1654b813d0c462d3e86f4a](https://github.com/videolan/vlc/commit/f7caf7922478f6cb7f1654b813d0c462d3e86f4a)
and applies 12 retained patches. Its exact wrapper source archive is retained locally
under `.agent-work/release-candidate-001/vlckit-build-materials` and hashed in the
record; this includes its build script, patches and COPYING file.

The packaged framework contains version string `4.0.0-dev-39728-gd107c01fbc`.
The different changeset may reflect patched sources; it is not proof of the
published binary's exact source/build configuration. Complete contrib component
notices/source materials and an independent rebuild remain open. The generic LGPL
text alone is not being presented as complete combined-binary notices.


Offline Cargo metadata with `--locked --filter-platform aarch64-apple-tvos`
currently reaches 209 Core and 275 torrent non-development packages when build
edges are included. These are dependency graphs, not proof that every package is
linked into the final binary. Metadata is retained locally in
`.agent-work/core-license-metadata.json` and `.agent-work/torrent-license-metadata.json`.
Core's undeclared metadata entries are cue-core, localsearch, stremio-core,
stremio-derive and stremio-watched-bitfield; the torrent graph's is cue-torrent.
Existing source notices must be matched explicitly before resolving these gaps;
missing Cargo fields do not imply absent source licenses.


The six undeclared metadata entries above now have explicit source-notice mappings
in [PROVENANCE.json](../../native/licenses/PROVENANCE.json): the repository MIT
notice covers Cue's crates, the pinned Stremio root notice covers its workspace,
and localsearch's retained notice matches its pinned source byte-for-byte. The
Stremio retained file was normalized to the source's final blank line, without
changing its text. rqbit's root Apache notice is now retained and mapped to its
workspace crates. This records source correspondence, not completed combined-app
license qualification.

The union of both non-dev/build graphs contains 327 packages. Initial top-level and
explicit-file discovery still needs additional source notice evidence for fxhash
0.2.1, stremio-official-addons 2.1.2, assert_cfg 0.1.0 and governor 0.10.4. fxhash
contains a license/copyright header in lib.rs; Stremio's README references a license
file absent from its crate archive; assert_cfg/governor retain exact VCS revisions
in their crate archives. Use those identities when retrieving missing materials.


Follow-up source retrieval retained the omitted assert_cfg/governor notices at the
crate-recorded VCS revisions, and the Stremio official-addons notice at the resolved
v2.1.2 commit. Stremio's lib.rs Git blob matches the published crate. fxhash's lib.rs
also matches commit 537570af68de22d2f4bbdd5bee5d6e07268e6f42; its copyright/license
header is retained alongside the full Apache 2.0 alternative from Apache's official
license URL. rqbit's source file is an attribution notice, so the full Apache text
is retained separately as well. All retained files have checksums in PROVENANCE.json.
Collecting the remaining graph notices and matching VLCKit bundled components are
still required; this targeted source recovery does not complete that inventory.
