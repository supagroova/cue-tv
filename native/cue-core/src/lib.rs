//! Native Stremio environment. Account persistence and the Swift ABI are separate stages.
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
