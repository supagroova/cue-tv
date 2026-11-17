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
        UpdateWithCtx,
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
            model: Player {
                meta_item: Some(meta),
                ..Default::default()
            },
        };
        playback.update(Action::Load(ActionLoad::Player(Box::new(selected))), ctx);
        let initial = playback.progress()?;
        // Establish the selected video without treating an existing resume offset as viewing.
        playback.update(
            Action::Player(ActionPlayer::TimeChanged {
                time: initial.time,
                duration: initial.duration,
                device: "tvos".into(),
            }),
            ctx,
        );
        playback.progress()?;
        Ok(playback)
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
                if time < self.progress()?.time {
                    return Err("playback_requires_seek");
                }
                ActionPlayer::TimeChanged {
                    time,
                    duration,
                    device: "tvos".into(),
                }
            }
            PlaybackEvent::Paused(paused) => ActionPlayer::PausedChanged { paused },
        };
        self.update(Action::Player(action), ctx);
        self.progress()
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
