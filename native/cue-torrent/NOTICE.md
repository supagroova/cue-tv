# Native dependencies

librqbit is pinned to `f23bacdddcc5318caa813418b8eb1b1d7307faaa` from https://github.com/ikatson/rqbit (Apache-2.0). Cargo.lock records transitive resolution.

`vendor/dualstack` is librqbit-dualstack-sockets 0.7.0 from crates.io, https://github.com/ikatson/librqbit-dualstack-sockets, Apache-2.0; its LICENSE is retained. The only source modification replaces the two `target_os = "macos"` conditions in `src/bind_device.rs` with `target_vendor = "apple"`, so tvOS uses the existing Apple interface-binding implementation instead of the Linux API. No UPnP port forwarding is enabled by Cue TV.

This is a controlled-peer feasibility implementation. Public tracker/DHT discovery, FFI integration, storage limits and playback qualification are not yet established by compiling it.
