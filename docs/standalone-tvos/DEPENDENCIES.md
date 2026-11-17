# Standalone tvOS dependency pins

All new standalone dependencies must use exact versions or full Git revisions. Keep lockfiles committed, build Rust with `--locked`, and make dependency upgrades explicit reviewed changes. Do not use branch heads, floating tags or automatic latest-version resolution.

- Rust toolchain: 1.100.0 (`native/cue-torrent/rust-toolchain.toml`).
- rqbit and its bencode crate: revision `f23bacdddcc5318caa813418b8eb1b1d7307faaa`.
- Registry crates: exact direct versions in Cargo.toml; complete resolved graph and checksums in Cargo.lock. The dualstack socket patch remains vendored with its notices.
- VLCKit candidate: revision `3d0f0fa7031eab1a67c817c98a495e1c68fc998c` in apple-tv/project.yml. That revision declares binary VLCKit-4.0-20261109-1916.zip with SHA-256 `1fec7232f88e706f44d9a015133e08d742166eb2666cb44915c9f570bf339463`. Its LGPL 2.1 license is retained in native/licenses/VLCKit-LGPL-2.1.txt. This is an evaluation dependency; production selection and complete binary component/source notices remain Phase 2/7 work.

The inherited Python bridge and JavaScript/Android applications are separate from the standalone runtime. Their existing dependency policies are not claimed to be fully pinned by this work: Python includes floating cryptography/tzdata declarations and JavaScript uses version ranges with checked-in npm lockfiles.
