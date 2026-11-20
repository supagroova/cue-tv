//! Owns real Core models and effects; published snapshots contain no account secrets.
use crate::environment::{Environment, NativeEnv};
use futures::FutureExt;
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};
use stremio_core::{
    constants::PROFILE_STORAGE_KEY,
    models::{
        addon_details::{AddonDetails, Selected as AddonSelected},
        common::Loadable,
        ctx::{Ctx, CtxStatus},
        meta_details::{MetaDetails, Selected as DetailsSelected},
    },
    runtime::{
        Effect, EffectFuture, Effects, Env, EnvError, UpdateWithCtx,
        msg::{
            Action, ActionCatalogWithFilters, ActionCtx, ActionLoad, ActionMetaDetails, Event,
            Internal, Msg,
        },
    },
    types::{api::AuthRequest, profile::Profile},
};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Clone, Debug, Serialize)]
pub struct AddonSnapshot {
    pub status: &'static str,
    pub installed: bool,
    pub id: Option<String>,
    pub name: Option<String>,
}
pub(crate) fn validate_addon_url(url: &str) -> Result<AddonSelected, &'static str> {
    if url.len() > 2048 {
        return Err("invalid_addon_url");
    }
    let selected: AddonSelected = serde_json::from_value(serde_json::json!({"transportUrl":url}))
        .map_err(|_| "invalid_addon_url")?;
    let url = &selected.transport_url;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url.path().ends_with("/manifest.json")
    {
        return Err("invalid_addon_url");
    }
    Ok(selected)
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SyncSnapshot {
    pub pending_titles: usize,
    pub state: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub sync: Option<SyncSnapshot>,
    pub authenticated: bool,
    pub revision: u64,
    pub stopped: bool,
    pub interface_language: String,
    pub addon_count: usize,
    pub catalogs: Vec<crate::catalog::Choice>,
    pub catalog_total: usize,
    pub catalog_choices_offset: usize,
    pub catalog: Option<crate::catalog::View>,
    pub addon: Option<AddonSnapshot>,
    pub details: Option<crate::details::View>,
    pub sources: Option<crate::sources::View>,
    pub details_in_library: Option<bool>,
    pub library: crate::library::View,
    pub pending_effects: usize,
    pub error: Option<&'static str>,
}
type SubtitlePlan = (u64, Vec<stremio_core::types::addon::ResourceRequest>);
enum Command {
    RetrySync(oneshot::Sender<Result<(), &'static str>>),
    #[cfg(test)]
    TestLibraryItem(
        stremio_core::types::library::LibraryItem,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    SyncPrepared(
        u64,
        stremio_core::types::profile::Auth,
        Result<Option<crate::sync_journal::Upload>, ()>,
    ),
    SyncUploaded(
        u64,
        crate::sync_journal::Upload,
        Result<(), crate::sync_upload::Error>,
    ),
    Checkpoint(u64, bool, oneshot::Sender<Result<(), &'static str>>),
    CheckpointDone(
        Result<(), &'static str>,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    StartPlayback(u64, oneshot::Sender<Result<(), &'static str>>),
    PlaybackEvent(
        u64,
        crate::player::PlaybackEvent,
        oneshot::Sender<Result<crate::player::Progress, &'static str>>,
    ),
    RetirePlayback(u64, oneshot::Sender<Result<(), &'static str>>),
    ClaimPlayback(
        u64,
        oneshot::Sender<Result<stremio_core::models::player::Selected, &'static str>>,
    ),
    DetailsWatched(
        Option<String>,
        bool,
        u64,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    DetailsLibrary(bool, u64, oneshot::Sender<Result<(), &'static str>>),
    SubtitlePlan {
        meta_id: String,
        video_id: String,
        kind: String,
        generation: Option<u64>,
        reply: oneshot::Sender<Result<SubtitlePlan, &'static str>>,
    },
    Logout(oneshot::Sender<Result<(), &'static str>>),
    Login(AuthRequest, oneshot::Sender<Result<(), &'static str>>),
    Language(String, oneshot::Sender<Result<(), &'static str>>),
    Addon(AddonSelected, oneshot::Sender<Result<(), &'static str>>),
    Install(u64, oneshot::Sender<Result<(), &'static str>>),
    Catalog(
        usize,
        Option<String>,
        u64,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    CatalogChoices(usize, u64, oneshot::Sender<Result<(), &'static str>>),
    CatalogItems(usize, u64, oneshot::Sender<Result<(), &'static str>>),
    Details(usize, u64, bool, oneshot::Sender<Result<(), &'static str>>),
    LibraryWindow(usize, u64, oneshot::Sender<Result<(), &'static str>>),
    DetailsVideos(usize, u64, oneshot::Sender<Result<(), &'static str>>),
    DetailsSeason(Option<u32>, u64, oneshot::Sender<Result<(), &'static str>>),
    Sources(
        Option<usize>,
        u64,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    SourceWindow(usize, u64, oneshot::Sender<Result<(), &'static str>>),
    PrepareTorrent(
        usize,
        u64,
        oneshot::Sender<Result<crate::playback_source::PreparedTorrent, &'static str>>,
    ),
    PrepareHttps(
        usize,
        u64,
        oneshot::Sender<Result<crate::playback_source::PreparedHttps, &'static str>>,
    ),
    Effect(Msg, u64, u64, u64, u64),
}
pub struct CoreSession {
    environment: Arc<Environment>,
    commands: mpsc::Sender<Command>,
    state: watch::Receiver<Snapshot>,
    identity: watch::Receiver<Option<String>>,
    owner: Option<tokio::task::JoinHandle<()>>,
}
impl CoreSession {
    #[cfg(test)]
    pub(crate) async fn test_library_item(
        &self,
        item: stremio_core::types::library::LibraryItem,
    ) -> Result<(), &'static str> {
        let (send, receive) = oneshot::channel();
        self.commands
            .send(Command::TestLibraryItem(item, send))
            .await
            .map_err(|_| "stopped")?;
        receive.await.map_err(|_| "stopped")?
    }
    /// Guest startup never accepts account credentials.
    pub async fn guest(environment: Arc<Environment>) -> Result<Self, EnvError> {
        Self::restore(environment, None, false).await
    }
    /// Restore only the account identity selected by protected app storage.
    /// This does not authenticate a new account or validate token freshness.
    pub async fn restore_account(
        environment: Arc<Environment>,
        expected_user: &str,
    ) -> Result<Self, EnvError> {
        if expected_user.is_empty() || expected_user.len() > 512 {
            return Err(EnvError::StorageUnavailable);
        }
        Self::restore(environment, Some(expected_user), false).await
    }
    /// A new login requires an empty dedicated account store.
    pub async fn new_account(environment: Arc<Environment>) -> Result<Self, EnvError> {
        if !environment.storage_is_empty()? {
            environment.shutdown().await;
            return Err(EnvError::StorageUnavailable);
        }
        Self::restore(environment, None, true).await
    }
    async fn restore(
        environment: Arc<Environment>,
        expected_user: Option<&str>,
        allow_login: bool,
    ) -> Result<Self, EnvError> {
        let setup = environment
            .scope(async {
                NativeEnv::migrate_storage_schema().await?;
                let profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                    .await?
                    .unwrap_or_default();
                match (&profile.auth, expected_user) {
                    (None, None) => {}
                    (Some(auth), Some(expected))
                        if auth.user.id.as_ref() == expected
                            && !auth.key.0.is_empty()
                            && auth.key.0.len() <= 4096 => {}
                    _ => return Err(EnvError::StorageUnavailable),
                }
                let ctx = crate::restore::context(profile).await?;
                let journal = NativeEnv::get_storage::<crate::sync_journal::Journal>(
                    crate::sync_journal::STORAGE_KEY,
                )
                .await?;
                Ok((ctx, journal))
            })
            .await;
        let (ctx, journal) = match setup {
            Ok(ctx) => ctx,
            Err(error) => {
                environment.shutdown().await;
                return Err(error);
            }
        };
        let (commands, inbox) = mpsc::channel(64);
        let mut initial = snapshot(
            &ctx,
            &AddonDetails::default(),
            &crate::catalog::Catalog::default(),
            &MetaDetails::default(),
            0,
            None,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
            None,
        );
        initial.sync = sync_view(
            &ctx,
            journal.as_ref(),
            &crate::sync_scheduler::Scheduler::default(),
        );
        let (identity_publish, identity) = watch::channel(account_identity(&ctx));
        let (publish, state) = watch::channel(initial);
        let env = environment.clone();
        let outgoing = commands.clone();
        let owner = tokio::spawn(async move {
            let result = std::panic::AssertUnwindSafe(env.scope(run(
                ctx,
                journal,
                inbox,
                outgoing,
                publish.clone(),
                env.clone(),
                allow_login,
                identity_publish.clone(),
            )))
            .catch_unwind()
            .await;
            if result.is_err() {
                identity_publish.send_replace(None);
                env.fail();
                let mut state = publish.borrow().clone();
                state.stopped = true;
                state.sync = None;
                state.pending_effects = 0;
                state.error = env.failure();
                publish.send_replace(state);
            }
        });
        Ok(Self {
            environment,
            commands,
            state,
            identity,
            owner: Some(owner),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.borrow().clone()
    }
    /// Explicit private identity copy for protected account selection, never UI snapshots.
    pub fn account_identity(&self) -> Option<String> {
        self.identity.borrow().clone()
    }
    pub async fn retry_sync(&self) -> Result<(), &'static str> {
        let (send, receive) = oneshot::channel();
        self.commands
            .try_send(Command::RetrySync(send))
            .map_err(|_| "account_busy")?;
        receive.await.map_err(|_| "stopped")?
    }
    pub async fn logout(&self) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Logout(reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Admit one login attempt. Callers must use a fresh account namespace, not guest storage.
    /// The reply acknowledges admission; authentication completes through snapshots.
    pub async fn login(&self, email: &str, password: &str) -> Result<(), &'static str> {
        if email.is_empty()
            || email.len() > 512
            || email.chars().any(char::is_control)
            || password.is_empty()
            || password.len() > 4096
            || password.contains('\0')
        {
            return Err("invalid_credentials");
        }
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Login(
                AuthRequest::Login {
                    email: email.to_owned(),
                    password: password.to_owned(),
                    facebook: false,
                },
                reply,
            ))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn set_interface_language(&self, language: &str) -> Result<(), &'static str> {
        if language.len() != 3 || !language.bytes().all(|b| b.is_ascii_lowercase()) {
            return Err("invalid_language");
        }
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Language(language.to_owned(), reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_addon(&self, url: &str) -> Result<(), &'static str> {
        let selected = validate_addon_url(url)?;
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Addon(selected, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn install_addon(&self, expected_revision: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Install(expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_catalog(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Catalog(index, None, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn search_catalog(
        &self,
        index: usize,
        query: &str,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let query = query.trim();
        if query.is_empty() || query.len() > 256 || query.chars().any(char::is_control) {
            return Err("invalid_search");
        }
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Catalog(
                index,
                Some(query.to_owned()),
                expected_revision,
                reply,
            ))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn show_catalog_choices(
        &self,
        offset: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::CatalogChoices(offset, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn show_catalog_items(
        &self,
        offset: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::CatalogItems(offset, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Acknowledges admission; settled state reports asynchronous persistence errors.
    pub async fn set_details_library(
        &self,
        member: bool,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::DetailsLibrary(member, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Acknowledges admission; callers must await settled persistence before success.
    pub async fn set_details_watched(
        &self,
        video_id: Option<String>,
        watched: bool,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::DetailsWatched(
                video_id,
                watched,
                expected_revision,
                reply,
            ))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn show_library(
        &self,
        offset: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::LibraryWindow(offset, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_library_details(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Details(index, expected_revision, true, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_details(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Details(index, expected_revision, false, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn show_details_videos(
        &self,
        offset: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::DetailsVideos(offset, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn select_details_season(
        &self,
        season: Option<u32>,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::DetailsSeason(season, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn load_sources(
        &self,
        index: Option<usize>,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Sources(index, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn show_sources(
        &self,
        offset: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::SourceWindow(offset, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Returns owned handoff data only to the requesting caller, never a snapshot.
    pub async fn prepare_torrent(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<crate::playback_source::PreparedTorrent, &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::PrepareTorrent(index, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Consumes one prepared selection for the native Player owner. This does not
    /// start transfer or acknowledge playback/checkpoint success.
    pub async fn claim_playback(
        &self,
        token: u64,
    ) -> Result<stremio_core::models::player::Selected, &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::ClaimPlayback(token, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn start_playback(&self, token: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::StartPlayback(token, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn playback_event(
        &self,
        token: u64,
        event: crate::player::PlaybackEvent,
    ) -> Result<crate::player::Progress, &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::PlaybackEvent(token, event, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Acknowledges one local durable checkpoint. Concurrent requests return busy.
    /// Periodic coalescing and lifecycle barriers are separate integration work.
    pub async fn checkpoint_playback(&self, token: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Checkpoint(token, false, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Freezes progress and retires the owner only after its final local save.
    /// A failed write leaves the owner available for explicit retry.
    pub async fn close_playback(&self, token: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Checkpoint(token, true, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Retires accounting only. Durable close barriers are not yet exposed to Swift.
    pub async fn retire_playback(&self, token: u64) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::RetirePlayback(token, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    pub async fn prepare_https(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<crate::playback_source::PreparedHttps, &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::PrepareHttps(index, expected_revision, reply))
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    async fn subtitle_plan(
        &self,
        meta_id: &str,
        video_id: &str,
        kind: &str,
        generation: Option<u64>,
    ) -> Result<SubtitlePlan, &'static str> {
        if [meta_id, video_id, kind].iter().any(|value| {
            value.is_empty() || value.len() > 2048 || value.chars().any(char::is_control)
        }) {
            return Err("invalid_subtitle_identity");
        }
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::SubtitlePlan {
                meta_id: meta_id.to_owned(),
                video_id: video_id.to_owned(),
                kind: kind.to_owned(),
                generation,
                reply,
            })
            .map_err(|_| "session_unavailable")?;
        ack.await.map_err(|_| "session_stopped")?
    }
    /// Fetch outside the Core owner loop; verify the selection generation again
    /// before releasing private results to the caller.
    pub async fn discover_subtitles(
        &self,
        meta_id: &str,
        video_id: &str,
        kind: &str,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> Result<crate::subtitles::Discovery, &'static str> {
        let (generation, requests) = self.subtitle_plan(meta_id, video_id, kind, None).await?;
        let result =
            crate::subtitles::discover(self.environment.clone(), requests, cancellation.clone())
                .await?;
        self.subtitle_plan(meta_id, video_id, kind, Some(generation))
            .await?;
        if cancellation.is_cancelled() {
            return Err("subtitle_discovery_cancelled");
        }
        Ok(result)
    }
    pub async fn settled(&mut self) -> Result<Snapshot, &'static str> {
        loop {
            let state = self.snapshot();
            if state.stopped {
                return Err("session_stopped");
            }
            if state.pending_effects == 0 {
                return Ok(state);
            }
            self.state.changed().await.map_err(|_| "session_stopped")?;
        }
    }
    pub async fn stop(&mut self) {
        self.environment.cancel();
        if let Some(owner) = self.owner.take() {
            let _ = owner.await;
        }
        self.environment.shutdown().await;
    }
}
impl Drop for CoreSession {
    fn drop(&mut self) {
        self.environment.cancel();
    }
}
fn snapshot(
    ctx: &Ctx,
    addon: &AddonDetails,
    catalog: &crate::catalog::Catalog,
    details: &MetaDetails,
    video_offset: usize,
    selected_season: Option<u32>,
    source_offset: usize,
    catalog_choices_offset: usize,
    library_offset: usize,
    catalog_item_offset: usize,
    revision: u64,
    pending_effects: usize,
    stopped: bool,
    error: Option<&'static str>,
) -> Snapshot {
    let (catalogs, catalog_total) = crate::catalog::choices(ctx, catalog_choices_offset);
    Snapshot {
        sync: None,
        library: crate::library::view(&ctx.library, library_offset),
        details_in_library: crate::details::selected_preview(details).map(|meta| {
            ctx.library
                .items
                .get(&meta.id)
                .is_some_and(|item| item.r#type == meta.r#type && !item.removed && !item.temp)
        }),
        authenticated: ctx.profile.auth.is_some(),
        catalogs,
        catalog_choices_offset,
        catalog_total,
        catalog: crate::catalog::view(catalog, catalog_item_offset),
        details: crate::details::view_filtered(details, video_offset, selected_season),
        sources: crate::sources::view_with_addons(details, source_offset, &ctx.profile.addons),
        revision,
        stopped,
        interface_language: ctx.profile.settings.interface_language.clone(),
        addon_count: ctx.profile.addons.len(),
        addon: addon
            .remote_addon
            .as_ref()
            .map(|addon| match &addon.content {
                Loadable::Loading => AddonSnapshot {
                    status: "loading",
                    installed: false,
                    id: None,
                    name: None,
                },
                Loadable::Err(_) => AddonSnapshot {
                    status: "failed",
                    installed: false,
                    id: None,
                    name: None,
                },
                Loadable::Ready(value) => AddonSnapshot {
                    status: "ready",
                    installed: ctx.profile.addons.contains(value),
                    id: Some(value.manifest.id.chars().take(256).collect()),
                    name: Some(value.manifest.name.chars().take(256).collect()),
                },
            }),
        pending_effects,
        error,
    }
}
fn account_identity(ctx: &Ctx) -> Option<String> {
    let auth = ctx.profile.auth.as_ref()?;
    let id: &str = auth.user.id.as_ref();
    if id.is_empty() || id.len() > 512 || id.chars().any(char::is_control) {
        return None;
    }
    Some(id.to_owned())
}

struct PendingPlayback {
    token: u64,
    source_generation: u64,
    selection: stremio_core::models::player::Selected,
}
fn retain_playback(
    pending: &mut Option<PendingPlayback>,
    details: &MetaDetails,
    index: usize,
    source_generation: u64,
) -> Result<u64, &'static str> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_PLAYBACK: AtomicU64 = AtomicU64::new(1);
    let selection = crate::sources::player_selection(details, index)?;
    let token = NEXT_PLAYBACK
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| "playback_limit")?;
    *pending = Some(PendingPlayback {
        token,
        source_generation,
        selection,
    });
    Ok(token)
}

fn sync_view(
    ctx: &Ctx,
    journal: Option<&crate::sync_journal::Journal>,
    scheduler: &crate::sync_scheduler::Scheduler,
) -> Option<SyncSnapshot> {
    ctx.profile.auth.as_ref()?;
    let pending_titles = journal.map_or(0, |journal| journal.pending_count());
    let state = if scheduler.is_paused() {
        "reauthentication_required"
    } else if scheduler.has_in_flight() {
        "syncing"
    } else if pending_titles > 0 {
        "waiting"
    } else {
        "idle"
    };
    Some(SyncSnapshot {
        pending_titles,
        state,
    })
}
fn publish_state(
    publish: &watch::Sender<Snapshot>,
    ctx: &Ctx,
    journal: Option<&crate::sync_journal::Journal>,
    scheduler: &crate::sync_scheduler::Scheduler,
    mut state: Snapshot,
) {
    state.sync = if state.stopped {
        None
    } else {
        sync_view(ctx, journal, scheduler)
    };
    publish.send_replace(state);
}

async fn run(
    mut ctx: Ctx,
    mut journal: Option<crate::sync_journal::Journal>,
    mut inbox: mpsc::Receiver<Command>,
    outgoing: mpsc::Sender<Command>,
    publish: watch::Sender<Snapshot>,
    env: Arc<Environment>,
    mut allow_login: bool,
    identity: watch::Sender<Option<String>>,
) {
    let mut addon = AddonDetails::default();
    let mut catalog = crate::catalog::Catalog::default();
    let mut catalog_generation = 0u64;
    let mut details = MetaDetails::default();
    let mut details_generation = 0u64;
    let mut source_generation = 0u64;
    let mut pending_playback: Option<PendingPlayback> = None;
    let mut playback: Option<crate::player::Playback> = None;
    let mut checkpoint_pending = false;
    let mut playback_closing = false;
    let mut video_offset = 0usize;
    let mut selected_season = None;
    let mut source_offset = 0usize;
    let mut catalog_choices_offset = 0usize;
    let mut library_offset = 0usize;
    let mut catalog_item_offset = 0usize;
    let mut addon_generation = 0u64;
    let mut revision = 0;
    let mut pending = 0usize;
    let mut error = None;
    let mut sync_allowed = true;
    let sync_started = std::time::Instant::now();
    let mut sync_scheduler = crate::sync_scheduler::Scheduler::default();
    let mut sync_cancel = tokio_util::sync::CancellationToken::new();
    let mut sync_ack: Option<(u64, crate::sync_journal::Upload)> = None;
    let mut sync_tick = tokio::time::interval(std::time::Duration::from_secs(1));
    sync_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        // Only enter the ACK barrier after all Core effects have delivered their
        // completions. No actor command mutates the captured library while this
        // short sequential write runs; the receipt uses a separate oneshot.
        if pending == 0 {
            if let Some((ticket, upload)) = sync_ack.take() {
                if sync_scheduler.is_current(ticket) {
                    if let Some(current) = journal.clone() {
                        let library = ctx.library.clone();
                        let (send, receive) = oneshot::channel();
                        NativeEnv::exec_sequential(async move {
                            let result =
                                crate::checkpoint::acknowledge(&library, &current, &upload).await;
                            let _ = send.send(result);
                        });
                        let result = tokio::select! { biased;
                            _ = env.cancelled() => break,
                            result = receive => result,
                        };
                        let outcome = match result {
                            Ok(Ok(next)) => {
                                journal = Some(next);
                                error = None;
                                crate::sync_scheduler::Outcome::Success
                            }
                            _ => {
                                error = Some("account_sync_save_failed");
                                crate::sync_scheduler::Outcome::Retry
                            }
                        };
                        sync_scheduler.finish(ticket, sync_started.elapsed(), outcome);
                        publish.send_modify(|state| state.error = error);
                    }
                }
            }
        }
        if sync_allowed
            && journal
                .as_ref()
                .is_some_and(|journal| journal.has_pending())
        {
            if let Some(auth) = ctx.profile.auth.clone() {
                if let Some(ticket) = sync_scheduler.begin(sync_started.elapsed()) {
                    sync_cancel.cancel();
                    sync_cancel = tokio_util::sync::CancellationToken::new();
                    let cancel = sync_cancel.clone();
                    let sender = outgoing.clone();
                    NativeEnv::exec_sequential(async move {
                        if cancel.is_cancelled() {
                            return;
                        }
                        let result = crate::checkpoint::committed_upload(&auth.user.id.0)
                            .await
                            .map_err(|_| ());
                        tokio::select! { biased;
                            _ = cancel.cancelled() => {},
                            _ = sender.send(Command::SyncPrepared(ticket, auth, result)) => {},
                        }
                    });
                }
            }
        }
        let sync = sync_view(&ctx, journal.as_ref(), &sync_scheduler);
        publish.send_if_modified(|state| {
            if state.sync != sync {
                state.sync = sync;
                true
            } else {
                false
            }
        });
        let command = tokio::select! { biased;
            _ = env.cancelled() => break,
            command = inbox.recv() => match command { Some(command) => command, None => break },
            _ = sync_tick.tick() => continue,
        };
        let mut manual_library_title = None;
        let (message, reply, origin, catalog_origin, details_origin, source_origin) = match command
        {
            #[cfg(test)]
            Command::TestLibraryItem(item, reply) => (
                Msg::Internal(Internal::UpdateLibraryItem(item)),
                Some(reply),
                addon_generation,
                catalog_generation,
                details_generation,
                source_generation,
            ),
            Command::RetrySync(reply) => {
                let result = if !sync_allowed || ctx.profile.auth.is_none() {
                    Err("account_sync_unavailable")
                } else if sync_scheduler.is_paused() {
                    Err("reauthentication_required")
                } else {
                    sync_scheduler.retry_now(sync_started.elapsed());
                    Ok(())
                };
                let _ = reply.send(result);
                continue;
            }
            Command::SyncPrepared(ticket, auth, result) => {
                if !sync_scheduler.is_current(ticket) {
                    continue;
                }
                match result {
                    Ok(Some(upload)) => {
                        let sender = outgoing.clone();
                        let cancel = sync_cancel.clone();
                        NativeEnv::exec_concurrent(async move {
                            let result = tokio::select! { biased;
                                _ = cancel.cancelled() => return,
                                result = crate::sync_upload::send(&upload, &auth) => result,
                            };
                            tokio::select! { biased;
                                _ = cancel.cancelled() => {},
                                _ = sender.send(Command::SyncUploaded(ticket, upload, result)) => {},
                            }
                        });
                    }
                    _ => {
                        sync_scheduler.finish(
                            ticket,
                            sync_started.elapsed(),
                            crate::sync_scheduler::Outcome::Retry,
                        );
                        error = Some("account_sync_read_failed");
                        publish.send_modify(|state| state.error = error);
                    }
                }
                continue;
            }
            Command::SyncUploaded(ticket, upload, result) => {
                if !sync_scheduler.is_current(ticket) {
                    continue;
                }
                match result {
                    Ok(()) => sync_ack = Some((ticket, upload)),
                    Err(failure) => {
                        let outcome = match failure {
                            crate::sync_upload::Error::Transport => {
                                crate::sync_scheduler::Outcome::Retry
                            }
                            _ => crate::sync_scheduler::Outcome::Rejected,
                        };
                        sync_scheduler.finish(ticket, sync_started.elapsed(), outcome);
                        error = Some("account_sync_failed");
                        publish.send_modify(|state| state.error = error);
                    }
                }
                continue;
            }
            Command::DetailsWatched(video_id, watched, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_details"));
                    continue;
                }
                let Some(meta) = crate::details::selected_preview(&details) else {
                    let _ = reply.send(Err("details_not_ready"));
                    continue;
                };
                if ctx
                    .library
                    .items
                    .get(&meta.id)
                    .is_some_and(|item| item.r#type != meta.r#type)
                {
                    let _ = reply.send(Err("library_identity_conflict"));
                    continue;
                }
                let Some(item) = details
                    .library_item
                    .as_ref()
                    .filter(|item| item.id == meta.id && item.r#type == meta.r#type)
                else {
                    let _ = reply.send(Err("details_not_ready"));
                    continue;
                };
                let playback_video = video_id.clone();
                let (action, unchanged) = match video_id {
                    None if meta.r#type == "movie" => (
                        ActionMetaDetails::MarkAsWatched(watched),
                        item.watched() == watched,
                    ),
                    Some(id) if meta.r#type == "series" => {
                        let video = details
                            .meta_items
                            .iter()
                            .filter_map(|page| page.content.as_ref()?.ready())
                            .find(|ready| {
                                ready.preview.id == meta.id && ready.preview.r#type == meta.r#type
                            })
                            .and_then(|ready| {
                                ready.videos.iter().find(|video| {
                                    video.id == id && !id.is_empty() && id.len() <= 512
                                })
                            });
                        let (Some(video), Some(bits)) = (video, details.watched.as_ref()) else {
                            let _ = reply.send(Err("invalid_video"));
                            continue;
                        };
                        (
                            ActionMetaDetails::MarkVideoAsWatched(video.clone(), watched),
                            bits.get_video(&id) == watched,
                        )
                    }
                    _ => {
                        let _ = reply.send(Err("invalid_video"));
                        continue;
                    }
                };
                // Reject before changing the playback override or Core details. The
                // synchronous message drain below admits no intervening commands;
                // library_update commits this same title's intent before its write.
                if crate::sync_journal::Journal::plan_intent(
                    &journal,
                    &ctx.library,
                    &meta.r#type,
                    &meta.id,
                )
                .is_err()
                {
                    let _ = reply.send(Err("library_write_failed"));
                    continue;
                }
                if let Some(owner) = &mut playback {
                    owner.override_watched(
                        &meta.r#type,
                        &meta.id,
                        playback_video.as_deref(),
                        watched,
                    );
                }
                manual_library_title = Some(meta.id.clone());
                // Repeated intent retries disk persistence without increasing play count.
                let message = if unchanged {
                    Msg::Internal(Internal::LibraryChanged(false))
                } else {
                    Msg::Action(Action::MetaDetails(action))
                };
                error = None;
                (
                    message,
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::DetailsLibrary(member, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_details"));
                    continue;
                }
                let Some(meta) = crate::details::selected_preview(&details) else {
                    let _ = reply.send(Err("details_not_ready"));
                    continue;
                };
                // Core keys library records by ID. Never mutate a conflicting type.
                if ctx
                    .library
                    .items
                    .get(&meta.id)
                    .is_some_and(|item| item.r#type != meta.r#type)
                {
                    let _ = reply.send(Err("library_identity_conflict"));
                    continue;
                }
                if !member && !ctx.library.items.contains_key(&meta.id) {
                    let _ = reply.send(Ok(()));
                    continue;
                }
                manual_library_title = Some(meta.id.clone());
                // Core suppresses unchanged updates, including after a failed disk write.
                // Reasserting membership must still persist the current library.
                let message = if ctx
                    .library
                    .items
                    .get(&meta.id)
                    .is_some_and(|item| item.removed == !member && !item.temp)
                {
                    Msg::Internal(Internal::LibraryChanged(false))
                } else {
                    let action = if member {
                        ActionCtx::AddToLibrary(meta.clone())
                    } else {
                        ActionCtx::RemoveFromLibrary(meta.id.clone())
                    };
                    Msg::Action(Action::Ctx(action))
                };
                error = None;
                (
                    message,
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Logout(reply) => {
                if checkpoint_pending || matches!(ctx.status, CtxStatus::Loading(_)) {
                    let _ = reply.send(Err("account_busy"));
                    continue;
                }
                sync_allowed = false;
                sync_cancel.cancel();
                sync_scheduler.invalidate(sync_started.elapsed());
                sync_ack = None;
                allow_login = false;
                error = None;
                addon_generation += 1;
                catalog_generation += 1;
                details_generation += 1;
                source_generation += 1;
                pending_playback = None;
                playback = None;
                addon = AddonDetails::default();
                catalog = crate::catalog::Catalog::default();
                details = MetaDetails::default();
                catalog_choices_offset = 0;
                library_offset = 0;
                catalog_item_offset = 0;
                video_offset = 0;
                selected_season = None;
                source_offset = 0;
                revision += 1;
                (
                    Msg::Action(Action::Ctx(ActionCtx::Logout)),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Login(request, reply) => {
                if !allow_login {
                    let _ = reply.send(Err("account_login_unavailable"));
                    continue;
                }
                if checkpoint_pending
                    || ctx.profile.auth.is_some()
                    || matches!(ctx.status, CtxStatus::Loading(_))
                {
                    let _ = reply.send(Err("account_busy"));
                    continue;
                }
                error = None;
                (
                    Msg::Action(Action::Ctx(ActionCtx::Authenticate(request))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Language(language, reply) => {
                error = None;
                let mut settings = ctx.profile.settings.clone();
                settings.interface_language = language;
                (
                    Msg::Action(Action::Ctx(ActionCtx::UpdateSettings(settings))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Addon(selected, reply) => {
                error = None;
                if addon.selected.as_ref() != Some(&selected)
                    || addon
                        .remote_addon
                        .as_ref()
                        .is_some_and(|value| value.content.is_err())
                {
                    let Some(next) = addon_generation.checked_add(1) else {
                        env.fail();
                        break;
                    };
                    addon_generation = next;
                }
                (
                    Msg::Action(Action::Load(ActionLoad::AddonDetails(selected))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Install(expected_revision, reply) => {
                if expected_revision != revision {
                    let _ = reply.send(Err("stale_preview"));
                    continue;
                }
                let Some(descriptor) = addon
                    .remote_addon
                    .as_ref()
                    .and_then(|value| value.content.ready())
                    .cloned()
                else {
                    let _ = reply.send(Err("addon_not_ready"));
                    continue;
                };
                error = None;
                (
                    Msg::Action(Action::Ctx(ActionCtx::InstallAddon(descriptor))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Catalog(index, query, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_catalog"));
                    continue;
                }
                let Some(selected) =
                    crate::catalog::selection_with_search(&ctx, index, query.as_deref())
                else {
                    let _ = reply.send(Err("invalid_catalog"));
                    continue;
                };
                let Some(next) = catalog_generation.checked_add(1) else {
                    env.fail();
                    break;
                };
                catalog_generation = next;
                catalog_item_offset = 0;
                error = None;
                (
                    Msg::Action(Action::Load(ActionLoad::CatalogWithFilters(Some(selected)))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::CatalogItems(offset, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_catalog"));
                    continue;
                }
                let total = crate::catalog::loaded_count(&catalog);
                if catalog.selected.is_none() || offset > total {
                    let _ = reply.send(Err("invalid_item_offset"));
                    continue;
                }
                catalog_item_offset = offset;
                if offset == total {
                    crate::catalog::prepare_retry(&mut catalog);
                }
                if offset == total && catalog.selectable.next_page.is_some() {
                    error = None;
                    (
                        Msg::Action(Action::CatalogWithFilters(
                            ActionCatalogWithFilters::LoadNextPage,
                        )),
                        Some(reply),
                        addon_generation,
                        catalog_generation,
                        details_generation,
                        source_generation,
                    )
                } else {
                    revision += 1;
                    publish_state(
                        &publish,
                        &ctx,
                        journal.as_ref(),
                        &sync_scheduler,
                        snapshot(
                            &ctx,
                            &addon,
                            &catalog,
                            &details,
                            video_offset,
                            selected_season,
                            source_offset,
                            catalog_choices_offset,
                            library_offset,
                            catalog_item_offset,
                            revision,
                            pending,
                            false,
                            error,
                        ),
                    );
                    let _ = reply.send(Ok(()));
                    continue;
                }
            }
            Command::LibraryWindow(offset, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_library"));
                    continue;
                }
                let total = crate::library::view(&ctx.library, 0).total;
                if offset != 0 && offset >= total {
                    let _ = reply.send(Err("invalid_library_offset"));
                    continue;
                }
                library_offset = offset;
                error = None;
                revision += 1;
                publish_state(
                    &publish,
                    &ctx,
                    journal.as_ref(),
                    &sync_scheduler,
                    snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        library_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ),
                );
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::CatalogChoices(offset, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_catalog"));
                    continue;
                }
                let total = ctx
                    .profile
                    .addons
                    .iter()
                    .map(|addon| addon.manifest.catalogs.len())
                    .sum::<usize>();
                if offset != 0 && offset >= total {
                    let _ = reply.send(Err("invalid_catalog_offset"));
                    continue;
                }
                catalog_choices_offset = offset;
                revision += 1;
                publish_state(
                    &publish,
                    &ctx,
                    journal.as_ref(),
                    &sync_scheduler,
                    snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        library_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ),
                );
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::Sources(index, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_details"));
                    continue;
                }
                let Some(selected) = crate::sources::selection_filtered(
                    &details,
                    video_offset,
                    index,
                    selected_season,
                ) else {
                    let _ = reply.send(Err("invalid_video"));
                    continue;
                };
                let Some(next) = source_generation.checked_add(1) else {
                    env.fail();
                    break;
                };
                source_generation = next;
                pending_playback = None;
                details.streams.clear();
                details.meta_streams.clear();
                source_offset = 0;
                error = None;
                (
                    Msg::Action(Action::Load(ActionLoad::MetaDetails(selected))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::SubtitlePlan {
                meta_id,
                video_id,
                kind,
                generation,
                reply,
            } => {
                let matches = details.selected.as_ref().is_some_and(|selected| {
                    selected.meta_path.id == meta_id
                        && selected.meta_path.r#type == kind
                        && selected
                            .stream_path
                            .as_ref()
                            .is_some_and(|path| path.id == video_id && path.r#type == kind)
                }) && generation.is_none_or(|expected| expected == source_generation);
                let result = if matches {
                    crate::subtitles::plan(&ctx.profile.addons, &kind, &video_id)
                        .map(|requests| (source_generation, requests))
                } else {
                    Err("stale_subtitle_identity")
                };
                let _ = reply.send(result);
                continue;
            }
            Command::Checkpoint(token, close, reply) => {
                let Some(owner) = playback.as_ref().filter(|owner| owner.token == token) else {
                    let _ = reply.send(Err("stale_playback"));
                    continue;
                };
                if checkpoint_pending {
                    let _ = reply.send(Err("checkpoint_busy"));
                    continue;
                }
                let item = match owner.checkpoint_item(&ctx) {
                    Ok(item) => item,
                    Err(error) => {
                        let _ = reply.send(Err(error));
                        continue;
                    }
                };
                let next_journal = match crate::sync_journal::Journal::plan_intent(
                    &journal,
                    &ctx.library,
                    &item.r#type,
                    &item.id,
                ) {
                    Ok(next) => next,
                    Err(_) => {
                        let _ = reply.send(Err("checkpoint_write_failed"));
                        continue;
                    }
                };
                let mut library = ctx.library.clone();
                let checkpoint_title = item.id.clone();
                library.merge_items(vec![item]);
                let planned_write = match next_journal.as_ref() {
                    Some(next) => crate::checkpoint::write_with_journal(&library, next),
                    None => crate::checkpoint::write(&library, &checkpoint_title),
                };
                let write = match planned_write {
                    Ok(write) => write,
                    Err(_) => {
                        let _ = reply.send(Err("checkpoint_write_failed"));
                        continue;
                    }
                };
                // Later library actions snapshot this admitted progress. Writes
                // share Core's sequential queue, preserving admission order.
                ctx.library = library;
                journal = next_journal;
                checkpoint_pending = true;
                playback_closing = close;
                pending += 1;
                revision += 1;
                error = None;
                let sender = outgoing.clone();
                NativeEnv::exec_sequential(async move {
                    let result = write.await.map_err(|_| "checkpoint_write_failed");
                    let _ = sender.send(Command::CheckpointDone(result, reply)).await;
                });
                (
                    Msg::Internal(Internal::LibraryChanged(true)),
                    None,
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::CheckpointDone(result, reply) => {
                if playback_closing && result.is_ok() {
                    playback = None;
                }
                playback_closing = false;
                checkpoint_pending = false;
                pending = pending.saturating_sub(1);
                if let Err(failure) = result {
                    error = Some(failure);
                }
                let _ = reply.send(result);
                (
                    Msg::Internal(Internal::LibraryChanged(true)),
                    None,
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::StartPlayback(token, reply) => {
                let result = if playback.is_some() {
                    Err("playback_busy")
                } else if pending_playback.as_ref().is_some_and(|pending| {
                    pending.token == token && pending.source_generation == source_generation
                }) {
                    let pending = pending_playback.take().expect("checked pending selection");
                    crate::player::Playback::new(token, pending.selection, &details, &ctx).map(
                        |owner| {
                            playback = Some(owner);
                        },
                    )
                } else {
                    Err("stale_playback")
                };
                let _ = reply.send(result);
                continue;
            }
            Command::PlaybackEvent(token, event, reply) => {
                let result = match playback.as_mut().filter(|owner| owner.token == token) {
                    Some(_) if playback_closing => Err("playback_closing"),
                    Some(owner) => owner.event(event, &ctx),
                    None => Err("stale_playback"),
                };
                let _ = reply.send(result);
                continue;
            }
            Command::RetirePlayback(token, reply) => {
                let result = if checkpoint_pending {
                    Err("checkpoint_busy")
                } else if playback.as_ref().is_some_and(|owner| owner.token == token) {
                    playback = None;
                    Ok(())
                } else {
                    Err("stale_playback")
                };
                let _ = reply.send(result);
                continue;
            }
            Command::ClaimPlayback(token, reply) => {
                let result = if pending_playback.as_ref().is_some_and(|pending| {
                    pending.token == token && pending.source_generation == source_generation
                }) {
                    Ok(pending_playback
                        .take()
                        .expect("checked pending selection")
                        .selection)
                } else {
                    Err("stale_playback")
                };
                let _ = reply.send(result);
                continue;
            }
            Command::PrepareTorrent(index, expected, reply) => {
                let result = if expected != revision {
                    Err("stale_sources")
                } else if !crate::sources::view(&details, source_offset)
                    .is_some_and(|view| view.items.iter().any(|item| item.index == index))
                {
                    Err("invalid_source")
                } else {
                    crate::sources::prepare(&details, index).and_then(|mut request| {
                        request.playback_token = retain_playback(
                            &mut pending_playback,
                            &details,
                            index,
                            source_generation,
                        )?;
                        Ok(request)
                    })
                };
                let _ = reply.send(result);
                continue;
            }
            Command::PrepareHttps(index, expected, reply) => {
                let result = if expected != revision {
                    Err("stale_sources")
                } else if !crate::sources::view(&details, source_offset)
                    .is_some_and(|view| view.items.iter().any(|item| item.index == index))
                {
                    Err("invalid_source")
                } else {
                    crate::sources::prepare_https(&details, index).and_then(|mut request| {
                        request.playback_token = retain_playback(
                            &mut pending_playback,
                            &details,
                            index,
                            source_generation,
                        )?;
                        Ok(request)
                    })
                };
                let _ = reply.send(result);
                continue;
            }
            Command::SourceWindow(offset, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_sources"));
                    continue;
                }
                if !crate::sources::view(&details, 0)
                    .is_some_and(|view| offset == 0 || offset < view.total)
                {
                    let _ = reply.send(Err("invalid_source_offset"));
                    continue;
                }
                source_offset = offset;
                revision += 1;
                publish_state(
                    &publish,
                    &ctx,
                    journal.as_ref(),
                    &sync_scheduler,
                    snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        library_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ),
                );
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::DetailsVideos(offset, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_details"));
                    continue;
                }
                let view = crate::details::view_filtered(&details, 0, selected_season);
                if !view.is_some_and(|view| {
                    view.status == "ready" && (offset == 0 || offset < view.video_total)
                }) {
                    let _ = reply.send(Err("invalid_video_offset"));
                    continue;
                }
                video_offset = offset;
                revision += 1;
                publish_state(
                    &publish,
                    &ctx,
                    journal.as_ref(),
                    &sync_scheduler,
                    snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        library_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ),
                );
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::DetailsSeason(season, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_details"));
                    continue;
                }
                let view = crate::details::view_filtered(&details, 0, season);
                if !view.is_some_and(|view| {
                    view.status == "ready" && (season.is_none() || view.video_total > 0)
                }) {
                    let _ = reply.send(Err("invalid_season"));
                    continue;
                }
                video_offset = 0;
                selected_season = season;
                revision += 1;
                publish_state(
                    &publish,
                    &ctx,
                    journal.as_ref(),
                    &sync_scheduler,
                    snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        library_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ),
                );
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::Details(index, expected, from_library, reply) => {
                if expected != revision {
                    let _ = reply.send(Err(if from_library {
                        "stale_library"
                    } else {
                        "stale_catalog"
                    }));
                    continue;
                }
                let selected = if from_library {
                    crate::library::selection(&ctx.library, library_offset, index)
                } else {
                    crate::catalog::view(&catalog, catalog_item_offset)
                        .and_then(|view| view.items.into_iter().nth(index))
                        .map(|item| DetailsSelected {
                            meta_path: stremio_core::types::addon::ResourcePath::without_extra(
                                "meta", &item.kind, &item.id,
                            ),
                            stream_path: None,
                            guess_stream: false,
                        })
                };
                let Some(selected) = selected else {
                    let _ = reply.send(Err(if from_library {
                        "invalid_library_item"
                    } else {
                        "invalid_catalog_item"
                    }));
                    continue;
                };
                let Some(next) = details_generation.checked_add(1) else {
                    env.fail();
                    break;
                };
                details_generation = next;
                let Some(next_source) = source_generation.checked_add(1) else {
                    env.fail();
                    break;
                };
                source_generation = next_source;
                pending_playback = None;
                // A fresh model also permits explicit retry of a cached failed result.
                details = MetaDetails::default();
                video_offset = 0;
                selected_season = None;
                source_offset = 0;
                error = None;
                (
                    Msg::Action(Action::Load(ActionLoad::MetaDetails(selected))),
                    Some(reply),
                    addon_generation,
                    catalog_generation,
                    details_generation,
                    source_generation,
                )
            }
            Command::Effect(
                message,
                generation,
                catalog_version,
                details_version,
                source_version,
            ) => {
                pending = pending.saturating_sub(1);
                (
                    message,
                    None,
                    generation,
                    catalog_version,
                    details_version,
                    source_version,
                )
            }
        };
        let mut messages = VecDeque::from([message]);
        let mut handled = 0;
        while let Some(message) = messages.pop_front() {
            handled += 1;
            if handled > 1024 {
                error = Some("core_effect_limit");
                env.cancel();
                break;
            }
            if let Msg::Event(event) = message {
                if matches!(event, Event::Error { .. }) {
                    error = Some("core_effect_failed");
                }
                continue;
            }
            // Core caches a failed descriptor by URL. Explicit reselection permits
            // retry without unloading an in-flight request or a successful preview.
            let retry = if matches!(
                &message,
                Msg::Action(Action::Load(ActionLoad::AddonDetails(_)))
            ) && addon
                .remote_addon
                .as_ref()
                .is_some_and(|value| value.content.is_err())
            {
                <AddonDetails as UpdateWithCtx<NativeEnv>>::update(
                    &mut addon,
                    &Msg::Action(Action::Unload),
                    &ctx,
                )
            } else {
                Effects::none().unchanged()
            };
            // Pinned Core does not reset its server URL bucket on logout. Preserve
            // identity-scoped restore by resetting it and using Core's persistence effect.
            if matches!(&message, Msg::Internal(Internal::Logout(_))) {
                ctx.streaming_server_urls =
                    stremio_core::types::server_urls::ServerUrlsBucket::new::<NativeEnv>(None);
                messages.push_back(Msg::Internal(Internal::StreamingServerUrlsBucketChanged));
            }
            let context_effects = crate::library_update::update(
                &mut ctx,
                &message,
                manual_library_title.as_deref(),
                &mut journal,
            );
            let stale_manifest = origin != addon_generation
                && matches!(&message, Msg::Internal(Internal::ManifestRequestResult(..)));
            let addon_effects = if stale_manifest {
                Effects::none().unchanged()
            } else {
                <AddonDetails as UpdateWithCtx<NativeEnv>>::update(&mut addon, &message, &ctx)
            };
            let stale_resource = catalog_origin != catalog_generation
                && matches!(&message, Msg::Internal(Internal::ResourceRequestResult(..)));
            let catalog_effects = if stale_resource {
                Effects::none().unchanged()
            } else {
                <crate::catalog::Catalog as UpdateWithCtx<NativeEnv>>::update(
                    &mut catalog,
                    &message,
                    &ctx,
                )
            };
            let stale_details = matches!(&message,
                Msg::Internal(Internal::ResourceRequestResult(request, _))
                    if (request.path.resource == "meta" && details_origin != details_generation)
                        || (request.path.resource == "stream" && source_origin != source_generation));
            let details_effects = if stale_details {
                Effects::none().unchanged()
            } else {
                <MetaDetails as UpdateWithCtx<NativeEnv>>::update(&mut details, &message, &ctx)
            };
            let effects = retry
                .join(context_effects)
                .join(addon_effects)
                .join(catalog_effects)
                .join(details_effects);
            if effects.has_changed {
                revision += 1;
            }
            for effect in effects {
                match effect {
                    Effect::Msg(message) => messages.push_back(*message),
                    Effect::Future(future) => {
                        pending += 1;
                        let sender = outgoing.clone();
                        match future {
                            EffectFuture::Concurrent(future) => {
                                NativeEnv::exec_concurrent(async move {
                                    let _ = sender
                                        .send(Command::Effect(
                                            future.await,
                                            addon_generation,
                                            catalog_generation,
                                            details_generation,
                                            source_generation,
                                        ))
                                        .await;
                                })
                            }
                            EffectFuture::Sequential(future) => {
                                NativeEnv::exec_sequential(async move {
                                    let _ = sender
                                        .send(Command::Effect(
                                            future.await,
                                            addon_generation,
                                            catalog_generation,
                                            details_generation,
                                            source_generation,
                                        ))
                                        .await;
                                })
                            }
                        }
                    }
                }
            }
        }
        identity.send_replace(account_identity(&ctx));
        publish_state(
            &publish,
            &ctx,
            journal.as_ref(),
            &sync_scheduler,
            snapshot(
                &ctx,
                &addon,
                &catalog,
                &details,
                video_offset,
                selected_season,
                source_offset,
                catalog_choices_offset,
                library_offset,
                catalog_item_offset,
                revision,
                pending,
                false,
                error,
            ),
        );
        if let Some(reply) = reply {
            let _ = reply.send(if error.is_none() {
                Ok(())
            } else {
                Err("core_effect_failed")
            });
        }
    }
    sync_cancel.cancel();
    inbox.close();
    identity.send_replace(None);
    publish_state(
        &publish,
        &ctx,
        journal.as_ref(),
        &sync_scheduler,
        snapshot(
            &ctx,
            &addon,
            &catalog,
            &details,
            video_offset,
            selected_season,
            source_offset,
            catalog_choices_offset,
            library_offset,
            catalog_item_offset,
            revision,
            0,
            true,
            env.failure().or(error),
        ),
    );
}
