//! Native Stremio environment. Account persistence and the Swift ABI are separate stages.
#[cfg(all(feature = "account-fixture", target_os = "tvos", not(target_abi = "sim")))]
compile_error!("Account fixtures must not be included in physical Apple TV builds");

pub mod environment;

pub mod session;

pub mod ffi;

pub mod persistence;

pub mod catalog;

pub mod details;

pub mod sources;

pub mod playback_source;
pub mod subtitles;

mod restore;

#[cfg(test)]
mod auth_tests;

pub mod library;

mod addon_transport;

pub mod player;

mod checkpoint;

mod sync_journal;

mod library_update;

mod sync_upload;

mod sync_scheduler;

#[cfg(test)]
mod sync_session_tests;

mod reauth;
