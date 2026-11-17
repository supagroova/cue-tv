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
        Effect, EffectFuture, Effects, Env, EnvError, Update, UpdateWithCtx,
        msg::{Action, ActionCatalogWithFilters, ActionCtx, ActionLoad, Event, Internal, Msg},
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
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
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
    pub pending_effects: usize,
    pub error: Option<&'static str>,
}
type SubtitlePlan = (u64, Vec<stremio_core::types::addon::ResourceRequest>);
enum Command {
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
    Details(usize, u64, oneshot::Sender<Result<(), &'static str>>),
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
                crate::restore::context(profile).await
            })
            .await;
        let ctx = match setup {
            Ok(ctx) => ctx,
            Err(error) => {
                environment.shutdown().await;
                return Err(error);
            }
        };
        let (commands, inbox) = mpsc::channel(64);
        let initial = snapshot(
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
            false,
            None,
        );
        let (identity_publish, identity) = watch::channel(account_identity(&ctx));
        let (publish, state) = watch::channel(initial);
        let env = environment.clone();
        let outgoing = commands.clone();
        let owner = tokio::spawn(async move {
            let result = std::panic::AssertUnwindSafe(env.scope(run(
                ctx,
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
    pub async fn load_details(
        &self,
        index: usize,
        expected_revision: u64,
    ) -> Result<(), &'static str> {
        let (reply, ack) = oneshot::channel();
        self.commands
            .try_send(Command::Details(index, expected_revision, reply))
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
    catalog_item_offset: usize,
    revision: u64,
    pending_effects: usize,
    stopped: bool,
    error: Option<&'static str>,
) -> Snapshot {
    let (catalogs, catalog_total) = crate::catalog::choices(ctx, catalog_choices_offset);
    Snapshot {
        authenticated: ctx.profile.auth.is_some(),
        catalogs,
        catalog_choices_offset,
        catalog_total,
        catalog: crate::catalog::view(catalog, catalog_item_offset),
        details: crate::details::view_filtered(details, video_offset, selected_season),
        sources: crate::sources::view(details, source_offset),
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

async fn run(
    mut ctx: Ctx,
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
    let mut video_offset = 0usize;
    let mut selected_season = None;
    let mut source_offset = 0usize;
    let mut catalog_choices_offset = 0usize;
    let mut catalog_item_offset = 0usize;
    let mut addon_generation = 0u64;
    let mut revision = 0;
    let mut pending = 0usize;
    let mut error = None;
    loop {
        let command = tokio::select! { biased;
            _ = env.cancelled() => break,
            command = inbox.recv() => match command { Some(command) => command, None => break },
        };
        let (message, reply, origin, catalog_origin, details_origin, source_origin) = match command
        {
            Command::Logout(reply) => {
                if matches!(ctx.status, CtxStatus::Loading(_)) {
                    let _ = reply.send(Err("account_busy"));
                    continue;
                }
                allow_login = false;
                error = None;
                addon_generation += 1;
                catalog_generation += 1;
                details_generation += 1;
                source_generation += 1;
                addon = AddonDetails::default();
                catalog = crate::catalog::Catalog::default();
                details = MetaDetails::default();
                catalog_choices_offset = 0;
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
                if ctx.profile.auth.is_some() || matches!(ctx.status, CtxStatus::Loading(_)) {
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
                    publish.send_replace(snapshot(
                        &ctx,
                        &addon,
                        &catalog,
                        &details,
                        video_offset,
                        selected_season,
                        source_offset,
                        catalog_choices_offset,
                        catalog_item_offset,
                        revision,
                        pending,
                        false,
                        error,
                    ));
                    let _ = reply.send(Ok(()));
                    continue;
                }
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
                publish.send_replace(snapshot(
                    &ctx,
                    &addon,
                    &catalog,
                    &details,
                    video_offset,
                    selected_season,
                    source_offset,
                    catalog_choices_offset,
                    catalog_item_offset,
                    revision,
                    pending,
                    false,
                    error,
                ));
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
            Command::PrepareTorrent(index, expected, reply) => {
                let result = if expected != revision {
                    Err("stale_sources")
                } else if !crate::sources::view(&details, source_offset)
                    .is_some_and(|view| view.items.iter().any(|item| item.index == index))
                {
                    Err("invalid_source")
                } else {
                    crate::sources::prepare(&details, index)
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
                    crate::sources::prepare_https(&details, index)
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
                publish.send_replace(snapshot(
                    &ctx,
                    &addon,
                    &catalog,
                    &details,
                    video_offset,
                    selected_season,
                    source_offset,
                    catalog_choices_offset,
                    catalog_item_offset,
                    revision,
                    pending,
                    false,
                    error,
                ));
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
                publish.send_replace(snapshot(
                    &ctx,
                    &addon,
                    &catalog,
                    &details,
                    video_offset,
                    selected_season,
                    source_offset,
                    catalog_choices_offset,
                    catalog_item_offset,
                    revision,
                    pending,
                    false,
                    error,
                ));
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
                publish.send_replace(snapshot(
                    &ctx,
                    &addon,
                    &catalog,
                    &details,
                    video_offset,
                    selected_season,
                    source_offset,
                    catalog_choices_offset,
                    catalog_item_offset,
                    revision,
                    pending,
                    false,
                    error,
                ));
                let _ = reply.send(Ok(()));
                continue;
            }
            Command::Details(index, expected, reply) => {
                if expected != revision {
                    let _ = reply.send(Err("stale_catalog"));
                    continue;
                }
                let item = crate::catalog::view(&catalog, catalog_item_offset)
                    .and_then(|view| view.items.into_iter().nth(index));
                let Some(item) = item else {
                    let _ = reply.send(Err("invalid_catalog_item"));
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
                // A fresh model also permits explicit retry of a cached failed result.
                details = MetaDetails::default();
                video_offset = 0;
                selected_season = None;
                source_offset = 0;
                error = None;
                let selected = DetailsSelected {
                    meta_path: stremio_core::types::addon::ResourcePath::without_extra(
                        "meta", &item.kind, &item.id,
                    ),
                    stream_path: None,
                    guess_stream: false,
                };
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
            let context_effects = <Ctx as Update<NativeEnv>>::update(&mut ctx, &message);
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
        publish.send_replace(snapshot(
            &ctx,
            &addon,
            &catalog,
            &details,
            video_offset,
            selected_season,
            source_offset,
            catalog_choices_offset,
            catalog_item_offset,
            revision,
            pending,
            false,
            error,
        ));
        if let Some(reply) = reply {
            let _ = reply.send(if error.is_none() {
                Ok(())
            } else {
                Err("core_effect_failed")
            });
        }
    }
    inbox.close();
    identity.send_replace(None);
    publish.send_replace(snapshot(
        &ctx,
        &addon,
        &catalog,
        &details,
        video_offset,
        selected_season,
        source_offset,
        catalog_choices_offset,
        catalog_item_offset,
        revision,
        0,
        true,
        env.failure().or(error),
    ));
}
