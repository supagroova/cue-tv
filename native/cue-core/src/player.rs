//! Core Player accounting, owned by one native session. Content retrieval and
//! durable checkpoints are host responsibilities, not implicit Player effects.
use crate::environment::NativeEnv;
use stremio_core::{
    models::{
        common::Loadable,
        ctx::Ctx,
        meta_details::MetaDetails,
        player::{Player, Selected},
    },
    runtime::{
        Env, UpdateWithCtx,
        msg::{Action, ActionLoad, ActionPlayer, Msg},
    },
};

#[derive(Clone, Debug)]
pub enum PlaybackEvent {
    Seek { time: u64, duration: u64 },
    TimeChanged { time: u64, duration: u64 },
    Paused(bool),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub time: u64,
    pub duration: u64,
    pub time_watched: u64,
    pub watched: bool,
}

pub(crate) struct Playback {
    pub token: u64,
    model: Player,
    manual_watched: Option<bool>,
}
impl Playback {
    pub fn new(
        token: u64,
        selected: Selected,
        details: &MetaDetails,
        ctx: &Ctx,
    ) -> Result<Self, &'static str> {
        let request = selected.meta_request.as_ref().ok_or("invalid_playback")?;
        if ctx
            .library
            .items
            .get(&request.path.id)
            .is_some_and(|item| item.r#type != request.path.r#type)
        {
            return Err("library_identity_conflict");
        }
        let meta = details
            .meta_items
            .iter()
            .find(|page| {
                &page.request == request && matches!(page.content, Some(Loadable::Ready(_)))
            })
            .ok_or("invalid_playback")?
            .clone();
        let mut playback = Self {
            token,
            manual_watched: None,
            model: Player {
                meta_item: Some(meta),
                ..Default::default()
            },
        };
        playback.update(Action::Load(ActionLoad::Player(Box::new(selected))), ctx);
        let initial = playback.progress()?;
        // Establish the selected video without treating an existing resume offset as viewing.
        // Initialization establishes identity, not newly viewed content.
        if let Some(item) = &mut playback.model.library_item {
            item.state.flagged_watched = 1;
        }
        playback.update(
            Action::Player(ActionPlayer::TimeChanged {
                time: initial.time,
                duration: initial.duration,
                device: "tvos".into(),
            }),
            ctx,
        );
        // An explicit new Play permits automatic completion again.
        if let Some(item) = &mut playback.model.library_item {
            item.state.flagged_watched = 0;
        }
        playback.progress()?;
        Ok(playback)
    }
    pub fn override_watched(
        &mut self,
        kind: &str,
        meta_id: &str,
        video_id: Option<&str>,
        watched: bool,
    ) {
        let Some(selected) = self.model.selected.as_ref() else {
            return;
        };
        let Some(meta) = selected.meta_request.as_ref() else {
            return;
        };
        let Some(stream) = selected.stream_request.as_ref() else {
            return;
        };
        if meta.path.id != meta_id || meta.path.r#type != kind {
            return;
        }
        if (kind == "movie" && video_id.is_some())
            || (kind == "series" && video_id != Some(stream.path.id.as_str()))
        {
            return;
        }
        let Some(item) = self.model.library_item.as_mut() else {
            return;
        };
        if kind == "movie" {
            let position = item.state.time_offset;
            if item.watched() != watched {
                item.mark_as_watched::<NativeEnv>(watched);
            }
            // Keep the decoder accounting baseline; the details action separately
            // applies Core's manual resume-state semantics to the stored library.
            item.state.time_offset = position;
        } else if kind == "series" {
            let Some(meta) = self
                .model
                .meta_item
                .as_ref()
                .and_then(|page| page.content.as_ref())
                .and_then(|content| content.ready())
            else {
                return;
            };
            let Some(video) = meta.videos.iter().find(|video| video.id == stream.path.id) else {
                return;
            };
            let bits = item.state.watched_bitfield(&meta.videos);
            item.mark_video_as_watched::<NativeEnv>(&bits, video, watched);
            self.model.watched = Some(item.state.watched_bitfield(&meta.videos));
        } else {
            return;
        }
        self.manual_watched = Some(watched);
        item.state.flagged_watched = 1;
    }
    fn update(&mut self, action: Action, ctx: &Ctx) {
        // Deliberately do not execute Core's returned effects here. Metadata is already
        // loaded, explicit source/subtitle discovery owns networking, and the host must
        // checkpoint the resulting Core LibraryItem separately from remote sync. This
        // also prevents implicit rating/intro-outro/stream-history and analytics traffic.
        drop(<Player as UpdateWithCtx<NativeEnv>>::update(
            &mut self.model,
            &Msg::Action(action),
            ctx,
        ));
    }
    pub fn event(&mut self, event: PlaybackEvent, ctx: &Ctx) -> Result<Progress, &'static str> {
        let action = match event {
            PlaybackEvent::Seek { time, duration } => ActionPlayer::Seek {
                time: if duration > 0 {
                    time.min(duration)
                } else {
                    time
                },
                duration,
                device: "tvos".into(),
            },
            PlaybackEvent::TimeChanged { time, duration } => {
                if self.model.paused == Some(true) {
                    return self.progress();
                }
                let time = if duration > 0 {
                    time.min(duration)
                } else {
                    time
                };
                let current = self.progress()?;
                if time < current.time {
                    return Err("playback_requires_seek");
                }
                if duration != current.duration {
                    // Core updates duration in TimeChanged only when position advances.
                    // Synchronize it without adding viewing time before threshold checks.
                    self.update(
                        Action::Player(ActionPlayer::Seek {
                            time: current.time,
                            duration,
                            device: "tvos".into(),
                        }),
                        ctx,
                    );
                }
                ActionPlayer::TimeChanged {
                    time,
                    duration,
                    device: "tvos".into(),
                }
            }
            PlaybackEvent::Paused(paused) => ActionPlayer::PausedChanged { paused },
        };
        if self.manual_watched.is_some() {
            if let Some(item) = &mut self.model.library_item {
                item.state.flagged_watched = 1;
            }
        }
        self.update(Action::Player(action), ctx);
        self.progress()
    }
    /// Project only this owner's playback state onto the current Core record.
    /// Admission and durable acknowledgement are separate session responsibilities.
    pub fn checkpoint_item(
        &self,
        ctx: &Ctx,
    ) -> Result<stremio_core::types::library::LibraryItem, &'static str> {
        let played = self.model.library_item.as_ref().ok_or("invalid_playback")?;
        let latest = ctx.library.items.get(&played.id);
        if latest.is_some_and(|item| item.r#type != played.r#type) {
            return Err("library_identity_conflict");
        }
        let mut item = latest.unwrap_or(played).clone();
        // Keep the latest metadata, membership and notification preference.
        // Explicit completion can clear/advance resume; preserve that decision
        // for this owner instead of resurrecting its decoder's older position.
        if self.manual_watched != Some(true) || latest.is_none() {
            item.state = played.state.clone();
            if let Some(latest) = latest {
                item.state.no_notif = latest.state.no_notif;
            }
        }
        if played.r#type == "series" {
            let meta = self
                .model
                .meta_item
                .as_ref()
                .and_then(|page| page.content.as_ref())
                .and_then(|content| content.ready())
                .ok_or("invalid_playback")?;
            let video = self
                .model
                .selected
                .as_ref()
                .and_then(|selected| selected.stream_request.as_ref())
                .ok_or("invalid_playback")?;
            let mut bits = latest
                .unwrap_or(played)
                .state
                .watched_bitfield(&meta.videos);
            bits.set_video(&video.path.id, self.progress()?.watched);
            item.state.watched = Some(bits.into());
        }
        item.mtime = item.mtime.max(NativeEnv::now());
        Ok(item)
    }

    fn progress(&self) -> Result<Progress, &'static str> {
        let item = self.model.library_item.as_ref().ok_or("invalid_playback")?;
        let video = self
            .model
            .selected
            .as_ref()
            .and_then(|selected| selected.stream_request.as_ref())
            .ok_or("invalid_playback")?;
        let watched = if item.r#type == "series" {
            let meta = self
                .model
                .meta_item
                .as_ref()
                .and_then(|page| page.content.as_ref())
                .and_then(|content| content.ready())
                .ok_or("invalid_playback")?;
            item.state
                .watched_bitfield(&meta.videos)
                .get_video(&video.path.id)
        } else {
            item.watched()
        };
        Ok(Progress {
            time: item.state.time_offset,
            duration: item.state.duration,
            time_watched: item.state.time_watched,
            watched,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use stremio_core::{models::common::ResourceLoadable, types::profile::Profile};

    #[tokio::test]
    async fn movie_checkpoint_preserves_manual_completion_and_unwatched_progress() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let mut ctx = crate::restore::context(Profile::default()).await.unwrap();
            let selected: Selected = serde_json::from_value(serde_json::json!({
                "stream":{"infoHash":"0123456789abcdef0123456789abcdef01234567","fileIdx":0},
                "metaRequest":{"base":"https://owned.test/manifest.json","path":{"resource":"meta","type":"movie","id":"film","extra":[]}},
                "streamRequest":{"base":"https://owned.test/manifest.json","path":{"resource":"stream","type":"movie","id":"film","extra":[]}},
                "subtitlesPath":null
            })).unwrap();
            let details = MetaDetails {
                meta_items: vec![ResourceLoadable {
                    request: selected.meta_request.clone().unwrap(),
                    content: Some(Loadable::Ready(serde_json::from_value(serde_json::json!({
                        "id":"film","type":"movie","name":"Film"
                    })).unwrap())),
                }],
                ..Default::default()
            };
            let mut owner = Playback::new(1, selected, &details, &ctx).unwrap();
            owner.event(PlaybackEvent::TimeChanged { time: 400, duration: 1000 }, &ctx).unwrap();
            let initial = owner.checkpoint_item(&ctx).unwrap();
            assert_eq!(initial.state.time_offset, 400);
            assert_eq!(initial.state.time_watched, 400);
            assert!(!initial.watched());
            ctx.library.items.insert("film".into(), initial);
            owner.override_watched("movie", "film", None, true);
            let current = ctx.library.items.get_mut("film").unwrap();
            current.mark_as_watched::<NativeEnv>(true);
            current.mtime = NativeEnv::now() + chrono::Duration::days(1);
            let future_mtime = current.mtime;
            owner.event(PlaybackEvent::TimeChanged { time: 800, duration: 1000 }, &ctx).unwrap();
            let marked = owner.checkpoint_item(&ctx).unwrap();
            assert_eq!(marked.state.time_offset, 0);
            assert_eq!(marked.state.times_watched, 1);
            assert_eq!(marked.mtime, future_mtime);
            ctx.library.items.insert("film".into(), marked);
            owner.override_watched("movie", "film", None, false);
            ctx.library.items.get_mut("film").unwrap().mark_as_watched::<NativeEnv>(false);
            owner.event(PlaybackEvent::TimeChanged { time: 900, duration: 1000 }, &ctx).unwrap();
            let unmarked = owner.checkpoint_item(&ctx).unwrap();
            assert_eq!(unmarked.state.time_offset, 900);
            assert_eq!(unmarked.state.time_watched, 900);
            assert!(!unmarked.watched());
        }).await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn episode_override_is_identity_scoped_and_new_play_reenables_completion() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let mut ctx = crate::restore::context(Profile::default()).await.unwrap();
            let selected: Selected = serde_json::from_value(serde_json::json!({
                "stream":{"infoHash":"0123456789abcdef0123456789abcdef01234567","fileIdx":0},
                "metaRequest":{"base":"https://owned.test/manifest.json","path":{"resource":"meta","type":"series","id":"owned","extra":[]}},
                "streamRequest":{"base":"https://owned.test/manifest.json","path":{"resource":"stream","type":"series","id":"owned:1:1","extra":[]}},
                "subtitlesPath":null
            })).unwrap();
            let details = MetaDetails {
                meta_items: vec![ResourceLoadable {
                    request: selected.meta_request.clone().unwrap(),
                    content: Some(Loadable::Ready(serde_json::from_value(serde_json::json!({
                        "id":"owned","type":"series","name":"Owned",
                        "videos":[{"id":"owned:1:1","title":"First","season":1,"episode":1},
                                  {"id":"owned:1:2","title":"Second","season":1,"episode":2}]
                    })).unwrap())),
                }],
                ..Default::default()
            };
            let mut owner = Playback::new(1, selected.clone(), &details, &ctx).unwrap();
            assert!(owner.event(PlaybackEvent::TimeChanged { time: 800, duration: 1000 }, &ctx).unwrap().watched);
            owner.override_watched("series", "owned", Some("owned:1:2"), false);
            assert!(owner.event(PlaybackEvent::TimeChanged { time: 801, duration: 1000 }, &ctx).unwrap().watched);
            owner.override_watched("series", "owned", Some("owned:1:1"), false);
            assert!(!owner.event(PlaybackEvent::TimeChanged { time: 900, duration: 1000 }, &ctx).unwrap().watched);
            owner.override_watched("series", "another-title", Some("owned:1:1"), true);
            assert!(!owner.event(PlaybackEvent::TimeChanged { time: 901, duration: 1000 }, &ctx).unwrap().watched);
            let item = owner.model.library_item.as_ref().unwrap().clone();
            ctx.library.items.insert(item.id.clone(), item);
            let mut fresh = Playback::new(2, selected, &details, &ctx).unwrap();
            assert!(!fresh.progress().unwrap().watched, "Load itself must not undo the manual choice");
            let unknown = fresh.event(PlaybackEvent::TimeChanged { time: 901, duration: 0 }, &ctx).unwrap();
            assert_eq!(unknown.duration, 0);
            assert!(!unknown.watched, "An unchanged clock with unknown duration must not use the previous duration");
            assert!(fresh.event(PlaybackEvent::TimeChanged { time: 902, duration: 1000 }, &ctx).unwrap().watched);
            let meta = details.meta_items[0].content.as_ref().unwrap().ready().unwrap();
            let bits = fresh.model.library_item.as_ref().unwrap().state.watched_bitfield(&meta.videos);
            assert!(bits.get_video("owned:1:1"));
            assert!(!bits.get_video("owned:1:2"));
            // A checkpoint must merge into the latest library record, not the
            // player's older copy of membership, metadata or other episode bits.
            let current = ctx.library.items.get_mut("owned").unwrap();
            current.removed = true;
            current.temp = false;
            current.name = "Updated name".into();
            current.state.no_notif = true;
            let current_bits = current.state.watched_bitfield(&meta.videos);
            current.mark_video_as_watched::<NativeEnv>(&current_bits, &meta.videos[1], true);
            let checkpoint = fresh.checkpoint_item(&ctx).unwrap();
            assert!(checkpoint.removed);
            assert!(!checkpoint.temp);
            assert_eq!(checkpoint.name, "Updated name");
            assert!(checkpoint.state.no_notif);
            assert_eq!(checkpoint.state.time_offset, 902);
            let merged = checkpoint.state.watched_bitfield(&meta.videos);
            assert!(merged.get_video("owned:1:1"));
            assert!(merged.get_video("owned:1:2"));
            assert!(!ctx.library.items["owned"].state.watched_bitfield(&meta.videos).get_video("owned:1:1"), "Projection must not mutate Core before admission");

            // A manual completed episode may move resume to the next episode.
            // A late checkpoint must not restore the playing episode's pointer.
            fresh.override_watched("series", "owned", Some("owned:1:1"), true);
            let current = ctx.library.items.get_mut("owned").unwrap();
            let current_bits = current.state.watched_bitfield(&meta.videos);
            current.mark_video_as_watched::<NativeEnv>(&current_bits, &meta.videos[0], true);
            current.advance_to_video("owned:1:2");
            fresh.event(PlaybackEvent::TimeChanged { time: 950, duration: 1000 }, &ctx).unwrap();
            let checkpoint = fresh.checkpoint_item(&ctx).unwrap();
            assert_eq!(checkpoint.state.video_id.as_deref(), Some("owned:1:2"));
            assert_eq!(checkpoint.state.time_offset, 1);
            assert_eq!(checkpoint.state.time_watched, 0);
            ctx.library.items.get_mut("owned").unwrap().r#type = "movie".into();
            assert_eq!(fresh.checkpoint_item(&ctx).unwrap_err(), "library_identity_conflict");
        }).await;
        env.shutdown().await;
    }
}
